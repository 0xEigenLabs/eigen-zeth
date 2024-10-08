use crate::config::env::GLOBAL_ENV;
use crate::db::{keys, prefix, Database, ProofResult, Status};
use crate::prover::ProverChannel;
use crate::settlement::{BatchData, Settlement};
use alloy_rlp::{length_of_length, BytesMut, Encodable, Header};
use anyhow::{anyhow, Result};
use config::{Config, File};
use ethers::prelude::U64;
use ethers_core::types::{BlockId, BlockNumber, Transaction};
use ethers_providers::{Http, Middleware, Provider};
use prost::bytes;
use reqwest::Client;
use reth_primitives::{Bytes, TransactionKind, TxLegacy};
use serde::Deserialize;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

pub(crate) struct Settler {}

/// A general configuration that needs to be included in the configuration structure of each implementation.
#[derive(Debug, Clone, Deserialize)]
pub struct WorkerConfig {
    pub proof_worker_interval: u64,
    pub verify_worker_interval: u64,
    pub rollup_worker_interval: u64,
}

impl WorkerConfig {
    pub fn from_conf_path(conf_path: &str) -> Result<Self> {
        log::info!("Load the settlement worker config from: {}", conf_path);

        let config = Config::builder()
            .add_source(File::from(Path::new(conf_path)))
            .build()
            .map_err(|e| anyhow!("Failed to build config: {:?}", e))?;

        config
            .get("settlement_worker_config")
            .map_err(|e| anyhow!("Failed to parse WorkerConfig: {:?}", e))
    }
}

// TODO: Use channels, streams, and other methods to flow data between workers,
// and use event driven replacement of rotation databases to drive Zeth to run,
// avoiding frequent database access

pub fn gen_proof_without_prover(
    db: Arc<Box<dyn Database>>,
    next_batch: u64,
    last_submitted_block: u64,
) {
    let mut batch = next_batch;
    while batch <= last_submitted_block {
        let proof_path = format!("{}/proof/proof.json", env!("CARGO_MANIFEST_DIR"));
        let public_input_path = format!("{}/proof/public_input.json", env!("CARGO_MANIFEST_DIR"));
        let proof: String = std::fs::read_to_string(proof_path).unwrap_or_default();
        let public_input: String = std::fs::read_to_string(public_input_path).unwrap_or_default();

        let mut default_bytes: [u8; 32] = Default::default();
        default_bytes[24..].copy_from_slice(&(batch).to_le_bytes());
        let execute_result = ProofResult {
            block_number: batch,
            proof,
            public_input,
            post_state_root: default_bytes,
            ..Default::default()
        };
        log::info!("execute batch {} success: {:?}", batch, execute_result);
        let key_with_prefix = format!(
            "{}{}",
            std::str::from_utf8(prefix::PREFIX_BATCH_PROOF).unwrap(),
            execute_result.block_number
        );
        // save the proof to the database
        let encoded_execute_result = serde_json::to_vec(&execute_result).unwrap();
        db.put(key_with_prefix.as_bytes().to_vec(), encoded_execute_result);
        // save the last proven block number, trigger the next verify task
        db.put(
            keys::KEY_LAST_PROVEN_BLOCK_NUMBER.to_vec(),
            batch.to_be_bytes().to_vec(),
        );
        batch += 1;
        db.put(keys::KEY_NEXT_BATCH.to_vec(), batch.to_be_bytes().to_vec());
        // update the block status to Batching
        let status_key = format!(
            "{}{}",
            std::str::from_utf8(prefix::PREFIX_BLOCK_STATUS).unwrap(),
            batch
        );
        let status = Status::Batching;
        let encoded_status = serde_json::to_vec(&status).unwrap();
        db.put(status_key.as_bytes().to_vec(), encoded_status);
    }
}

