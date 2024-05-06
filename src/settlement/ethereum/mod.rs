use crate::settlement::Settlement;
use std::path::Path;
pub(crate) mod interfaces;
use super::BatchData as RustBatchData;
use crate::settlement::ethereum::interfaces::bridge::BridgeContractClient;
use crate::settlement::ethereum::interfaces::global_exit_root::GlobalExitRootContractClient;
use crate::settlement::ethereum::interfaces::zkvm::{
    BatchData, G1Point, G2Point, Proof, ZkVMContractClient,
};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use config::{Config, File};
use ethers::signers::{LocalWallet, Signer};
use ethers_core::k256::elliptic_curve::SecretKey;
use ethers_core::types::{Address, Bytes, U256};
use ethers_core::utils::hex;
use ethers_providers::{Http, Provider};
use serde::Deserialize;
use serde_json::Value;
use std::str::FromStr;

pub struct EthereumSettlement {
    pub bridge_client: BridgeContractClient,
    pub global_exit_root_client: GlobalExitRootContractClient,
    pub zkvm_contract_client: ZkVMContractClient,
}

#[derive(Debug, Deserialize)]
pub struct EthereumSettlementConfig {
    pub provider_url: String,
    pub local_wallet: LocalWalletConfig,
    pub l1_contracts_addr: EthContractsAddr,
}

#[derive(Debug, Deserialize)]
pub struct LocalWalletConfig {
    pub private_key: String,
    pub chain_id: u64,
}

#[derive(Debug, Deserialize)]
pub struct EthContractsAddr {
    pub bridge: String,
    pub global_exit: String,
    pub zkvm: String,
}

impl EthereumSettlementConfig {
    pub fn from_conf_path(conf_path: &str) -> Result<Self> {
        log::info!("Load the Ethereum settlement config from: {}", conf_path);

        let config = Config::builder()
            .add_source(File::from(Path::new(conf_path)))
            .build()
            .map_err(|e| anyhow!("Failed to build config: {:?}", e))?;

        config
            .get("ethereum_settlement_config")
            .map_err(|e| anyhow!("Failed to parse EthereumSettlementConfig: {:?}", e))
    }
}

impl EthereumSettlement {
    pub fn new(config: EthereumSettlementConfig) -> Result<Self> {
        let provider = Provider::<Http>::try_from(&config.provider_url).map_err(|e| {
            anyhow!(
                "Failed to create provider from URL {}: {:?}",
                config.provider_url,
                e
            )
        })?;

        let kye_bytes = hex::decode(&config.local_wallet.private_key).map_err(|e| {
            anyhow!(
                "Failed to decode private key {}: {:?}",
                config.local_wallet.private_key,
                e
            )
        })?;

        let secret_key = SecretKey::from_slice(&kye_bytes)
            .map_err(|e| anyhow!("Failed to parse secret key: {:?}", e))?;

        let local_wallet: LocalWallet =
            LocalWallet::from(secret_key).with_chain_id(config.local_wallet.chain_id);

        let bridge_address: Address = config.l1_contracts_addr.bridge.parse().map_err(|e| {
            anyhow!(
                "Failed to parse bridge address {}: {:?}",
                config.l1_contracts_addr.bridge,
                e
            )
        })?;

        let global_exit_root_address: Address =
            config.l1_contracts_addr.global_exit.parse().map_err(|e| {
                anyhow!(
                    "Failed to parse global exit root address {}: {:?}",
                    config.l1_contracts_addr.global_exit,
                    e
                )
            })?;

        let zkvm_address: Address = config.l1_contracts_addr.zkvm.parse().map_err(|e| {
            anyhow!(
                "Failed to parse zkvm address {}: {:?}",
                config.l1_contracts_addr.zkvm,
                e
            )
        })?;

        Ok(EthereumSettlement {
            bridge_client: BridgeContractClient::new(
                bridge_address,
                provider.clone(),
                local_wallet.clone(),
            ),
            global_exit_root_client: GlobalExitRootContractClient::new(
                global_exit_root_address,
                provider.clone(),
                local_wallet.clone(),
            ),
            zkvm_contract_client: ZkVMContractClient::new(zkvm_address, provider, local_wallet),
        })
    }
}

