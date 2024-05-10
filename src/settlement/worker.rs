use crate::db::{keys, prefix, Database};
use crate::prover::provider::ProofResult;
use crate::prover::ProverChannel;
use crate::settlement::Settlement;
use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

const PROOF_INTERVAL: Duration = Duration::from_secs(1);
const VERIFY_INTERVAL: Duration = Duration::from_secs(1);

pub(crate) struct Settler {}

impl Settler {
    pub(crate) async fn proof_worker(
        db: Arc<Box<dyn Database>>,
        mut prover: ProverChannel,
        mut stop_rx: mpsc::Receiver<()>,
    ) -> Result<()> {
        let mut ticker = tokio::time::interval(PROOF_INTERVAL);
        prover.start().await.unwrap();

        log::info!("Prove Worker started");
        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    let last_sequence_finality_block = match db.get(keys::KEY_LAST_SEQUENCE_FINALITY_BLOCK_NUMBER) {
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
                            1
                        }
                        Some(block_number_bytes) => {
                            u64::from_be_bytes(block_number_bytes.try_into().unwrap())
                        }
                    };

                    //
                    if next_batch > last_sequence_finality_block {
                        continue;
                    }

                    let next_batch = db.get(keys::KEY_NEXT_BATCH);
                    let current_batch = prover.get_current_batch();
                    log::debug!("fetch block {:?}, {:?}", next_batch, current_batch);

                    match (next_batch, current_batch){
                        (None, None) => {
                            // insert the first block
                            db.put(keys::KEY_NEXT_BATCH.to_vec(), 1_u64.to_be_bytes().to_vec());
                        },
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
                                            log::error!("execute batch {} failed: {:?}", block_no, e);
                                            // TODO: retry or skip?
                                        }
                                    }

                                    // update the next batch number, trigger the next prove task
                                    let block_no_next = block_no + 1;
                                    db.put(keys::KEY_NEXT_BATCH.to_vec(), block_no_next.to_be_bytes().to_vec());
                                }

                                _ = stop_rx.recv() => {
                                    log::info!("Prove Worker stopped");
                                    return Ok(());
                                }
                            }
                        },
                        (None, Some(no)) => todo!("Invalid branch, block: {no}"),
                        (Some(next), Some(cur)) => {
                            let block_no = u64::from_be_bytes(next.try_into().unwrap());
                            log::debug!("next: {block_no}, current: {cur}");
                        },
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
    ) -> Result<()> {
        let mut ticker = tokio::time::interval(VERIFY_INTERVAL);
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

                    log::info!("last proven block({}), last verified block({})", last_proven_block, last_verified_block);

                    // if the last proven block is less than or equal to the last verified block, skip
                    // waiting for the new proof of the next block
                    if last_proven_block <= last_verified_block {
                        log::info!("no new proof to verify, try again later");
                        continue;
                    }

                    log::info!("start to verify the proof of the next block({})", last_verified_block + 1);
                    // get the proof of the next block
                    let next_proof_key = format!("{}{}", std::str::from_utf8(prefix::PREFIX_BATCH_PROOF).unwrap(), last_verified_block + 1);
                    if let Some(proof_bytes) = db.get(next_proof_key.as_bytes()){
                        let proof_data: ProofResult = serde_json::from_slice(&proof_bytes).unwrap();
                        // verify the proof
                        // TODO: update the new_local_exit_root
                        match settlement_provider.verify_batches(
                            0,
                            last_verified_block,
                            last_verified_block + 1,
                            [0; 32],
                            proof_data.post_state_root,
                            proof_data.proof,
                            proof_data.public_input,
                        ).await {
                            Ok(_) => {
                                log::info!("verify proof success, block({})", proof_data.block_number);
                                db.put(keys::KEY_LAST_VERIFIED_BLOCK_NUMBER.to_vec(), proof_data.block_number.to_be_bytes().to_vec());
                            }
                            Err(e) => {
                                log::error!("verify proof failed, block({}), err: {:?}",proof_data.block_number, e);
                                // TODO:
                                db.put(keys::KEY_LAST_VERIFIED_BLOCK_NUMBER.to_vec(), proof_data.block_number.to_be_bytes().to_vec());
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
}