impl Settler {
    pub(crate) async fn proof_worker(
        db: Arc<Box<dyn Database>>,
        mut prover: ProverChannel,
        mut stop_rx: mpsc::Receiver<()>,
        worker_interval: Duration,
    ) -> Result<()> {
        let mut ticker = tokio::time::interval(worker_interval);
        let debug_proof = GLOBAL_ENV.debug_proof;
        if !debug_proof {
            prover.start().await.unwrap();
        }

        log::info!("Prove Worker started");
        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    let last_submitted_block = match db.get(keys::KEY_LAST_SUBMITTED_BLOCK_NUMBER) {
                        None => {
                            // db.put(keys::KEY_LAST_SEQUENCE_FINALITY_BLOCK_NUMBER.to_vec(), 0_u64.to_be_bytes().to_vec());
                            0
                        }
                        Some(block_number_bytes) => {
                            u64::from_be_bytes(block_number_bytes.try_into().unwrap())
                        }
                    };

                    let next_batch = match db.get(keys::KEY_NEXT_BATCH) {
                        None => {
                            db.put(keys::KEY_NEXT_BATCH.to_vec(), 1_u64.to_be_bytes().to_vec());
                            // update the block status to Batching
                            let status_key = format!("{}{}", std::str::from_utf8(prefix::PREFIX_BLOCK_STATUS).unwrap(), 1);
                            let status = Status::Batching;
                            let encoded_status = serde_json::to_vec(&status).unwrap();
                            db.put(status_key.as_bytes().to_vec(), encoded_status);
                            1
                        }
                        Some(block_number_bytes) => {
                            u64::from_be_bytes(block_number_bytes.try_into().unwrap())
                        }
                    };

                    if next_batch > last_submitted_block {
                        log::info!("no new block to prove, try again later");
                        continue;
                    }

                    if debug_proof{
                        gen_proof_without_prover(db.clone(), next_batch, last_submitted_block);
                        continue;
                    }

                    let next_batch = db.get(keys::KEY_NEXT_BATCH);
                    let current_batch = prover.get_current_batch();
                    log::debug!("fetch block {:?}, {:?}", next_batch, current_batch);

                    match (next_batch, current_batch) {
                        (None, None) => {
                            // insert the first block
                            // packing the first block
                            db.put(keys::KEY_NEXT_BATCH.to_vec(), 1_u64.to_be_bytes().to_vec());
                            // update the block status to Batching
                            let status_key = format!(
                                "{}{}",
                                std::str::from_utf8(prefix::PREFIX_BLOCK_STATUS).unwrap(),
                                1
                            );
                            let status = Status::Batching;
                            let encoded_status = serde_json::to_vec(&status).unwrap();
                            db.put(status_key.as_bytes().to_vec(), encoded_status);
                        }
                        (Some(no), None) => {
                            let block_no = u64::from_be_bytes(no.try_into().unwrap());
                            let prover_task = prover.execute(block_no);
                            tokio::select! {
                                result = prover_task => {
                                    match result {
                                        Ok(execute_result) => {
                                            log::info!("execute batch {} success: {:?}", block_no, execute_result);
                                            // let block_number_str = execute_result.block_number.to_string();

                                            let key_with_prefix = format!("{}{}", std::str::from_utf8(prefix::PREFIX_BATCH_PROOF).unwrap(), execute_result.block_number);
                                            // save the proof to the database
                                            let encoded_execute_result = serde_json::to_vec(&execute_result).unwrap();
                                            db.put(key_with_prefix.as_bytes().to_vec(), encoded_execute_result);
                                            // save the last proven block number, trigger the next verify task
                                            db.put(keys::KEY_LAST_PROVEN_BLOCK_NUMBER.to_vec(), block_no.to_be_bytes().to_vec());
                                        }
                                        Err(e) => {
                                            panic!("execute batch {} failed: {:?}", block_no, e);
                                        }
                                    }

                                    // update the next batch number, trigger the next prove task
                                    let block_no_next = block_no + 1;
                                    // packing the next block
                                    db.put(keys::KEY_NEXT_BATCH.to_vec(), block_no_next.to_be_bytes().to_vec());
                                    // update the block status to Batching
                                    let status_key = format!("{}{}", std::str::from_utf8(prefix::PREFIX_BLOCK_STATUS).unwrap(), block_no_next);
                                    let status = Status::Batching;
                                    let encoded_status = serde_json::to_vec(&status).unwrap();
                                    db.put(status_key.as_bytes().to_vec(), encoded_status);
                                }

                                _ = stop_rx.recv() => {
                                    log::info!("Prove Worker stopped");
                                    return Ok(());
                                }
                            }
                        }
                        (None, Some(no)) => todo!("Invalid branch, block: {no}"),
                        (Some(next), Some(cur)) => {
                            let block_no = u64::from_be_bytes(next.try_into().unwrap());
                            log::debug!("next: {block_no}, current: {cur}");
                        }
                    };
                }
                _ = stop_rx.recv() => {
                    prover.stop().await.unwrap();
                    log::info!("Prove Worker stopped");
                    return Ok(())
                }
            }
        }
    }

    pub(crate) async fn verify_worker(
        db: Arc<Box<dyn Database>>,
        settlement_provider: Arc<Box<dyn Settlement>>,
        mut stop_rx: mpsc::Receiver<()>,
        worker_interval: Duration,
    ) -> Result<()> {
        let mut ticker = tokio::time::interval(worker_interval);
        let bridge_service_client = Client::new();
        log::info!("Verify Worker started");
        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    let last_proven_block = match db.get(keys::KEY_LAST_PROVEN_BLOCK_NUMBER) {
                        None => {
                            db.put(keys::KEY_LAST_PROVEN_BLOCK_NUMBER.to_vec(), 0_u64.to_be_bytes().to_vec());
                            0
                        }
                        Some(block_number_bytes) => {
                            u64::from_be_bytes(block_number_bytes.try_into().unwrap())
                        }
                    };
                    let last_verified_block = match db.get(keys::KEY_LAST_VERIFIED_BLOCK_NUMBER) {
                        None => {
                            db.put(keys::KEY_LAST_VERIFIED_BLOCK_NUMBER.to_vec(), 0_u64.to_be_bytes().to_vec());
                            0
                        }
                        Some(block_number_bytes) => {
                            u64::from_be_bytes(block_number_bytes.try_into().unwrap())
                        }
                    };

                    let last_verified_batch = match db.get(keys::KEY_LAST_VERIFIED_BATCH_NUMBER) {
                        None => {
                            db.put(keys::KEY_LAST_VERIFIED_BATCH_NUMBER.to_vec(), 0_u64.to_be_bytes().to_vec());
                            0
                        }
                        Some(block_number_bytes) => {
                            u64::from_be_bytes(block_number_bytes.try_into().unwrap())
                        }
                    };

                    log::info!("last proven block({}), last verified block({}), last verified batch({})", last_proven_block, last_verified_block, last_verified_batch);

                    // if the last proven block is less than or equal to the last verified block, skip
                    // waiting for the new proof of the next block
                    if last_proven_block <= last_verified_block {
                        log::info!("no new proof to verify, try again later");
                        continue;
                    }

                    log::info!("start to verify the proof of the next block({})", last_verified_block + 1);
                    // get the proof of the next block
                    let next_proof_key = format!("{}{}", std::str::from_utf8(prefix::PREFIX_BATCH_PROOF).unwrap(), last_verified_block + 1);
                    if let Some(proof_bytes) = db.get(next_proof_key.as_bytes()) {
                        let proof_data: ProofResult = serde_json::from_slice(&proof_bytes).unwrap();
                        // verify the proof
                        let zeth_last_rollup_exit_root = get_rollup_exit_root_by_block(proof_data.block_number, &bridge_service_client).await?;

                        match settlement_provider.verify_batches(
                            0,
                            last_verified_batch,
                            last_verified_batch + 1,
                            zeth_last_rollup_exit_root,
                            proof_data.post_state_root,
                            proof_data.proof,
                            proof_data.public_input,
                        ).await {
                            Ok(_) => {
                                log::info!("verify proof success, block({})", proof_data.block_number);
                                db.put(keys::KEY_LAST_VERIFIED_BLOCK_NUMBER.to_vec(), proof_data.block_number.to_be_bytes().to_vec());
                                db.put(keys::KEY_LAST_VERIFIED_BATCH_NUMBER.to_vec(), (last_verified_batch + 1).to_be_bytes().to_vec());
                                // verify success, update the block status to Finalized
                                let status_key = format!("{}{}", std::str::from_utf8(prefix::PREFIX_BLOCK_STATUS).unwrap(), proof_data.block_number);
                                let status = Status::Finalized;
                                let encoded_status = serde_json::to_vec(&status).unwrap();
                                db.put(status_key.as_bytes().to_vec(), encoded_status);
                            }
                            Err(e) => {
                                log::error!("verify proof failed, block({}), err: {:?}",proof_data.block_number, e);
                            }
                        }
                    };
                }
                _ = stop_rx.recv() => {
                    log::info!("Verify Worker stopped");
                    return Ok(());
                }
            }
        }
    }

    pub(crate) async fn rollup(
        db: Arc<Box<dyn Database>>,
        l2provider: Provider<Http>,
        settlement_provider: Arc<Box<dyn Settlement>>,
        mut stop_rx: mpsc::Receiver<()>,
        worker_interval: Duration,
    ) -> Result<()> {
        let mut ticker = tokio::time::interval(worker_interval);
        log::info!("Submit Worker started");
        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    // get the last submitted block
                    let last_sequence_finality_block_number = match db.get(keys::KEY_LAST_SEQUENCE_FINALITY_BLOCK_NUMBER) {
                        None => {
                            db.put(keys::KEY_LAST_SEQUENCE_FINALITY_BLOCK_NUMBER.to_vec(), 0_u64.to_be_bytes().to_vec());
                            0
                        }
                        Some(block_number_bytes) => {
                            u64::from_be_bytes(block_number_bytes.try_into().unwrap())
                        }
                    };

                    // get the last fetched block
                    let last_submitted_block = match db.get(keys::KEY_LAST_SUBMITTED_BLOCK_NUMBER) {
                        None => {
                            db.put(keys::KEY_LAST_SUBMITTED_BLOCK_NUMBER.to_vec(), 0_u64.to_be_bytes().to_vec());
                            0
                        }
                        Some(block_number_bytes) => {
                            u64::from_be_bytes(block_number_bytes.try_into().unwrap())
                        }
                    };

                    let last_verified_block = match db.get(keys::KEY_LAST_VERIFIED_BLOCK_NUMBER) {
                        None => {
                            db.put(keys::KEY_LAST_VERIFIED_BLOCK_NUMBER.to_vec(), 0_u64.to_be_bytes().to_vec());
                            0
                        }
                        Some(block_number_bytes) => {
                            u64::from_be_bytes(block_number_bytes.try_into().unwrap())
                        }
                    };

                    if last_submitted_block >= last_sequence_finality_block_number || last_verified_block < last_submitted_block{
                        log::info!("no new block to submit, try again later");
                        continue;
                    }
                    log::info!("start to submit the block({})", last_submitted_block + 1);
                    let number = l2provider.get_block_number().await.map_err(|e| anyhow!("failed to get block number, err: {:?}", e))?;
                    log::info!("get_block_number success, number: {:?}", number);

                    // let block = l2provider.get_block_with_txs(last_submitted_block + 1).await.map_err(|e| anyhow!(e))?.ok_or(anyhow!("block not found"))?;
                    let block = l2provider.get_block_with_txs(BlockId::Number(BlockNumber::Number(U64::from(last_submitted_block + 1)))).await.map_err(|e| anyhow!("failed to get_block_with_txs, err: {:?}", e))?;
                    let block = match block {
                        Some(block) => {
                            log::info!("block({}) found, {:?}", last_submitted_block + 1, block);
                            block
                        },
                        None => {
                            log::info!("block({}) not found", last_submitted_block + 1);
                            continue;
                        }
                    };

                    let block_clone = block.clone();
                    let txs = block.transactions.clone();
                    if txs.is_empty() {
                        log::info!("Block({}) is an empty block, Skip proof generation for empty blocks", last_submitted_block + 1);
                        // NOTE: For empty blocks, we need to do the following things in the specified order:
                        // The order of these steps must not be changed.

                        let execute_result = ProofResult{
                            block_number: last_submitted_block + 1,
                            ..Default::default()
                        };

                        // 1. update the last verified block number, trigger the next verify task

                        db.put(keys::KEY_LAST_VERIFIED_BLOCK_NUMBER.to_vec(), execute_result.block_number.to_be_bytes().to_vec());

                        let status_key = format!("{}{}", std::str::from_utf8(prefix::PREFIX_BLOCK_STATUS).unwrap(), execute_result.block_number);
                        let status = Status::Finalized;
                        let encoded_status = serde_json::to_vec(&status).unwrap();
                        db.put(status_key.as_bytes().to_vec(), encoded_status);

                        // 2. update the last proven block number, trigger the next verify task

                        let key_with_prefix = format!("{}{}", std::str::from_utf8(prefix::PREFIX_BATCH_PROOF).unwrap(), execute_result.block_number);
                        // save the proof to the database
                        let encoded_execute_result = serde_json::to_vec(&execute_result).unwrap();
                        db.put(key_with_prefix.as_bytes().to_vec(), encoded_execute_result);
                        // save the last proven block number, trigger the next verify task
                        db.put(keys::KEY_LAST_PROVEN_BLOCK_NUMBER.to_vec(), execute_result.block_number.to_be_bytes().to_vec());

                        // 3. update the next batch number, trigger the next prove task
                        // packing the next block
                        db.put(keys::KEY_NEXT_BATCH.to_vec(), (last_submitted_block+1).to_be_bytes().to_vec());

                        // 4. update the block status to Submitted, update the last submitted block number
                        log::info!("submit block({}) success", last_submitted_block + 1);
                        db.put(keys::KEY_LAST_SUBMITTED_BLOCK_NUMBER.to_vec(), (last_submitted_block + 1).to_be_bytes().to_vec());

                        db.del(key_with_prefix.as_bytes().to_vec());
                        continue;
                    }
                    let txs_clone = block.transactions;
                    let mut batches = Vec::<BatchData>::new();
                    let global_exit_root = settlement_provider.get_global_exit_root().await.map_err(|e| anyhow!("failed to get global exit root, err: {:?}", e))?;
                    //
                    for tx in txs {
                        let tx_legacy = convert_to_tx_legacy(&tx)?;

                        let mut v_vec = tx.v.to_string().as_bytes().to_vec();
                        let mut r_vec = tx.r.to_string().as_bytes().to_vec();
                        let mut s_vec = tx.s.to_string().as_bytes().to_vec();

                        let mut buf = BytesMut::with_capacity(payload_len_for_signature(&tx_legacy));
                        encode_for_signing(&tx_legacy, &mut buf);
                        let mut rlp_vec = buf.to_vec();
                        rlp_vec.append(&mut v_vec);
                        rlp_vec.append(&mut r_vec);
                        rlp_vec.append(&mut s_vec);

                        // the batches will be changed, now the structure is:
                        // one batches contains one batch_data
                        // one batch_data contains one block
                        // one block contains one transaction
                        let batch_data = BatchData {
                            transactions: rlp_vec,
                            global_exit_root,
                            timestamp: block.timestamp.as_u64(),
                        };
                        batches.push(batch_data);
                    }
                    log::info!("block({:?}), txs({:?}), batches({:?})", block_clone, txs_clone, batches);

                    // BatchData
                    match settlement_provider.sequence_batches(batches).await {
                        Ok(_) => {
                            log::info!("submit block({}) success", last_submitted_block + 1);
                            db.put(keys::KEY_LAST_SUBMITTED_BLOCK_NUMBER.to_vec(), (last_submitted_block + 1).to_be_bytes().to_vec());
                            // update the block status to Submitted
                            let status_key = format!("{}{}", std::str::from_utf8(prefix::PREFIX_BLOCK_STATUS).unwrap(), last_submitted_block + 1);
                            let status = Status::Submitted;
                            let encoded_status = serde_json::to_vec(&status).unwrap();
                            db.put(status_key.as_bytes().to_vec(), encoded_status);
                        }
                        Err(e) => {
                            log::error!("submit block({}) failed: {:?}", last_submitted_block + 1, e);
                        }
                    }
                }
                _ = stop_rx.recv() => {
                    log::info!("Submit Worker stopped");
                    return Ok(());
                }
            }
        }
    }
}