#[async_trait]
impl Settlement for EthereumSettlement {
    async fn bridge_asset(
        &self,
        destination_network: u32,
        destination_address: Address,
        amount: U256,
        token: Address,
        force_update_global_exit_root: bool,
        calldata: Bytes,
    ) -> Result<()> {
        self.bridge_client
            .bridge_asset(
                destination_network,
                destination_address,
                amount,
                token,
                force_update_global_exit_root,
                calldata,
            )
            .await
    }

    async fn bridge_message(
        &self,
        destination_network: u32,
        destination_address: Address,
        force_update_global_exit_root: bool,
        calldata: Bytes,
    ) -> Result<()> {
        self.bridge_client
            .bridge_message(
                destination_network,
                destination_address,
                force_update_global_exit_root,
                calldata,
            )
            .await
    }

    async fn claim_asset(
        &self,
        smt_proof: [[u8; 32]; 32],
        index: u32,
        mainnet_exit_root: [u8; 32],
        rollup_exit_root: [u8; 32],
        origin_network: u32,
        origin_token_address: Address,
        destination_network: u32,
        destination_address: Address,
        amount: U256,
        metadata: Bytes,
    ) -> Result<()> {
        self.bridge_client
            .claim_asset(
                smt_proof,
                index,
                mainnet_exit_root,
                rollup_exit_root,
                origin_network,
                origin_token_address,
                destination_network,
                destination_address,
                amount,
                metadata,
            )
            .await
    }

    async fn claim_message(
        &self,
        smt_proof: [[u8; 32]; 32],
        index: u32,
        mainnet_exit_root: [u8; 32],
        rollup_exit_root: [u8; 32],
        origin_network: u32,
        origin_address: Address,
        destination_network: u32,
        destination_address: Address,
        amount: U256,
        metadata: Bytes,
    ) -> Result<()> {
        self.bridge_client
            .claim_message(
                smt_proof,
                index,
                mainnet_exit_root,
                rollup_exit_root,
                origin_network,
                origin_address,
                destination_network,
                destination_address,
                amount,
                metadata,
            )
            .await
    }

    async fn update_global_exit_root(&self, new_root: [u8; 32]) -> Result<()> {
        self.global_exit_root_client
            .update_exit_root(new_root)
            .await
    }

    async fn get_global_exit_root(&self) -> Result<[u8; 32]> {
        self.global_exit_root_client
            .get_last_global_exit_root()
            .await
    }

    /// solidity BatchData struct:
    /// ```solidity
    ///      struct BatchData {
    ///         bytes transactions;
    ///         bytes32 globalExitRoot;
    ///         uint64 timestamp;
    ///         uint64 minForcedTimestamp;
    ///     }
    /// ```
    ///
    /// the binding of a Rust structure to a Solidity structure:
    /// ```rust
    /// pub struct BatchData(bytes,bytes32,uint64,uint64)
    /// ```
    ///
    /// eigen-zeth BatchData struct:
    /// ```rust
    /// pub(crate) struct BatchData {
    //     pub transactions: Vec<u8>,
    //     pub global_exit_root: [u8; 32],
    //     pub timestamp: u64,
    //     pub min_forced_timestamp: u64,
    // }
    /// ```
    ///
    async fn sequence_batches(
        &self,
        batches: Vec<RustBatchData>,
        l2_coinbase: Address,
    ) -> Result<()> {
        let solidity_batches = batches
            .iter()
            .map(|b| BatchData {
                transactions: Bytes::from(b.transactions.clone()),
                global_exit_root: b.global_exit_root,
                timestamp: b.timestamp,
                min_forced_timestamp: b.min_forced_timestamp,
            })
            .collect();

        self.zkvm_contract_client
            .sequence_batches(solidity_batches, l2_coinbase)
            .await
    }

