use reth::consensus::Consensus;
use reth::builder::PayloadBuilderConfig;
use reth_node_ethereum::node::EthereumConsensusBuilder;
use reth_node_builder::components::BasicPayloadServiceBuilder;
use reth_node_ethereum::EthereumEthApiBuilder;
use reth_node_ethereum::node::EthereumExecutorBuilder;
use reth_db::mdbx::DatabaseArguments;
use alloy_eips::eip4844::MAX_DATA_GAS_PER_BLOCK;
use alloy_eips::merge::BEACON_NONCE;
use alloy_consensus::constants::EMPTY_WITHDRAWALS;
//use reth_optimism_primitives::ADDRESS_L2_TO_L1_MESSAGE_PASSER;
use alloy_consensus::EMPTY_OMMER_ROOT_HASH;
use revm::context_interface::result::{EVMError, InvalidTransaction};
use reth::rpc::types::Withdrawal;
use reth_provider::ExecutionOutcome;
use reth::beacon_consensus::EthBeaconConsensus;
use reth_node_ethereum::engine::EthPayloadAttributes;
use reth::payload::PayloadId;
use revm::{context_interface::result::ResultAndState, DatabaseCommit, DatabaseRef};
use reth::rpc::types::Withdrawals;
use reth_node_ethereum::engine::{
    ExecutionPayloadV1,
    ExecutionPayloadEnvelopeV2,
    ExecutionPayloadEnvelopeV3,
    ExecutionPayloadEnvelopeV4,
};
use reth_payload_builder::PayloadBuilderService;
use reth_basic_payload_builder::is_better_payload;
use reth_basic_payload_builder::BasicPayloadJobGenerator;
use reth_transaction_pool::Pool;
use reth_primitives::Receipt;
use reth::chainspec::ChainSpec;
use reth_db::Receipts;
use reth_node_core::primitives::proofs;
use reth_node_core::primitives::Header;
use alloy_primitives::{Address, B256, address};
use reth::transaction_pool::ValidPoolTransaction;
use reth::revm::database::StateProviderDatabase;
use reth_basic_payload_builder::BasicPayloadJobGeneratorConfig;
use std::fmt;
use reth::{
    api::{InvalidPayloadAttributesError, PayloadTypes},
    builder::{
        components::{PayloadServiceBuilder, ComponentsBuilder, PayloadBuilderBuilder},
        node::NodeTypes,
        rpc::{EngineValidatorBuilder, RpcAddOns},
        BuilderContext, FullNodeTypes, Node, NodeAdapter, NodeBuilder, NodeComponentsBuilder,
    },
    primitives::{Block, EthPrimitives, RecoveredBlock, SealedBlock, TransactionSigned},
    providers::{EthStorage, StateProviderFactory},
    rpc::types::engine::ExecutionPayload,
    tasks::TaskManager,
    transaction_pool::{PoolTransaction, TransactionPool},
};
use reth_basic_payload_builder::{BuildArguments, BuildOutcome, PayloadConfig};
use reth_ethereum_payload_builder::{EthereumBuilderConfig};
use reth_node_api::{
    EngineTypes,
    FullNodeComponents, PayloadAttributes, PayloadBuilderAttributes,
};
use reth_node_core::{node_config::NodeConfig};
use reth_node_ethereum::{
    node::{
        EthereumNetworkBuilder,
        EthereumPoolBuilder,
    },
    EthEvmConfig, 
    EthEngineTypes,
};
use reth_payload_builder::{EthBuiltPayload, EthPayloadBuilderAttributes, PayloadBuilderError, PayloadBuilderHandle};
use reth_tracing::{RethTracer, Tracer};
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, sync::Arc};


/// The L2 contract `L2ToL1MessagePasser`, stores commitments to withdrawal transactions.
pub const ADDRESS_L2_TO_L1_MESSAGE_PASSER: Address =
    address!("0x4200000000000000000000000000000000000016");

use reth_db::init_db;
use reth_provider::{
    providers::{BlockchainProvider, ProviderFactory},
    CanonStateSubscriptions,
};
use reth_transaction_pool::{
    BestTransactionsAttributes,
};

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
use reth_revm::db::states::bundle_state::BundleRetention;
use reth_revm::{revm, State};

use alloy_primitives::U256;

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
}

/// Custom engine types - uses a custom payload attributes RPC type, but uses the default
/// payload builder attributes type.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[non_exhaustive]
pub struct CustomEngineTypes;

