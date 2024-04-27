use crate::settlement::Settlement;
pub(crate) mod interfaces;
use crate::env::EthereumEnv;
use crate::settlement::ethereum::interfaces::eigen_bridge::EigenBridgeContractClient;
use crate::settlement::ethereum::interfaces::eigen_global_exit_root::EigenGlobalExitRootContractClient;
use anyhow::Result;
use async_trait::async_trait;
use ethers::signers::{LocalWallet, Signer};
use ethers_core::k256::elliptic_curve::SecretKey;
use ethers_core::types::{Address, Bytes, U256};
use ethers_core::utils::hex;
use ethers_providers::{Http, Provider};

pub struct EthereumSettlement {
    pub eigen_bridge_client: EigenBridgeContractClient,
    pub eigen_global_exit_root_client: EigenGlobalExitRootContractClient,
}

pub struct EthereumSettlementConfig {
    pub eth_settlement_env: EthereumEnv,
}

impl EthereumSettlement {
    pub fn new(config: EthereumSettlementConfig) -> Self {
        let provider = Provider::<Http>::try_from(&config.eth_settlement_env.provider_url).unwrap();
        let kye_bytes = hex::decode(&config.eth_settlement_env.local_wallet.private_key).unwrap();
        let secret_key = SecretKey::from_slice(&kye_bytes).unwrap();
        let local_wallet: LocalWallet = LocalWallet::from(secret_key)
            .with_chain_id(config.eth_settlement_env.local_wallet.chain_id);

        let eigen_bridge_address: Address = config
            .eth_settlement_env
            .l1_contracts_addr
            .eigen_bridge
            .parse()
            .unwrap();

        let eigen_global_exit_root_address: Address = config
            .eth_settlement_env
            .l1_contracts_addr
            .eigen_global_exit
            .parse()
            .unwrap();

        EthereumSettlement {
            eigen_bridge_client: EigenBridgeContractClient::new(
                eigen_bridge_address,
                provider.clone(),
                local_wallet.clone(),
            ),
            eigen_global_exit_root_client: EigenGlobalExitRootContractClient::new(
                eigen_global_exit_root_address,
                provider,
                local_wallet.clone(),
            ),
        }
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
        self.eigen_bridge_client
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
        self.eigen_bridge_client
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
        self.eigen_bridge_client
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
        self.eigen_bridge_client
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
        self.eigen_global_exit_root_client
            .update_exit_root(new_root)
            .await
    }

    async fn get_global_exit_root(&self) -> Result<[u8; 32]> {
        self.eigen_global_exit_root_client
            .get_last_global_exit_root()
            .await
    }
}
