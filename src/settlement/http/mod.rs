use super::BatchData as RustBatchData;
use crate::config::env::GLOBAL_ENV;
use crate::settlement::ethereum::interfaces::bridge::BridgeContractClient;
use crate::settlement::ethereum::interfaces::global_exit_root::GlobalExitRootContractClient;
use crate::settlement::ethereum::interfaces::zeth_global_exit_root::ZethGlobalExitRootContractClient;
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

pub async fn get_last_rollup_exit_root(
    block_number: u64,
    bridge_service_client: &Client,
) -> Result<[u8; 32]> {
    let respose = bridge_service_client
        .get(format!(
            "{}/get-root",
            GLOBAL_ENV.bridge_service_addr.clone()
        ))
        .query(&[("block_num", block_number)])
        .send()
        .await;
    let mut last_rollup_exit_root = [0u8; 32];
    match respose {
        Ok(res) => {
            if res.status().is_success() {
                let body = res.text().await?;
                let parsed_json: serde_json::Value = serde_json::from_str(&body).unwrap();
                let rollup_exit_root = parsed_json["rollup_exit_root"].as_str().unwrap_or_default();
                let rollup_exit_root_bytes =
                    hex::decode(&rollup_exit_root[2..]).expect("Failed to decode hex string");

                last_rollup_exit_root.copy_from_slice(&rollup_exit_root_bytes);
                log::debug!(
                    "block: {}, rollup_exit_root: {}",
                    block_number,
                    rollup_exit_root
                );
            } else {
                log::error!("Request failed, response: {:?}", res);
            }
        }
        Err(e) => {
            log::error!("Request error: {:?}", e);
        }
    }
    Ok(last_rollup_exit_root)
}

pub async fn get_global_exit_root(bridge_service_client: &Client) -> Result<[u8; 32]> {
    let respose = bridge_service_client
        .get(format!(
            "{}/get-global-exit-root",
            GLOBAL_ENV.bridge_service_addr.clone()
        ))
        .send()
        .await;
    let mut last_global_exit_root = [0u8; 32];
    match respose {
        Ok(res) => {
            if res.status().is_success() {
                let body = res.text().await?;
                let parsed_json: serde_json::Value = serde_json::from_str(&body).unwrap();
                let global_exit_root = parsed_json["global_exit_root"].as_str().unwrap_or_default();
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
    destination_network: u32,
    destination_address: Address,
    amount: U256,
    token: Address,
    force_update_global_exit_root: bool,
    calldata: Bytes,
    bridge_service_client: &Client,
) -> Result<()> {
    let body = json!({
        "destination_network": destination_network,
        "destination_address": destination_address.to_string(),
        "amount": amount.to_string(),
        "token": token.to_string(),
        "force_update_global_exit_root": force_update_global_exit_root,
        "calldata": hex::encode(calldata),
    });

    let response = bridge_service_client
        .post(format!(
            "{}/bridge-asset",
            GLOBAL_ENV.bridge_service_addr.clone()
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

pub async fn bridge_message(
    destination_network: u32,
    destination_address: Address,
    force_update_global_exit_root: bool,
    calldata: Bytes,
    bridge_service_client: &Client,
) -> Result<()> {
    let body = json!({
        "destination_network": destination_network,
        "destination_address": destination_address.to_string(),
        "force_update_global_exit_root": force_update_global_exit_root,
        "calldata": hex::encode(calldata),
    });

    let response = bridge_service_client
        .post(format!(
            "{}/bridge-message",
            GLOBAL_ENV.bridge_service_addr.clone()
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

pub async fn claim_asset(
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
    bridge_service_client: &Client,
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

    let response = bridge_service_client
        .post(format!(
            "{}/claim-asset",
            GLOBAL_ENV.bridge_service_addr.clone()
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

pub async fn claim_message(
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
    bridge_service_client: &Client,
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

    let response = bridge_service_client
        .post(format!(
            "{}/claim-message",
            GLOBAL_ENV.bridge_service_addr.clone()
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

pub async fn update_exit_root(
    network: u32,
    new_root: [u8; 32],
    bridge_service_client: &Client,
) -> Result<()> {
    let body = json!({
        "network": network,
        "new_root": new_root
    });

    let response = bridge_service_client
        .post(format!(
            "{}/update-exit-root",
            GLOBAL_ENV.bridge_service_addr.clone()
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

pub async fn sequence_batches(
    batches: Vec<BatchData>,
    bridge_service_client: &Client,
) -> Result<()> {
    let body = json!({
        "batches": batches
    });

    let response = bridge_service_client
        .post(format!(
            "{}/sequence-batches",
            GLOBAL_ENV.bridge_service_addr.clone()
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

pub async fn verify_batches(
    pending_state_num: u64,
    init_num_batch: u64,
    final_new_batch: u64,
    new_local_exit_root: [u8; 32],
    new_state_root: [u8; 32],
    proof: String,
    input: String,
    bridge_service_client: &Client,
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

    let response = bridge_service_client
        .post(format!(
            "{}/verify-batches",
            GLOBAL_ENV.bridge_service_addr.clone()
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

pub async fn verify_batches_trusted_aggregator(
    pending_state_num: u64,
    init_num_batch: u64,
    final_new_batch: u64,
    new_local_exit_root: [u8; 32],
    new_state_root: [u8; 32],
    proof: String,
    input: String,
    bridge_service_client: &Client,
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

    let response = bridge_service_client
        .post(format!(
            "{}/verify-batches-trusted-aggregator",
            GLOBAL_ENV.bridge_service_addr.clone()
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