fn convert_to_tx_legacy(tx: &Transaction) -> Result<TxLegacy> {
    // create a legacy transaction
    let chain_id = tx.chain_id.ok_or_else(|| anyhow!("chain id is required"))?;
    let gas_price = tx
        .gas_price
        .ok_or_else(|| anyhow!("gas price is required"))?;
    let input = Bytes::from(tx.input.clone().to_vec());

    let tx_legacy = TxLegacy {
        chain_id: Some(chain_id.as_u64()),
        nonce: tx.nonce.as_u64(),
        gas_price: gas_price.as_u128(),
        gas_limit: tx.gas.as_u64(),
        to: match tx.to {
            Some(address) => {
                TransactionKind::Call(reth_primitives::Address::from_slice(address.as_bytes()))
            }
            None => TransactionKind::Create,
        },
        value: reth_primitives::alloy_primitives::Uint::from(tx.value.as_u128()),
        input,
    };

    Ok(tx_legacy)
}

// === wrap the reth/crates/primitives/src/transaction/legacy.rs TxLegacy private methods ===

pub fn payload_len_for_signature(tx: &TxLegacy) -> usize {
    let payload_length = fields_len(tx) + eip155_fields_len(tx);
    // 'header length' + 'payload length'
    length_of_length(payload_length) + payload_length
}

