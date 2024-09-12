use super::BatchData as RustBatchData;
use crate::cli::Cli;
use crate::config::env::GLOBAL_ENV;
use crate::settlement::BatchData;
use crate::settlement::Settlement;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use config::{Config, File};
use ethers::signers::{LocalWallet, Signer};
use ethers_core::k256::elliptic_curve::SecretKey;
use ethers_core::types::{Address, Bytes, U256};
use ethers_core::utils::hex;
use ethers_providers::{Http, Provider};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use serde_json::Value;
use std::path::Path;
use std::str::FromStr;

pub struct CustomClient {
    pub client: Client,
    pub url: String,
}

impl CustomClient {
    pub fn new(url: String) -> Self {
        let client = Client::new();
        CustomClient { client, url }
    }

    pub async fn get_last_rollup_exit_root(&self) -> Result<[u8; 32]> {
        // TODO
        todo!()
    }

    pub async fn get_global_exit_root(&self) -> Result<[u8; 32]> {
        let respose = self
            .client
            .get(format!("{}/get-global-exit-root", self.url.clone()))
            .send()
            .await;
        let mut last_global_exit_root = [0u8; 32];
        match respose {
            Ok(res) => {
                if res.status().is_success() {
                    let body = res.text().await?;
                    let parsed_json: serde_json::Value = serde_json::from_str(&body).unwrap();
                    let global_exit_root =
                        parsed_json["global_exit_root"].as_str().unwrap_or_default();
                    let global_exit_root_bytes =
                        hex::decode(&global_exit_root[2..]).expect("Failed to decode hex string");

                    last_global_exit_root.copy_from_slice(&global_exit_root_bytes);
                } else {
                    log::error!("Request failed, response: {:?}", res);
                }
            }
            Err(e) => {
                log::error!("Request error: {:?}", e);
            }
        }
        Ok(last_global_exit_root)
    }