impl EngineTypes for CustomEngineTypes {
    type ExecutionPayloadEnvelopeV1 = ExecutionPayloadV1;
    type ExecutionPayloadEnvelopeV2 = ExecutionPayloadEnvelopeV2;
    type ExecutionPayloadEnvelopeV3 = ExecutionPayloadEnvelopeV3;
    type ExecutionPayloadEnvelopeV4 = ExecutionPayloadEnvelopeV4;
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

use reth_trie_db::MerklePatriciaTrie;
/// Configure the node types
impl NodeTypes for MyCustomNode {
    type Primitives = EthPrimitives;
    type ChainSpec = ChainSpec;
    type StateCommitment = MerklePatriciaTrie;
    type Storage = EthStorage;
    type Payload = CustomEngineTypes;
}

/// Custom addons configuring RPC types
pub type MyNodeAddOns<N> = RpcAddOns<N, EthereumEthApiBuilder, CustomEngineValidatorBuilder>;

/// Implement the Node trait for the custom node
///
/// This provides a preset configuration for the node
impl<N> Node<N> for MyCustomNode
where
    N: FullNodeTypes<
        Types: NodeTypes<
            Payload = CustomEngineTypes,
            ChainSpec = ChainSpec,
            Primitives = EthPrimitives,
            Storage = EthStorage,
        >,
    >,
{
    type ComponentsBuilder = ComponentsBuilder<
        N,
        EthereumPoolBuilder,
        BasicPayloadServiceBuilder<CustomPayloadBuilderBuilder>,
        EthereumNetworkBuilder,
        EthereumExecutorBuilder,
        EthereumConsensusBuilder,
    >;
    type AddOns = MyNodeAddOns<
        NodeAdapter<N, <Self::ComponentsBuilder as NodeComponentsBuilder<N>>::Components>,
    >;

    fn components_builder(&self) -> Self::ComponentsBuilder {
        ComponentsBuilder::default()
            .node_types::<N>()
            .pool(EthereumPoolBuilder::default())
            .payload(BasicPayloadServiceBuilder::default())
            .network(EthereumNetworkBuilder::default())
            .executor(EthereumExecutorBuilder::default())
            .consensus(EthereumConsensusBuilder::default())
    }

    fn add_ons(&self) -> Self::AddOns {
        MyNodeAddOns::default()
    }
}

/// A custom payload service builder that supports the custom eng/ine types
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
    Node: FullNodeTypes<
        Types: NodeTypes<
            Payload = EthEngineTypes,
            ChainSpec = ChainSpec,
            Primitives = EthPrimitives,
        >,
    >,
    Pool: TransactionPool<Transaction: PoolTransaction<Consensus = TransactionSigned>>
        + Unpin
        + 'static,
{
    async fn spawn_payload_builder_service(
        self,
        ctx: &BuilderContext<Node>,
        pool: Pool,
    ) -> eyre::Result<PayloadBuilderHandle<<Node::Types as NodeTypes>::Payload>> {
        // let payload_builder = CustomPayloadBuilder::default();
        //let payload_builder = CustomPayloadBuilder::new(self.tx_filter_config);
        let payload_builder = reth_ethereum_payload_builder::EthereumPayloadBuilder::new(
            ctx.provider().clone(),
            pool,
            EthEvmConfig::new(ctx.chain_spec()),
            EthereumBuilderConfig::new(),
        );
        let conf = ctx.payload_builder_config();

        let payload_job_config = BasicPayloadJobGeneratorConfig::default()
            .interval(conf.interval())
            .deadline(conf.deadline())
            .max_payload_tasks(conf.max_payload_tasks());

        let payload_generator = BasicPayloadJobGenerator::with_builder(
            ctx.provider().clone(),
            ctx.task_executor().clone(),
            payload_job_config,
            payload_builder,
        );
        
        let (payload_service, payload_builder) =
            PayloadBuilderService::new(payload_generator, ctx.provider().canonical_state_stream());

        ctx.task_executor()
            .spawn_critical("payload builder service", Box::pin(payload_service));

        Ok(payload_builder)
    }
}

///// The type responsible for building custom payloads
//#[derive(Debug, Default, Clone)]
//#[non_exhaustive]
//pub struct CustomPayloadBuilder {
//    // custom fields
//    pub tx_filter_config: TxFilterConfig,
//}
//
//impl CustomPayloadBuilder {
//    pub fn new(tx_filter_config: TxFilterConfig) -> Self {
//        Self { tx_filter_config }
//    }
//}
//
//impl PayloadBuilder for CustomPayloadBuilder
//where
//    Client: StateProviderFactory,
//    Pool: TransactionPool,
//{
//    type Attributes = CustomPayloadBuilderAttributes;
//    type BuiltPayload = EthBuiltPayload;
//
//    // When the CL (Consensus Client) creates a new proposal, it accesses the EL (Execution Client) by calling the get_payload_v4 API to get the ExecutionPayload.
//    // The ExecutionPayload is built here by selecting high gas fee transactions from the transaction pool to construct a new block.
//    fn try_build(
//        &self,
//        args: BuildArguments<Self::Attributes, Self::BuiltPayload>,
//    ) -> Result<BuildOutcome<Self::BuiltPayload>, PayloadBuilderError> {
//        let BuildArguments {
//            client,
//            pool,
//            cached_reads,
//            config,
//            cancel,
//            best_payload,
//        } = args;
//        let PayloadConfig {
//            initialized_block_env,
//            initialized_cfg,
//            parent_block,
//            extra_data,
//            attributes,
//            chain_spec,
//        } = config;
//
//        // This reuses the default EthereumPayloadBuilder to build the payload
//        // but any custom logic can be implemented here
//        // reth_ethereum_payload_builder::EthereumPayloadBuilder::default().try_build(BuildArguments {
//        //     client,
//        //     pool,
//        //     cached_reads,
//        //     config: PayloadConfig {
//        //         initialized_block_env,
//        //         initialized_cfg,
//        //         parent_block,
//        //         extra_data,
//        //         attributes: attributes.0,
//        //         chain_spec,
//        //     },
//        //     cancel,
//        //     best_payload,
//        // })
//
//        // we can customize the payload builder here, to control the block building process
//        custom_payload_builder(
//            BuildArguments {
//                client,
//                pool,
//                cached_reads,
//                config: PayloadConfig {
//                    initialized_block_env,
//                    initialized_cfg,
//                    parent_block,
//                    extra_data,
//                    attributes: attributes.0,
//                    chain_spec,
//                },
//                cancel,
//                best_payload,
//            },
//            self.tx_filter_config.clone(),
//        )
//    }
//
//    fn build_empty_payload(
//        client: &Client,
//        config: PayloadConfig<Self::Attributes>,
//    ) -> Result<Self::BuiltPayload, PayloadBuilderError> {
//        let PayloadConfig { parent_header, attributes } = config;
//        self.inner.build_empty_payload(PayloadConfig { parent_header, attributes: attributes.0 })
//        //<reth_ethereum_payload_builder::EthereumPayloadBuilder  as PayloadBuilder<Pool,Client>>  ::build_empty_payload(
//        //    client,
//        //    PayloadConfig { initialized_block_env, initialized_cfg, parent_block, extra_data, attributes: attributes.0, chain_spec }
//        //)
//    }
//}

pub fn custom_payload_builder<Pool, Client>(
    args: BuildArguments<EthPayloadBuilderAttributes, EthBuiltPayload>,
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

        // 0x647c576c000000000000000000000000000000000000000000000000000000000000000 &[u8]
        let tx_input = tx.transaction.input();
        // When calling the built-in method of eth, the input is 0x
        // let tx_input_bytes: Vec<u8> = Vec::from_hex(tx_input).expect("err msg");
        let function_selector = &tx_input[0..4];

        // let function_selector_str: String = function_selector.encode_hex();

        // let mut function_selector_str: String = String::new();
        // _ = function_selector.read_to_string(&mut function_selector_str);
        let function_selector_str = function_selector.encode_hex_with_prefix();

        let _parameters_data = &tx_input[4..];
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
        //let mut evm = revm::Evm::builder()
        //    .with_db(&mut db)
        //    .with_env_with_handler_cfg(EnvWithHandlerCfg::new_with_cfg_env(
        //        initialized_cfg.clone(),
        //        initialized_block_env.clone(),
        //        evm_config.tx_env(&tx),
        //    ))
        //    .build();
        let mut evm_env = EthEvmConfig::default().evm_env(&header);
        let mut evm = EthEvmConfig::default().evm_with_env(&mut db, evm_env);

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

    //let WithdrawalsOutcome {
    //    withdrawals_root,
    //    withdrawals,
    //} = commit_withdrawals(
    //    &mut db,
    //    &chain_spec,
    //    attributes.timestamp,
    //    attributes.withdrawals,
    //)?;
    let BlockBuilderOutcome { execution_result, block, .. } = builder.finish(&state_provider)?;

    let requests = chain_spec
        .is_prague_active_at_timestamp(attributes.timestamp)
        .then_some(execution_result.requests);

    let withdrawals_root = if ctx.is_isthmus_active() {
        // withdrawals root field in block header is used for storage root of L2 predeploy
        // `l2tol1-message-passer`
        Some(
            state
                .database
                .as_ref()
                .storage_root(ADDRESS_L2_TO_L1_MESSAGE_PASSER, Default::default())?,
        )
    } else if ctx.is_canyon_active() {
        Some(EMPTY_WITHDRAWALS)
    } else {
        None
    };

    // merge all transitions into bundle state, this would apply the withdrawal balance changes
    // and 4788 contract call
    db.merge_transitions(BundleRetention::PlainState);

    let bundle = ExecutionOutcome::new(
        db.take_bundle(),
        vec![receipts],
        block_number,
        requests,
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
            parent_block.maybe_next_block_excess_blob_gas(
                self.chain_spec.blob_params_at_timestamp(timestamp),
            )
        } else {
            // for the first post-fork block, both parent.blob_gas_used and
            // parent.excess_blob_gas are evaluated as 0
            Some(alloy_eips::eip7840::BlobParams::cancun().next_block_excess_blob_gas(0, 0))
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

    let sealed_block = Arc::new(block.sealed_block().clone());
    debug!(target: "payload_builder", id=%attributes.id, sealed_block_header = ?sealed_block.sealed_header(), "sealed built block");

    let mut payload = EthBuiltPayload::new(attributes.id, sealed_block, total_fees, requests);

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

    let consensus: Arc<dyn Consensus> = Arc::new(EthBeaconConsensus::new(Arc::clone(&spec)));

    let evm_config = EthEvmConfig::default();

    let provider = BlockchainProvider::new(factory)?;

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