pub fn encode_for_signing(tx: &TxLegacy, out: &mut dyn bytes::BufMut) {
    let payload_length = fields_len(tx) + eip155_fields_len(tx);
    Header {
        list: true,
        payload_length,
    }
    .encode(out);
    encode_fields(tx, out);
    encode_eip155_fields(tx, out);
}

pub fn fields_len(tx: &TxLegacy) -> usize {
    tx.nonce.length()
        + tx.gas_price.length()
        + tx.gas_limit.length()
        + tx.to.length()
        + tx.value.length()
        + tx.input.0.length()
}

pub fn eip155_fields_len(tx: &TxLegacy) -> usize {
    if let Some(id) = tx.chain_id {
        id.length() + 2
    } else {
        0
    }
}

pub fn encode_fields(tx: &TxLegacy, out: &mut dyn bytes::BufMut) {
    tx.nonce.encode(out);
    tx.gas_price.encode(out);
    tx.gas_limit.encode(out);
    tx.to.encode(out);
    tx.value.encode(out);
    tx.input.0.encode(out);
}

pub fn encode_eip155_fields(tx: &TxLegacy, out: &mut dyn bytes::BufMut) {
    if let Some(id) = tx.chain_id {
        id.encode(out);
        0x00u8.encode(out);
        0x00u8.encode(out);
    }
}

