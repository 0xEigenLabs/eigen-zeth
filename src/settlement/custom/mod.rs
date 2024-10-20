use super::BatchData as RustBatchData;
use crate::settlement::Settlement;
use anyhow::Result;
use async_trait::async_trait;
use ethers_core::types::{Address, Bytes, U256};
use reqwest::Client;
use serde::Deserialize;
pub mod methods;
use crate::settlement::custom::methods::*;

#[derive(Debug, Clone, Deserialize)]
pub struct CustomSettlementConfig {
    pub service_url: String,
}

pub struct CustomSettlement {
    pub bridge_service_client: CustomClient,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LocalWalletConfig {
    pub private_key: String,
    pub chain_id: u64,
}

impl CustomSettlement {
    pub fn new(config: CustomSettlementConfig) -> Result<Self> {
        let client = Client::new();
        Ok(CustomSettlement {
            bridge_service_client: CustomClient {
                client,
                url: config.service_url,
            },
        })
    }
}

#[async_trait]
impl Settlement for CustomSettlement {
    async fn bridge_asset(
        &self,
        destination_network: u32,
        destination_address: Address,
        amount: U256,
        token: Address,
        force_update_global_exit_root: bool,
        calldata: Bytes,
    ) -> Result<()> {
        self.bridge_service_client
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
        self.bridge_service_client
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
        self.bridge_service_client
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
        self.bridge_service_client
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

    async fn update_exit_root(&self, network: u32, new_root: [u8; 32]) -> Result<()> {
        self.bridge_service_client
            .update_exit_root(network, new_root)
            .await
    }

    async fn get_global_exit_root(&self) -> Result<[u8; 32]> {
        self.bridge_service_client.get_global_exit_root().await
    }

    async fn sequence_batches(&self, batches: Vec<RustBatchData>) -> Result<()> {
        self.bridge_service_client.sequence_batches(batches).await
    }

    async fn verify_batches(
        &self,
        pending_state_num: u64,
        cur_block_num: u64,
        init_num_batch: u64,
        final_new_batch: u64,
        new_local_exit_root: [u8; 32],
        new_state_root: [u8; 32],
        proof: String,
        input: String,
    ) -> Result<()> {
        self.bridge_service_client
            .verify_batches(
                pending_state_num,
                cur_block_num,
                init_num_batch,
                final_new_batch,
                new_local_exit_root,
                new_state_root,
                proof,
                input,
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
        self.bridge_service_client
            .verify_batches_trusted_aggregator(
                pending_state_num,
                init_num_batch,
                final_new_batch,
                new_local_exit_root,
                new_state_root,
                _proof,
                _input,
            )
            .await
    }

    async fn get_last_rollup_exit_root(&self) -> Result<[u8; 32]> {
        self.bridge_service_client.get_last_rollup_exit_root().await
    }
}
