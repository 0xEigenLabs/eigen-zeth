use reth_beacon_consensus::BeaconConsensus;
use reth_node_builder::{
    components::{ComponentsBuilder, PayloadServiceBuilder},
    node::NodeTypes,
    BuilderContext, FullNodeTypes, Node, NodeBuilder, PayloadBuilderConfig,
};
use reth_primitives::revm_primitives::{BlockEnv, CfgEnvWithHandlerCfg};
use reth_primitives::{
    proofs, Address, Block, ChainSpec, Header, IntoRecoveredTransaction, Receipt, Receipts,
    Withdrawals, B256, EMPTY_OMMER_ROOT_HASH,
};
use std::sync::Arc;

use reth_basic_payload_builder::{
    commit_withdrawals, is_better_payload, pre_block_beacon_root_contract_call,
    BasicPayloadJobGenerator, BasicPayloadJobGeneratorConfig, BuildArguments, BuildOutcome,
    PayloadBuilder, PayloadConfig, WithdrawalsOutcome,
};
use reth_db::init_db;
use reth_node_api::{
    validate_version_specific_fields, AttributesValidationError, EngineApiMessageVersion,
    EngineTypes, PayloadAttributes, PayloadBuilderAttributes, PayloadOrAttributes,
};
use reth_provider::{
    providers::{BlockchainProvider, ProviderFactory},
    BundleStateWithReceipts, CanonStateSubscriptions, StateProviderFactory,
};
use reth_tasks::TaskManager;
use reth_transaction_pool::{
    BestTransactionsAttributes, PoolTransaction, TransactionPool, ValidPoolTransaction,
};

use reth_node_ethereum::{
    node::{EthereumNetworkBuilder, EthereumPoolBuilder},
    EthEvmConfig,
};
use reth_payload_builder::{
    error::PayloadBuilderError, EthBuiltPayload, EthPayloadBuilderAttributes, PayloadBuilderHandle,
    PayloadBuilderService,
};
use reth_rpc_types::{
    engine::{
        ExecutionPayloadEnvelopeV2, ExecutionPayloadEnvelopeV3,
        PayloadAttributes as EthPayloadAttributes, PayloadId,
    },
    withdrawal::Withdrawal,
    ExecutionPayloadV1,
};
use reth_tracing::{RethTracer, Tracer};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use thiserror::Error;

use crate::commands::reth::RethCmd;
use crate::custom_reth::eigen::EigenRpcExt;
use crate::custom_reth::eigen::EigenRpcExtApiServer;
use crate::db::Database as RollupDatabase;
use anyhow::{anyhow, Result};
use config::{Config, File};
use jsonrpsee::tracing;
use jsonrpsee::tracing::{debug, trace};
use reth_blockchain_tree::{
    BlockchainTree, BlockchainTreeConfig, ShareableBlockchainTree, TreeExternals,
};
use reth_db::mdbx::DatabaseArguments;
use reth_interfaces::consensus::Consensus;
use reth_node_core::node_config::NodeConfig;
use reth_node_core::primitives::U256;
use reth_primitives::constants::eip4844::MAX_DATA_GAS_PER_BLOCK;
use reth_primitives::constants::BEACON_NONCE;
use reth_primitives::eip4844::calculate_excess_blob_gas;
use reth_primitives::hex::{FromHex, ToHex};
use reth_primitives::revm::env::tx_env_with_recovered;
use reth_revm::database::StateProviderDatabase;
use reth_revm::db::states::bundle_state::BundleRetention;
use reth_revm::{revm, EvmProcessorFactory, State};
use revm_primitives::db::DatabaseCommit;
use revm_primitives::{EVMError, EnvWithHandlerCfg, InvalidTransaction, ResultAndState};

pub(crate) mod eigen;

/// A custom payload attributes type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomPayloadAttributes {
    /// An inner payload type
    #[serde(flatten)]
    pub inner: EthPayloadAttributes,
    // /// A custom field
    // pub custom: u64,
}