    async fn verify_batches(
        &self,
        pending_state_num: u64,
        init_num_batch: u64,
        final_new_batch: u64,
        new_local_exit_root: [u8; 32],
        new_state_root: [u8; 32],
        proof: String,
        input: String,
    ) -> Result<()> {
        let p = parse_proof(&proof).map_err(|e| {
            anyhow!(
                "Failed to parse proof from json string: {:?}, err: {:?}",
                proof,
                e
            )
        })?;

        let i = parse_public_input(&input).map_err(|e| {
            anyhow!(
                "Failed to parse public input from json string: {:?}, err: {:?}",
                input,
                e
            )
        })?;

        self.zkvm_contract_client
            .verify_batches(
                pending_state_num,
                init_num_batch,
                final_new_batch,
                new_local_exit_root,
                new_state_root,
                p,
                i,
            )
            .await
    }

    async fn verify_batches_trusted_aggregator(
        &self,
        pending_state_num: u64,
        init_num_batch: u64,
        final_new_batch: u64,
        new_local_exit_root: [u8; 32],
        new_state_root: [u8; 32],
        _proof: String,
        _input: String,
    ) -> Result<()> {
        // TODO: parse the final proof to solidity Proof struct
        let p = Proof {
            a: G1Point {
                x: U256::from_str("1").unwrap(),
                y: U256::from_str("1").unwrap(),
            },
            b: G2Point {
                x: [U256::from_str("1").unwrap(), U256::from_str("1").unwrap()],
                y: [U256::from_str("1").unwrap(), U256::from_str("1").unwrap()],
            },
            c: G1Point {
                x: U256::from_str("1").unwrap(),
                y: U256::from_str("1").unwrap(),
            },
        };

        // TODO: parse the public input to solidity input: [U256; 1]
        let i = [U256::from(0u64)];

        self.zkvm_contract_client
            .verify_batches_trusted_aggregator(
                pending_state_num,
                init_num_batch,
                final_new_batch,
                new_local_exit_root,
                new_state_root,
                p,
                i,
            )
            .await
    }
}

pub fn parse_proof(json_str: &str) -> Result<Proof> {
    let err_msg = "invalid json data";
    let v: Value = serde_json::from_str(json_str)?;
    let pi_a_x: U256 = U256::from_dec_str(v["pi_a"]["x"].as_str().ok_or(anyhow!(err_msg))?)?;
    let pi_a_y: U256 = U256::from_dec_str(v["pi_a"]["y"].as_str().ok_or(anyhow!(err_msg))?)?;

    let pi_b_x_0: U256 = U256::from_dec_str(v["pi_b"]["x"][0].as_str().ok_or(anyhow!(err_msg))?)?;
    let pi_b_x_1: U256 = U256::from_dec_str(v["pi_b"]["x"][1].as_str().ok_or(anyhow!(err_msg))?)?;
    let pi_b_y_0: U256 = U256::from_dec_str(v["pi_b"]["y"][0].as_str().ok_or(anyhow!(err_msg))?)?;
    let pi_b_y_1: U256 = U256::from_dec_str(v["pi_b"]["y"][1].as_str().ok_or(anyhow!(err_msg))?)?;

    let pi_c_x: U256 = U256::from_dec_str(v["pi_c"]["x"].as_str().ok_or(anyhow!(err_msg))?)?;
    let pi_c_y: U256 = U256::from_dec_str(v["pi_c"]["y"].as_str().ok_or(anyhow!(err_msg))?)?;

    Ok(Proof {
        a: G1Point {
            x: pi_a_x,
            y: pi_a_y,
        },
        b: G2Point {
            x: [pi_b_x_0, pi_b_x_1],
            y: [pi_b_y_0, pi_b_y_1],
        },
        c: G1Point {
            x: pi_c_x,
            y: pi_c_y,
        },
    })
}

pub fn parse_public_input(json_str: &str) -> Result<[U256; 1]> {
    let err_msg = "invalid json data";
    let v: Value = serde_json::from_str(json_str)?;
    let pi: U256 = U256::from_dec_str(v["pi"].as_str().ok_or(anyhow!(err_msg))?)?;

    Ok([pi])
}