    pub async fn bridge_asset(
        &self,
        destination_network: u32,
        destination_address: Address,
        amount: U256,
        token: Address,
        force_update_global_exit_root: bool,
        calldata: Bytes,
    ) -> Result<()> {
        let body = json!({
            "destination_network": destination_network,
            "destination_address": destination_address.to_string(),
            "amount": amount.to_string(),
            "token": token.to_string(),
            "force_update_global_exit_root": force_update_global_exit_root,
            "calldata": hex::encode(calldata),
        });

        let response = self
            .client
            .post(format!("{}/bridge-asset", self.url.clone()))
            .json(&body)
            .send()
            .await;

        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "Request failed with status: {}",
                        resp.status()
                    ))
                }
            }
            Err(e) => Err(anyhow::anyhow!("Failed to send request: {:?}", e)),
        }
    }

    pub async fn bridge_message(
        &self,
        destination_network: u32,
        destination_address: Address,
        force_update_global_exit_root: bool,
        calldata: Bytes,
    ) -> Result<()> {
        let body = json!({
            "destination_network": destination_network,
            "destination_address": destination_address.to_string(),
            "force_update_global_exit_root": force_update_global_exit_root,
            "calldata": hex::encode(calldata),
        });

        let response = self
            .client
            .post(format!("{}/bridge-message", self.url.clone()))
            .json(&body)
            .send()
            .await;

        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "Request failed with status: {}",
                        resp.status()
                    ))
                }
            }
            Err(e) => Err(anyhow::anyhow!("Failed to send request: {:?}", e)),
        }
    }

    pub async fn claim_asset(
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
        let body = json!({
            "smt_proof": smt_proof,
            "index": index,
            "mainnet_exit_root": mainnet_exit_root,
            "rollup_exit_root": rollup_exit_root,
            "origin_network": origin_network,
            "origin_token_address": origin_token_address,
            "destination_network": destination_network,
            "destination_address": destination_address,
            "amount": amount,
            "metadata": metadata,
        });

        let response = self
            .client
            .post(format!("{}/claim-asset", self.url.clone()))
            .json(&body)
            .send()
            .await;

        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "Request failed with status: {}",
                        resp.status()
                    ))
                }
            }
            Err(e) => Err(anyhow::anyhow!("Failed to send request: {:?}", e)),
        }
    }

    pub async fn claim_message(
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
        let body = json!({
            "smt_proof": smt_proof,
            "index": index,
            "mainnet_exit_root": mainnet_exit_root,
            "rollup_exit_root": rollup_exit_root,
            "origin_network": origin_network,
            "origin_address": origin_address,
            "destination_network": destination_network,
            "destination_address": destination_address,
            "amount": amount,
            "metadata": metadata,
        });

        let response = self
            .client
            .post(format!("{}/claim-message", self.url.clone()))
            .json(&body)
            .send()
            .await;

        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "Request failed with status: {}",
                        resp.status()
                    ))
                }
            }
            Err(e) => Err(anyhow::anyhow!("Failed to send request: {:?}", e)),
        }
    }

    pub async fn update_exit_root(&self, network: u32, new_root: [u8; 32]) -> Result<()> {
        let body = json!({
            "network": network,
            "new_root": new_root
        });

        let response = self
            .client
            .post(format!("{}/update-exit-root", self.url.clone()))
            .json(&body)
            .send()
            .await;

        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "Request failed with status: {}",
                        resp.status()
                    ))
                }
            }
            Err(e) => Err(anyhow::anyhow!("Failed to send request: {:?}", e)),
        }
    }

    pub async fn sequence_batches(&self, batches: Vec<BatchData>) -> Result<()> {
        let body = json!({
            "batches": batches
        });

        let response = self
            .client
            .post(format!("{}/sequence-batches", self.url.clone()))
            .json(&body)
            .send()
            .await;

        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "Request failed with status: {}",
                        resp.status()
                    ))
                }
            }
            Err(e) => Err(anyhow::anyhow!("Failed to send request: {:?}", e)),
        }
    }

    pub async fn verify_batches(
        &self,
        pending_state_num: u64,
        init_num_batch: u64,
        final_new_batch: u64,
        new_local_exit_root: [u8; 32],
        new_state_root: [u8; 32],
        proof: String,
        input: String,
    ) -> Result<()> {
        let body = json!({
            "pending_state_num": pending_state_num,
            "init_num_batch": init_num_batch,
            "final_new_batch": final_new_batch,
            "new_local_exit_root": hex::encode(new_local_exit_root),
            "new_state_root": hex::encode(new_state_root),
            "proof": proof,
            "input": input
        });

        let response = self
            .client
            .post(format!("{}/verify-batches", self.url.clone()))
            .json(&body)
            .send()
            .await;

        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "Request failed with status: {}",
                        resp.status()
                    ))
                }
            }
            Err(e) => Err(anyhow::anyhow!("Failed to send request: {:?}", e)),
        }
    }

    pub async fn verify_batches_trusted_aggregator(
        &self,
        pending_state_num: u64,
        init_num_batch: u64,
        final_new_batch: u64,
        new_local_exit_root: [u8; 32],
        new_state_root: [u8; 32],
        proof: String,
        input: String,
    ) -> Result<()> {
        let body = json!({
            "pending_state_num": pending_state_num,
            "init_num_batch": init_num_batch,
            "final_new_batch": final_new_batch,
            "new_local_exit_root": hex::encode(new_local_exit_root),
            "new_state_root": hex::encode(new_state_root),
            "proof": proof,
            "input": input
        });

        let response = self
            .client
            .post(format!(
                "{}/verify-batches-trusted-aggregator",
                self.url.clone()
            ))
            .json(&body)
            .send()
            .await;

        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "Request failed with status: {}",
                        resp.status()
                    ))
                }
            }
            Err(e) => Err(anyhow::anyhow!("Failed to send request: {:?}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::env::GlobalEnv;
    use crate::db::lfs::libmdbx::{open_mdbx_db, Config};
    use crate::settlement::custom::CustomSettlementConfig;
    use crate::settlement::{init_settlement_provider, NetworkSpec};
    use std::{env, fs};

    #[tokio::test]
    async fn test_bridge_service() {
        let config = CustomSettlementConfig {
            service_url: GLOBAL_ENV.bridge_service_addr.clone(),
        };
        let settlement_spec = NetworkSpec::Custom(config);
        let settlement_provider = init_settlement_provider(settlement_spec)
            .map_err(|e| anyhow!("Failed to init settlement: {:?}", e))?;

        settlement_provider.bridge_asset(
            destination_network,
            destination_address,
            amount,
            token,
            force_update_global_exit_root,
            calldata,
        );
        settlement_provider.bridge_message(
            destination_network,
            destination_address,
            force_update_global_exit_root,
            calldata,
        );
        settlement_provider.claim_asset(
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
        );
        settlement_provider.claim_message(
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
        );
        settlement_provider.get_global_exit_root();
        settlement_provider.get_last_rollup_exit_root();
        settlement_provider.sequence_batches(batches);
        settlement_provider.verify_batches(
            pending_state_num,
            init_num_batch,
            final_new_batch,
            new_local_exit_root,
            new_state_root,
            proof,
            input,
        );
        settlement_provider.verify_batches_trusted_aggregator(
            pending_state_num,
            init_num_batch,
            final_new_batch,
            new_local_exit_root,
            new_state_root,
            proof,
            input,
        );
        settlement_provider.update_exit_root(network, new_root);
    }
}