/// Custom error type used in payload attributes validation
#[derive(Debug, Error)]
pub enum CustomError {
    #[error("Custom field is not zero")]
    #[allow(dead_code)]
    CustomFieldIsNotZero,
}

impl PayloadAttributes for CustomPayloadAttributes {
    fn timestamp(&self) -> u64 {
        self.inner.timestamp()
    }

    fn withdrawals(&self) -> Option<&Vec<Withdrawal>> {
        self.inner.withdrawals()
    }

    fn parent_beacon_block_root(&self) -> Option<B256> {
        self.inner.parent_beacon_block_root()
    }

    fn ensure_well_formed_attributes(
        &self,
        chain_spec: &ChainSpec,
        version: EngineApiMessageVersion,
    ) -> Result<(), AttributesValidationError> {
        validate_version_specific_fields(chain_spec, version, self.into())?;

        // // custom validation logic - ensure that the custom field is not zero
        // if self.custom == 0 {
        //     return Err(AttributesValidationError::invalid_params(
        //         CustomError::CustomFieldIsNotZero,
        //     ));
        // }

        Ok(())
    }
}

/// New type around the payload builder attributes type
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustomPayloadBuilderAttributes(EthPayloadBuilderAttributes);

impl PayloadBuilderAttributes for CustomPayloadBuilderAttributes {
    type RpcPayloadAttributes = CustomPayloadAttributes;
    type Error = Infallible;

    fn try_new(parent: B256, attributes: CustomPayloadAttributes) -> Result<Self, Infallible> {
        Ok(Self(EthPayloadBuilderAttributes::new(
            parent,
            attributes.inner,
        )))
    }

    fn payload_id(&self) -> PayloadId {
        self.0.id
    }

    fn parent(&self) -> B256 {
        self.0.parent
    }

    fn timestamp(&self) -> u64 {
        self.0.timestamp
    }

    fn parent_beacon_block_root(&self) -> Option<B256> {
        self.0.parent_beacon_block_root
    }

    fn suggested_fee_recipient(&self) -> Address {
        self.0.suggested_fee_recipient
    }

    fn prev_randao(&self) -> B256 {
        self.0.prev_randao
    }

    fn withdrawals(&self) -> &Withdrawals {
        &self.0.withdrawals
    }

    fn cfg_and_block_env(
        &self,
        chain_spec: &ChainSpec,
        parent: &Header,
    ) -> (CfgEnvWithHandlerCfg, BlockEnv) {
        self.0.cfg_and_block_env(chain_spec, parent)
    }
}

/// Custom engine types - uses a custom payload attributes RPC type, but uses the default
/// payload builder attributes type.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[non_exhaustive]
pub struct CustomEngineTypes;

impl EngineTypes for CustomEngineTypes {
    type PayloadAttributes = CustomPayloadAttributes;
    type PayloadBuilderAttributes = CustomPayloadBuilderAttributes;
    type BuiltPayload = EthBuiltPayload;
    type ExecutionPayloadV1 = ExecutionPayloadV1;
    type ExecutionPayloadV2 = ExecutionPayloadEnvelopeV2;
    type ExecutionPayloadV3 = ExecutionPayloadEnvelopeV3;

    fn validate_version_specific_fields(
        chain_spec: &ChainSpec,
        version: EngineApiMessageVersion,
        payload_or_attrs: PayloadOrAttributes<'_, CustomPayloadAttributes>,
    ) -> Result<(), AttributesValidationError> {
        validate_version_specific_fields(chain_spec, version, payload_or_attrs)
    }
}

#[derive(Debug, Clone, Default)]
#[non_exhaustive]
struct MyCustomNode {
    // custom fields
    pub tx_filter_config: TxFilterConfig,
}

