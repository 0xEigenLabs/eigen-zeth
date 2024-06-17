use crate::settlement::Settlement;
use std::path::Path;
pub(crate) mod interfaces;
use super::BatchData as RustBatchData;
use crate::settlement::ethereum::interfaces::bridge::BridgeContractClient;
use crate::settlement::ethereum::interfaces::global_exit_root::GlobalExitRootContractClient;
use crate::settlement::ethereum::interfaces::zeth_global_exit_root::ZethGlobalExitRootContractClient;
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
    pub zeth_global_exit_root_client: ZethGlobalExitRootContractClient,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EthereumSettlementConfig {
    pub provider_url: String,
    pub local_wallet: LocalWalletConfig,
    pub l1_contracts_addr: EthContractsAddr,
    pub zeth_config: ZethConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LocalWalletConfig {
    pub private_key: String,
    pub chain_id: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EthContractsAddr {
    pub bridge: String,
    pub global_exit: String,
    pub zkvm: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ZethConfig {
    pub provider_url: String,
    pub local_wallet: LocalWalletConfig,
    pub zeth_contracts_addr: ZethContractsAddr,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ZethContractsAddr {
    pub global_exit: String,
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
        let l1_provider = Provider::<Http>::try_from(&config.provider_url).map_err(|e| {
            anyhow!(
                "Failed to create provider from URL {}: {:?}",
                config.provider_url,
                e
            )
        })?;

        let zeth_provider =
            Provider::<Http>::try_from(&config.zeth_config.provider_url).map_err(|e| {
                anyhow!(
                    "Failed to create provider from URL {}: {:?}",
                    config.zeth_config.provider_url,
                    e
                )
            })?;

        let l1_kye_bytes = hex::decode(&config.local_wallet.private_key).map_err(|e| {
            anyhow!(
                "Failed to decode private key {}: {:?}",
                config.local_wallet.private_key,
                e
            )
        })?;
        let zeth_key_bytes =
            hex::decode(&config.zeth_config.local_wallet.private_key).map_err(|e| {
                anyhow!(
                    "Failed to decode zeth private key {}: {:?}",
                    config.zeth_config.local_wallet.private_key,
                    e
                )
            })?;

        let l1_secret_key = SecretKey::from_slice(&l1_kye_bytes)
            .map_err(|e| anyhow!("Failed to parse secret key: {:?}", e))?;

        let zeth_secret_key = SecretKey::from_slice(&zeth_key_bytes)
            .map_err(|e| anyhow!("Failed to parse zeht secret key: {:?}", e))?;

        let l1_local_wallet: LocalWallet =
            LocalWallet::from(l1_secret_key).with_chain_id(config.local_wallet.chain_id);

        let zeth_local_wallet: LocalWallet = LocalWallet::from(zeth_secret_key)
            .with_chain_id(config.zeth_config.local_wallet.chain_id);

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

        let zeth_global_exit_root_address: Address = config
            .zeth_config
            .zeth_contracts_addr
            .global_exit
            .parse()
            .map_err(|e| {
                anyhow!(
                    "Failed to parse zeth global exit root address {}: {:?}",
                    config.zeth_config.zeth_contracts_addr.global_exit,
                    e
                )
            })?;

        Ok(EthereumSettlement {
            bridge_client: BridgeContractClient::new(
                bridge_address,
                l1_provider.clone(),
                l1_local_wallet.clone(),
            ),
            global_exit_root_client: GlobalExitRootContractClient::new(
                global_exit_root_address,
                l1_provider.clone(),
                l1_local_wallet.clone(),
            ),
            zkvm_contract_client: ZkVMContractClient::new(
                zkvm_address,
                l1_provider.clone(),
                l1_local_wallet.clone(),
            ),
            zeth_global_exit_root_client: ZethGlobalExitRootContractClient::new(
                zeth_global_exit_root_address,
                zeth_provider,
                zeth_local_wallet,
            ),
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
    async fn sequence_batches(&self, batches: Vec<RustBatchData>) -> Result<()> {
        let solidity_batches = batches
            .iter()
            .map(|b| BatchData {
                transactions: Bytes::from(b.transactions.clone()),
                global_exit_root: b.global_exit_root,
                timestamp: b.timestamp,
            })
            .collect();

        self.zkvm_contract_client
            .sequence_batches(solidity_batches)
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
        let hex_state_root = hex::encode(new_state_root);

        log::info!(
            "verify batches param:\n\
            Pending state num: {}\n\
            Init batch num: {}\n\
            Final batch num: {}\n\
            New local exit root: {:?}\n\
            New state root: 0x{}\n\
            Proof: {:#?}\n\
            Input: {:#?}\n",
            pending_state_num,
            init_num_batch,
            final_new_batch,
            new_local_exit_root,
            hex_state_root,
            p,
            i,
        );

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

    async fn get_zeth_last_rollup_exit_root(&self) -> Result<[u8; 32]> {
        self.zeth_global_exit_root_client
            .last_rollup_exit_root()
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
    let pi: U256 = U256::from_dec_str(v[0].as_str().ok_or(anyhow!(err_msg))?)?;

    Ok([pi])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_proof() {
        let json_str = r#"
        {
            "pi_a": {
                "x": "3894342490756754565965792842715527136900748861589701158764596228794024251264",
                "y": "20894107483553289231882001225962797952407845477555540523796359231586935733846"
            },
            "pi_b": {
                "x": [
                    "11303930567367311460199515741381653021952386064455592553366819979016369690221",
                    "6242672563875954003611167743279154704567070869698427311198660395704910129045"
                ],
                "y": [
                    "17434448597653719769617331943336407199972406352134267729364687286477203023747",
                    "15948512474254961097324594943144741066670195944422197561675513233113633547165"
                ]
            },
            "pi_c": {
                "x": "5576428152840883288360621640303844771684916806430142078283655957062969038562",
                "y": "3607080246230286103684151564530274499289235914739390649652041366374324111626"
            },
            "protocol": "groth16",
            "curve": "BN128"
        }
        "#;

        let proof = parse_proof(json_str).unwrap();
        assert_eq!(
            proof.a.x,
            U256::from_dec_str(
                "3894342490756754565965792842715527136900748861589701158764596228794024251264"
            )
            .unwrap()
        );
        assert_eq!(
            proof.a.y,
            U256::from_dec_str(
                "20894107483553289231882001225962797952407845477555540523796359231586935733846"
            )
            .unwrap()
        );
        assert_eq!(
            proof.b.x[0],
            U256::from_dec_str(
                "11303930567367311460199515741381653021952386064455592553366819979016369690221"
            )
            .unwrap()
        );
        assert_eq!(
            proof.b.x[1],
            U256::from_dec_str(
                "6242672563875954003611167743279154704567070869698427311198660395704910129045"
            )
            .unwrap()
        );
        assert_eq!(
            proof.b.y[0],
            U256::from_dec_str(
                "17434448597653719769617331943336407199972406352134267729364687286477203023747"
            )
            .unwrap()
        );
        assert_eq!(
            proof.b.y[1],
            U256::from_dec_str(
                "15948512474254961097324594943144741066670195944422197561675513233113633547165"
            )
            .unwrap()
        );
        assert_eq!(
            proof.c.x,
            U256::from_dec_str(
                "5576428152840883288360621640303844771684916806430142078283655957062969038562"
            )
            .unwrap()
        );
        assert_eq!(
            proof.c.y,
            U256::from_dec_str(
                "3607080246230286103684151564530274499289235914739390649652041366374324111626"
            )
            .unwrap()
        );
        println!("{:#?}", proof);
    }

    #[test]
    fn test_parse_public_input() {
        let json_str = r#"[
  "18643838845950978012261839449398966728966000503704503228511271056327925534401"
]"#;

        let input = parse_public_input(json_str).unwrap();
        assert_eq!(
            input[0],
            U256::from_dec_str(
                "18643838845950978012261839449398966728966000503704503228511271056327925534401"
            )
            .unwrap()
        );
        println!("{:#?}", input)
    }
    
    #[test]
    fn test_from_conf_path() {
        let conf_path = "configs/settlement.toml";
        let config = EthereumSettlementConfig::from_conf_path(conf_path).unwrap();
        println!("{:#?}", config);
    }
}