pub async fn get_rollup_exit_root_by_block(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::lfs::libmdbx::{open_mdbx_db, Config};
    use crate::settlement::custom::CustomSettlementConfig;
    use crate::settlement::{init_settlement_provider, NetworkSpec};
    use std::{env, fs};

    #[tokio::test]
    #[ignore = "slow"]
    async fn test_proof_worker() {
        env::set_var("RUST_LOG", "debug");
        env::set_var("DEBUG_PROOF", "TRUE");
        env_logger::init();
        let path = "tmp/test_proof_worker";
        let max_dbs = 20;
        let config = Config {
            path: path.to_string(),
            max_dbs,
        };

        let db = open_mdbx_db(config).unwrap();
        let arc_db = Arc::new(db);

        arc_db.put(
            keys::KEY_LAST_SUBMITTED_BLOCK_NUMBER.to_vec(),
            1_u64.to_be_bytes().to_vec(),
        );
        arc_db.put(keys::KEY_NEXT_BATCH.to_vec(), 0_u64.to_be_bytes().to_vec());

        let (tx, rx) = mpsc::channel(1);
        let stop_rx = rx;

        let conf_path = "configs/settlement.toml";
        let config = WorkerConfig::from_conf_path(conf_path).unwrap();

        let prover = ProverChannel::new("", "", arc_db.clone());
        let submit_worker = Settler::proof_worker(
            arc_db,
            prover,
            stop_rx,
            Duration::from_secs(config.proof_worker_interval),
        );

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(5)).await;

            tx.send(()).await.unwrap();
        });

        submit_worker
            .await
            .map_err(|e| log::error!("submit_worker error: {:?}", e))
            .unwrap();
        // wait for the submit worker to finish
        tokio::time::sleep(Duration::from_secs(5)).await;

        fs::remove_dir_all(path).unwrap();
    }

    #[tokio::test]
    #[ignore = "slow"]
    async fn test_submit_worker() {
        env::set_var("RUST_LOG", "debug");
        env_logger::init();
        let path = "tmp/test_submit_worker";
        let max_dbs = 20;
        let config = Config {
            path: path.to_string(),
            max_dbs,
        };

        let db = open_mdbx_db(config).unwrap();
        let arc_db = Arc::new(db);

        arc_db.put(
            keys::KEY_LAST_SEQUENCE_FINALITY_BLOCK_NUMBER.to_vec(),
            1_u64.to_be_bytes().to_vec(),
        );
        arc_db.put(
            keys::KEY_LAST_SUBMITTED_BLOCK_NUMBER.to_vec(),
            0_u64.to_be_bytes().to_vec(),
        );

        let l2provider = Provider::<Http>::try_from("http://localhost:8546").unwrap();

        // let settlement_conf_path = "configs/settlement.toml";
        // let settlement_spec = NetworkSpec::Ethereum(
        //     EthereumSettlementConfig::from_conf_path(settlement_conf_path).unwrap(),
        // );

        let config = CustomSettlementConfig {
            service_url: GLOBAL_ENV.bridge_service_addr.clone(),
        };
        let settlement_spec = NetworkSpec::Custom(config);

        log::info!("settlement_spec: {:#?}", settlement_spec);

        let settlement_provider = init_settlement_provider(settlement_spec)
            .map_err(|e| anyhow!("Failed to init settlement: {:?}", e))
            .unwrap();
        let arc_settlement_provider = Arc::new(settlement_provider);

        let (tx, rx) = mpsc::channel(1);
        let stop_rx = rx;

        let conf_path = "configs/settlement.toml";
        let config = WorkerConfig::from_conf_path(conf_path).unwrap();
        let submit_worker = Settler::rollup(
            arc_db,
            l2provider,
            arc_settlement_provider,
            stop_rx,
            Duration::from_secs(config.rollup_worker_interval),
        );

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(5)).await;

            tx.send(()).await.unwrap();
        });

        submit_worker
            .await
            .map_err(|e| log::error!("submit_worker error: {:?}", e))
            .unwrap();
        // wait for the submit worker to finish
        tokio::time::sleep(Duration::from_secs(5)).await;

        fs::remove_dir_all(path).unwrap();
    }

    #[tokio::test]
    #[ignore = "slow"]
    async fn test_verify_worker() {
        env::set_var("RUST_LOG", "debug");
        env_logger::init();
        let path = "tmp/test_verify_worker";
        let max_dbs = 20;
        let config = Config {
            path: path.to_string(),
            max_dbs,
        };

        let db = open_mdbx_db(config).unwrap();
        let arc_db = Arc::new(db);

        arc_db.put(
            keys::KEY_LAST_PROVEN_BLOCK_NUMBER.to_vec(),
            1_u64.to_be_bytes().to_vec(),
        );
        arc_db.put(
            keys::KEY_LAST_VERIFIED_BLOCK_NUMBER.to_vec(),
            0_u64.to_be_bytes().to_vec(),
        );

        let next_proof_key = format!(
            "{}{}",
            std::str::from_utf8(prefix::PREFIX_BATCH_PROOF).unwrap(),
            1
        );

        let default_bytes: [u8; 32] = Default::default();
        let proof_data = ProofResult {
            block_number: 1,
            public_input: "[\n  \"8362010813876620668586816087691982027748700171077865203929785134913673568689\"\n]".to_string(),
            proof: "{\"pi_a\":{\"x\":\"9898772573056133167685730031820439200779677251915533133089180906078340598153\",\"y\":\"4988585277003916341444053304584863613990493280622140059976827625686886539523\"},\"pi_b\":{\"x\":[\"20509617804647477524035568417927205406397155061230874893680539238495862881580\",\"6585671937830049132098354513544251392871629080955562897302125989145227322378\"],\"y\":[\"16600973708004378995096185526104442264785941566440480473110565973138301268148\",\"4748797935380146483285828556664955484346426282325689850322414929184461946949\"]},\"pi_c\":{\"x\":\"13887335001047178786869303446566979485698626327666217004092778514366245776228\",\"y\":\"14978410198885453128070499586881293125336387926725723435864499846753605937804\"},\"protocol\":\"groth16\",\"curve\":\"BN128\"}".to_string(),
            pre_state_root: default_bytes,
            post_state_root: default_bytes
        };

        let proof_data_json = serde_json::to_string(&proof_data).unwrap_or_default();
        arc_db.put(
            next_proof_key.as_bytes().to_vec(),
            proof_data_json.as_bytes().to_vec(),
        );

        let config = CustomSettlementConfig {
            service_url: GLOBAL_ENV.bridge_service_addr.clone(),
        };
        let settlement_spec = NetworkSpec::Custom(config);

        log::info!("settlement_spec: {:#?}", settlement_spec);

        let settlement_provider = init_settlement_provider(settlement_spec)
            .map_err(|e| anyhow!("Failed to init settlement: {:?}", e))
            .unwrap();
        let arc_settlement_provider = Arc::new(settlement_provider);

        let (tx, rx) = mpsc::channel(1);
        let stop_rx: mpsc::Receiver<()> = rx;

        let conf_path = "configs/settlement.toml";
        let config = WorkerConfig::from_conf_path(conf_path).unwrap();
        let verify_worker = Settler::verify_worker(
            arc_db,
            arc_settlement_provider,
            stop_rx,
            Duration::from_secs(config.verify_worker_interval),
        );

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(5)).await;

            tx.send(()).await.unwrap();
        });

        verify_worker
            .await
            .map_err(|e| log::error!("verify_worker error: {:?}", e))
            .unwrap();
        // wait for the submit worker to finish
        tokio::time::sleep(Duration::from_secs(5)).await;

        fs::remove_dir_all(path).unwrap();
    }

    #[tokio::test]
    #[ignore = "slow"]
    async fn test_bridge_service() -> Result<(), Box<dyn std::error::Error>> {
        env::set_var("RUST_LOG", "debug");
        env_logger::init();
        let bridge_service_client = Client::new();

        let zeth_last_rollup_exit_root =
            get_rollup_exit_root_by_block(13, &bridge_service_client).await?;
        log::info!(
            "zeth_last_rollup_exit_root: {:?}",
            zeth_last_rollup_exit_root
        );
        Ok(())
    }

    #[test]
    fn test_from_conf_path() {
        let conf_path = "configs/settlement.toml";
        let config = WorkerConfig::from_conf_path(conf_path).unwrap();
        println!("{:#?}", config);
    }
}