impl MyCustomNode {
    pub fn new(tx_filter_config: TxFilterConfig) -> Self {
        Self { tx_filter_config }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct TxFilterConfig {
    pub bridge_contract_address: String,
    pub bridge_asset_selector: String,
}

impl TxFilterConfig {
    #[allow(dead_code)]
    pub fn new(bridge_contract_address: String, bridge_asset_selector: String) -> Self {
        Self {
            bridge_contract_address,
            bridge_asset_selector,
        }
    }

    pub fn from_conf_path(conf_path: &str) -> Result<Self> {
        log::info!(
            "Load the CustomNode TxFilterConfig config from: {}",
            conf_path
        );

        let config = Config::builder()
            .add_source(File::from(Path::new(conf_path)))
            .build()
            .map_err(|e| anyhow!("Failed to build config: {:?}", e))?;

        config
            .get("tx_filter_config")
            .map_err(|e| anyhow!("Failed to parse TxFilterConfig: {:?}", e))
    }
}

/// Configure the node types
impl NodeTypes for MyCustomNode {
    type Primitives = ();
    // use the custom engine types
    type Engine = CustomEngineTypes;
    // use the default ethereum EVM config
    type Evm = EthEvmConfig;

    fn evm_config(&self) -> Self::Evm {
        Self::Evm::default()
    }
}

/// Implement the Node trait for the custom node
///
/// This provides a preset configuration for the node
impl<N> Node<N> for MyCustomNode
where
    N: FullNodeTypes<Engine = CustomEngineTypes>,
{
    type PoolBuilder = EthereumPoolBuilder;
    type NetworkBuilder = EthereumNetworkBuilder;
    type PayloadBuilder = CustomPayloadServiceBuilder;

    fn components(
        self,
    ) -> ComponentsBuilder<N, Self::PoolBuilder, Self::PayloadBuilder, Self::NetworkBuilder> {
        ComponentsBuilder::default()
            .node_types::<N>()
            .pool(EthereumPoolBuilder::default())
            // .payload(CustomPayloadServiceBuilder::default())
            .payload(CustomPayloadServiceBuilder::new(self.tx_filter_config))
            .network(EthereumNetworkBuilder::default())
    }
}

/// A custom payload service builder that supports the custom engine types
#[derive(Debug, Default, Clone)]
#[non_exhaustive]
pub struct CustomPayloadServiceBuilder {
    // custom fields
    pub tx_filter_config: TxFilterConfig,
}

impl CustomPayloadServiceBuilder {
    pub fn new(tx_filter_config: TxFilterConfig) -> Self {
        Self { tx_filter_config }
    }
}

impl<Node, Pool> PayloadServiceBuilder<Node, Pool> for CustomPayloadServiceBuilder
where
    Node: FullNodeTypes<Engine = CustomEngineTypes>,
    Pool: TransactionPool + Unpin + 'static,
{
    async fn spawn_payload_service(
        self,
        ctx: &BuilderContext<Node>,
        pool: Pool,
    ) -> eyre::Result<PayloadBuilderHandle<Node::Engine>> {
        // let payload_builder = CustomPayloadBuilder::default();
        let payload_builder = CustomPayloadBuilder::new(self.tx_filter_config);
        let conf = ctx.payload_builder_config();

        let payload_job_config = BasicPayloadJobGeneratorConfig::default()
            .interval(conf.interval())
            .deadline(conf.deadline())
            .max_payload_tasks(conf.max_payload_tasks())
            .extradata(conf.extradata_rlp_bytes())
            .max_gas_limit(conf.max_gas_limit());

        let payload_generator = BasicPayloadJobGenerator::with_builder(
            ctx.provider().clone(),
            pool,
            ctx.task_executor().clone(),
            payload_job_config,
            ctx.chain_spec(),
            payload_builder,
        );
        let (payload_service, payload_builder) =
            PayloadBuilderService::new(payload_generator, ctx.provider().canonical_state_stream());

        ctx.task_executor()
            .spawn_critical("payload builder service", Box::pin(payload_service));

        Ok(payload_builder)
    }
}

/// The type responsible for building custom payloads
#[derive(Debug, Default, Clone)]
#[non_exhaustive]
pub struct CustomPayloadBuilder {
    // custom fields
    pub tx_filter_config: TxFilterConfig,
}

impl CustomPayloadBuilder {
    pub fn new(tx_filter_config: TxFilterConfig) -> Self {
        Self { tx_filter_config }
    }
}

impl<Pool, Client> PayloadBuilder<Pool, Client> for CustomPayloadBuilder
where
    Client: StateProviderFactory,
    Pool: TransactionPool,
{
    type Attributes = CustomPayloadBuilderAttributes;
    type BuiltPayload = EthBuiltPayload;

    // When the CL (Consensus Client) creates a new proposal, it accesses the EL (Execution Client) by calling the get_payload_v4 API to get the ExecutionPayload.
    // The ExecutionPayload is built here by selecting high gas fee transactions from the transaction pool to construct a new block.
    fn try_build(
        &self,
        args: BuildArguments<Pool, Client, Self::Attributes, Self::BuiltPayload>,
    ) -> Result<BuildOutcome<Self::BuiltPayload>, PayloadBuilderError> {
        let BuildArguments {
            client,
            pool,
            cached_reads,
            config,
            cancel,
            best_payload,
        } = args;
        let PayloadConfig {
            initialized_block_env,
            initialized_cfg,
            parent_block,
            extra_data,
            attributes,
            chain_spec,
        } = config;

        // This reuses the default EthereumPayloadBuilder to build the payload
        // but any custom logic can be implemented here
        // reth_ethereum_payload_builder::EthereumPayloadBuilder::default().try_build(BuildArguments {
        //     client,
        //     pool,
        //     cached_reads,
        //     config: PayloadConfig {
        //         initialized_block_env,
        //         initialized_cfg,
        //         parent_block,
        //         extra_data,
        //         attributes: attributes.0,
        //         chain_spec,
        //     },
        //     cancel,
        //     best_payload,
        // })

        // we can customize the payload builder here, to control the block building process
        custom_payload_builder(
            BuildArguments {
                client,
                pool,
                cached_reads,
                config: PayloadConfig {
                    initialized_block_env,
                    initialized_cfg,
                    parent_block,
                    extra_data,
                    attributes: attributes.0,
                    chain_spec,
                },
                cancel,
                best_payload,
            },
            self.tx_filter_config.clone(),
        )
    }

    fn build_empty_payload(
        client: &Client,
        config: PayloadConfig<Self::Attributes>,
    ) -> Result<Self::BuiltPayload, PayloadBuilderError> {
        let PayloadConfig {
            initialized_block_env,
            initialized_cfg,
            parent_block,
            extra_data,
            attributes,
            chain_spec,
        } = config;
        <reth_ethereum_payload_builder::EthereumPayloadBuilder  as PayloadBuilder<Pool,Client>>  ::build_empty_payload(
            client,
            PayloadConfig { initialized_block_env, initialized_cfg, parent_block, extra_data, attributes: attributes.0, chain_spec }
        )
    }
}

pub fn custom_payload_builder<Pool, Client>(
    args: BuildArguments<Pool, Client, EthPayloadBuilderAttributes, EthBuiltPayload>,
    tx_filter_config: TxFilterConfig,
) -> std::result::Result<BuildOutcome<EthBuiltPayload>, PayloadBuilderError>
where
    Client: StateProviderFactory,
    Pool: TransactionPool,
{
    tracing::info!(target: "custom_payload_builder", "TxFilterConfig {:?}", tx_filter_config);

    let BuildArguments {
        client,
        pool,
        mut cached_reads,
        config,
        cancel,
        best_payload,
    } = args;

    let state_provider = client.state_by_block_hash(config.parent_block.hash())?;
    let state = StateProviderDatabase::new(&state_provider);
    let mut db = State::builder()
        .with_database_ref(cached_reads.as_db(&state))
        .with_bundle_update()
        .build();
    let extra_data = config.extra_data();
    let PayloadConfig {
        initialized_block_env,
        initialized_cfg,
        parent_block,
        attributes,
        chain_spec,
        ..
    } = config;
    tracing::info!(target: "custom_payload_builder", id=%attributes.id, parent_hash = ?parent_block.hash(), parent_number = parent_block.number, "building new payload");
    debug!(target: "payload_builder", id=%attributes.id, parent_hash = ?parent_block.hash(), parent_number = parent_block.number, "building new payload");
    let mut cumulative_gas_used = 0;
    let mut sum_blob_gas_used = 0;
    let block_gas_limit: u64 = initialized_block_env
        .gas_limit
        .try_into()
        .unwrap_or(u64::MAX);
    let base_fee = initialized_block_env.basefee.to::<u64>();

    let mut executed_txs = Vec::new();

    let mut best_txs = pool.best_transactions_with_attributes(BestTransactionsAttributes::new(
        base_fee,
        initialized_block_env
            .get_blob_gasprice()
            .map(|gasprice| gasprice as u64),
    ));

    let flag = Arc::new(AtomicBool::new(false));

    let is_first_or_non_bridge_asset_call = |tx: Arc<
        ValidPoolTransaction<<Pool as TransactionPool>::Transaction>,
    >|
     -> bool {
        tracing::info!(target: "consensus::auto-seal::miner::pool-tx-filter","tx info: {:?}", tx);
        // load contract addr and function selector
        let contract_address = tx_filter_config.bridge_contract_address.clone();
        let bridge_asset_selector = tx_filter_config.bridge_asset_selector.clone();

        // check if the transaction is a bridge asset transaction
        let mut is_bridge_asset = false;

        let to = match tx.to() {
            Some(to) => to,
            None => return true,
        };
        tracing::info!(target: "consensus::auto-seal::miner::pool-tx-filter","tx to: {:?}", to);
        if to.to_string() != contract_address {
            tracing::info!(target: "consensus::auto-seal::miner::pool-tx-filter","tx to address({:?}) is not bridge contract address", to.to_string());
            return true;
        }

        let tx_input = tx.transaction.input();
        // When calling the built-in method of eth, the input is 0x
        let tx_input_bytes: Vec<u8> = Vec::from_hex(tx_input).expect("err msg");
        let function_selector = &tx_input_bytes[0..4];
        let function_selector_str: String = function_selector.encode_hex();
        let _parameters_data = &tx_input_bytes[4..];
        tracing::info!(target: "consensus::auto-seal::miner::pool-tx-filter","tx function selector: {:?}", function_selector_str);

        // check if the transaction is a bridge asset transaction
        if to.to_string() == contract_address && function_selector_str == bridge_asset_selector {
            is_bridge_asset = true;
        }

        if !is_bridge_asset {
            return true;
        }

        flag.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
    };

    let mut total_fees = U256::ZERO;

    let block_number = initialized_block_env.number.to::<u64>();

    // apply eip-4788 pre block contract call
    pre_block_beacon_root_contract_call(
        &mut db,
        &chain_spec,
        block_number,
        &initialized_cfg,
        &initialized_block_env,
        &attributes,
    )?;

    let mut receipts = Vec::new();
    while let Some(pool_tx) = best_txs.next() {
        // if the transaction is not the first or non-bridge asset call, we can mark_invalid it
        if !is_first_or_non_bridge_asset_call(pool_tx.clone()) {
            best_txs.mark_invalid(&pool_tx);
            continue;
        }

        // ensure we still have capacity for this transaction
        if cumulative_gas_used + pool_tx.gas_limit() > block_gas_limit {
            // we can't fit this transaction into the block, so we need to mark it as invalid
            // which also removes all dependent transaction from the iterator before we can
            // continue
            best_txs.mark_invalid(&pool_tx);
            continue;
        }

        // check if the job was cancelled, if so we can exit early
        if cancel.is_cancelled() {
            return Ok(BuildOutcome::Cancelled);
        }

        // convert tx to a signed transaction
        let tx = pool_tx.to_recovered_transaction();

        // There's only limited amount of blob space available per block, so we need to check if
        // the EIP-4844 can still fit in the block
        if let Some(blob_tx) = tx.transaction.as_eip4844() {
            let tx_blob_gas = blob_tx.blob_gas();
            if sum_blob_gas_used + tx_blob_gas > MAX_DATA_GAS_PER_BLOCK {
                // we can't fit this _blob_ transaction into the block, so we mark it as
                // invalid, which removes its dependent transactions from
                // the iterator. This is similar to the gas limit condition
                // for regular transactions above.
                trace!(target: "payload_builder", tx=?tx.hash, ?sum_blob_gas_used, ?tx_blob_gas, "skipping blob transaction because it would exceed the max data gas per block");
                best_txs.mark_invalid(&pool_tx);
                continue;
            }
        }

        // Configure the environment for the block.
        let mut evm = revm::Evm::builder()
            .with_db(&mut db)
            .with_env_with_handler_cfg(EnvWithHandlerCfg::new_with_cfg_env(
                initialized_cfg.clone(),
                initialized_block_env.clone(),
                tx_env_with_recovered(&tx),
            ))
            .build();

        let ResultAndState { result, state } = match evm.transact() {
            Ok(res) => res,
            Err(err) => {
                match err {
                    EVMError::Transaction(err) => {
                        if matches!(err, InvalidTransaction::NonceTooLow { .. }) {
                            // if the nonce is too low, we can skip this transaction
                            trace!(target: "payload_builder", %err, ?tx, "skipping nonce too low transaction");
                        } else {
                            // if the transaction is invalid, we can skip it and all of its
                            // descendants
                            trace!(target: "payload_builder", %err, ?tx, "skipping invalid transaction and its descendants");
                            best_txs.mark_invalid(&pool_tx);
                        }

                        continue;
                    }
                    err => {
                        // this is an error that we should treat as fatal for this attempt
                        return Err(PayloadBuilderError::EvmExecutionError(err));
                    }
                }
            }
        };
        // drop evm so db is released.
        drop(evm);
        // commit changes
        db.commit(state);

        // add to the total blob gas used if the transaction successfully executed
        if let Some(blob_tx) = tx.transaction.as_eip4844() {
            let tx_blob_gas = blob_tx.blob_gas();
            sum_blob_gas_used += tx_blob_gas;

            // if we've reached the max data gas per block, we can skip blob txs entirely
            if sum_blob_gas_used == MAX_DATA_GAS_PER_BLOCK {
                best_txs.skip_blobs();
            }
        }

        let gas_used = result.gas_used();

        // add gas used by the transaction to cumulative gas used, before creating the receipt
        cumulative_gas_used += gas_used;

        // Push transaction changeset and calculate header bloom filter for receipt.
        #[allow(clippy::needless_update)] // side-effect of optimism fields
        receipts.push(Some(Receipt {
            tx_type: tx.tx_type(),
            success: result.is_success(),
            cumulative_gas_used,
            logs: result.into_logs().into_iter().map(Into::into).collect(),
            ..Default::default()
        }));

        // update add to total fees
        let miner_fee = tx
            .effective_tip_per_gas(Some(base_fee))
            .expect("fee is always valid; execution succeeded");
        total_fees += U256::from(miner_fee) * U256::from(gas_used);

        // append transaction to the list of executed transactions
        executed_txs.push(tx.into_signed());
    }

    // check if we have a better block
    if !is_better_payload(best_payload.as_ref(), total_fees) {
        // can skip building the block
        return Ok(BuildOutcome::Aborted {
            fees: total_fees,
            cached_reads,
        });
    }

    let WithdrawalsOutcome {
        withdrawals_root,
        withdrawals,
    } = commit_withdrawals(
        &mut db,
        &chain_spec,
        attributes.timestamp,
        attributes.withdrawals,
    )?;

    // merge all transitions into bundle state, this would apply the withdrawal balance changes
    // and 4788 contract call
    db.merge_transitions(BundleRetention::PlainState);

    let bundle = BundleStateWithReceipts::new(
        db.take_bundle(),
        Receipts::from_vec(vec![receipts]),
        block_number,
    );
    let receipts_root = bundle
        .receipts_root_slow(block_number)
        .expect("Number is in range");
    let logs_bloom = bundle
        .block_logs_bloom(block_number)
        .expect("Number is in range");

    // calculate the state root
    let state_root = state_provider.state_root(&bundle)?;

    // create the block header
    let transactions_root = proofs::calculate_transaction_root(&executed_txs);

    // initialize empty blob sidecars at first. If cancun is active then this will
    let mut blob_sidecars = Vec::new();
    let mut excess_blob_gas = None;
    let mut blob_gas_used = None;

    // only determine cancun fields when active
    if chain_spec.is_cancun_active_at_timestamp(attributes.timestamp) {
        // grab the blob sidecars from the executed txs
        blob_sidecars = pool.get_all_blobs_exact(
            executed_txs
                .iter()
                .filter(|tx| tx.is_eip4844())
                .map(|tx| tx.hash)
                .collect(),
        )?;

        excess_blob_gas = if chain_spec.is_cancun_active_at_timestamp(parent_block.timestamp) {
            let parent_excess_blob_gas = parent_block.excess_blob_gas.unwrap_or_default();
            let parent_blob_gas_used = parent_block.blob_gas_used.unwrap_or_default();
            Some(calculate_excess_blob_gas(
                parent_excess_blob_gas,
                parent_blob_gas_used,
            ))
        } else {
            // for the first post-fork block, both parent.blob_gas_used and
            // parent.excess_blob_gas are evaluated as 0
            Some(calculate_excess_blob_gas(0, 0))
        };

        blob_gas_used = Some(sum_blob_gas_used);
    }

    let header = Header {
        parent_hash: parent_block.hash(),
        ommers_hash: EMPTY_OMMER_ROOT_HASH,
        beneficiary: initialized_block_env.coinbase,
        state_root,
        transactions_root,
        receipts_root,
        withdrawals_root,
        logs_bloom,
        timestamp: attributes.timestamp,
        mix_hash: attributes.prev_randao,
        nonce: BEACON_NONCE,
        base_fee_per_gas: Some(base_fee),
        number: parent_block.number + 1,
        gas_limit: block_gas_limit,
        difficulty: U256::ZERO,
        gas_used: cumulative_gas_used,
        extra_data,
        parent_beacon_block_root: attributes.parent_beacon_block_root,
        blob_gas_used,
        excess_blob_gas,
    };

    // seal the block
    let block = Block {
        header,
        body: executed_txs,
        ommers: vec![],
        withdrawals,
    };

    let sealed_block = block.seal_slow();
    debug!(target: "payload_builder", ?sealed_block, "sealed built block");

    let mut payload = EthBuiltPayload::new(attributes.id, sealed_block, total_fees);

    // extend the payload with the blob sidecars from the executed txs
    payload.extend_sidecars(blob_sidecars);

    Ok(BuildOutcome::Better {
        payload,
        cached_reads,
    })
}

pub async fn launch_custom_node(
    mut stop_rx: tokio::sync::mpsc::Receiver<()>,
    reth_started_signal_channel: tokio::sync::mpsc::Sender<()>,
    rollup_db: Arc<Box<dyn RollupDatabase>>,
    spec: Arc<ChainSpec>,
    reth_cmd: RethCmd,
    tx_filter_config: TxFilterConfig,
) -> Result<()> {
    let _guard = RethTracer::new().init().map_err(|e| anyhow!(e))?;

    let tasks = TaskManager::current();

    // let data_dir = data_dir.unwrap_or_chain_default(Default::default());
    // let db_path = data_dir.db_path();
    //
    // let db_arguments = DatabaseArguments::default();
    //
    // tracing::info!(target: "reth::cli", path = ?db_path, "Opening database");
    // let database = Arc::new(
    //     init_db(db_path.clone(), db_arguments)
    //         .map_err(|e| anyhow!(e))?
    //         .with_metrics(),
    // );

    // // create node config
    // let node_config = NodeConfig::test()
    //     .with_rpc(rpc_args)
    //     .with_chain(spec.clone())
    //     .with_dev(dev_args)
    //     .with_pruning(pruning_args)
    //     .with_payload_builder(payload_builder_args);

    let RethCmd {
        datadir,
        config,
        chain,
        metrics,
        trusted_setup_file,
        instance,
        with_unused_ports,
        network,
        rpc,
        txpool,
        builder,
        debug,
        db,
        dev,
        pruning,
    } = reth_cmd;

    // set up node config
    let mut node_config = NodeConfig {
        config,
        chain,
        metrics,
        instance,
        trusted_setup_file,
        network,
        rpc,
        txpool,
        builder,
        debug,
        db,
        dev,
        pruning,
    };

    let data_dir = datadir.unwrap_or_chain_default(node_config.chain.chain);
    let db_path = data_dir.db_path();

    tracing::info!(target: "reth::cli", path = ?db_path, "Opening database");
    let database = Arc::new(
        init_db(
            db_path.clone(),
            DatabaseArguments::default().log_level(db.log_level),
        )
        .map_err(|e| anyhow!(e))?
        .with_metrics(),
    );

    if with_unused_ports {
        node_config = node_config.with_unused_ports();
    }

    let factory =
        ProviderFactory::new(database.clone(), spec.clone(), data_dir.static_files_path())?;

    let consensus: Arc<dyn Consensus> = Arc::new(BeaconConsensus::new(Arc::clone(&spec)));

    let evm_config = EthEvmConfig::default();

    // Configure blockchain tree
    let tree_externals = TreeExternals::new(
        factory.clone(),
        Arc::clone(&consensus),
        EvmProcessorFactory::new(spec.clone(), evm_config),
    );

    let tree = BlockchainTree::new(tree_externals, BlockchainTreeConfig::default(), None)?;
    let blockchain_tree = ShareableBlockchainTree::new(tree);

    let provider = BlockchainProvider::new(factory, blockchain_tree.clone())?;

    let handle = NodeBuilder::new(node_config)
        .with_database(database)
        .with_launch_context(tasks.executor(), data_dir)
        // .node(MyCustomNode::default())
        .node(MyCustomNode::new(tx_filter_config))
        .extend_rpc_modules(move |ctx| {
            // create EigenRpcExt Instance
            let custom_rpc = EigenRpcExt {
                provider: provider.clone(),
                rollup_db: rollup_db.clone(),
            };

            // add EigenRpcExt to RPC modules
            ctx.modules.merge_configured(custom_rpc.into_rpc())?;

            log::info!("EigenRpcExt extension enabled");

            Ok(())
        })
        .on_node_started(move |_ctx| {
            log::info!("[OnNodeStartedHook] Node started");
            // layer2 node started, send the signal to the L2Watcher
            reth_started_signal_channel.try_send(()).unwrap();
            Ok(())
        })
        .launch()
        .await
        .unwrap();

    tokio::select! {
        _ = stop_rx.recv() => {
            log::info!("Node stopped by signal");
        }
        r = handle.node_exit_future => {
            if let Err(e) = r {
                log::error!("Node stopped with error: {:?}", e);
            } else {
                log::info!("Node stopped");
            }
        }
    }

    Ok(())
}
