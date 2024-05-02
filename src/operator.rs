//! Initialize all components of the eigen-zeth full node.
//! They will be launched in Run CMD.

// TODO: Fixme
#![allow(unused_imports)]

use crate::prover::ProverChannel;
use crate::settlement::{init_settlement, NetworkSpec, Settlement};
use anyhow::{anyhow, Result};
use ethers_core::types::{Bytes, H160, U256};
use ethers_providers::{Http, Provider};
use std::sync::Arc;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::time::{interval, Duration};

use crate::config::env::GLOBAL_ENV;
use crate::db::{lfs, Database};
use crate::settlement::ethereum::{EthereumSettlement, EthereumSettlementConfig};

pub(crate) struct Operator {
    db: Box<dyn Database>,
    prover: ProverChannel,
    settler: Box<dyn Settlement>,
    proof_sender: Sender<Vec<u8>>,
    proof_receiver: Receiver<Vec<u8>>,
}

impl Operator {
    pub fn new(
        _l1addr: &str,
        prover_addr: &str,
        settlement_spec: NetworkSpec,
        db_config: lfs::DBConfig,
        aggregator_addr: &str,
    ) -> Result<Self> {
        let (proof_sender, proof_receiver) = mpsc::channel(10);

        // initialize the prover
        let prover = ProverChannel::new(prover_addr, aggregator_addr);

        // initialize the database
        let db = lfs::open_db(db_config).map_err(|e| anyhow!("Failed to open db: {:?}", e))?;

        // initialize the settlement layer
        let settler = init_settlement(settlement_spec)
            .map_err(|e| anyhow!("Failed to init settlement: {:?}", e))?;

        Ok(Operator {
            prover,
            db,
            settler,
            proof_sender,
            proof_receiver,
        })
    }

    pub async fn run(&mut self, mut stop_channel: Receiver<()>) -> Result<()> {
        let mut ticker = interval(Duration::from_millis(1000));
        let batch_key = "next_batch".to_string().as_bytes().to_vec();
        let proof_key = "batch_proof".to_string().as_bytes().to_vec();

        // start the endpoint
        self.prover.start().await.unwrap();

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    let next_batch = self.db.get(&batch_key);
                    let current_batch = self.prover.get_current_batch();
                    log::debug!("fetch block {:?}, {:?}", next_batch, current_batch);

                    match (next_batch, current_batch){
                        (None, None) => {
                            // insert the first block
                            self.db.put(batch_key.clone(), 1_u64.to_be_bytes().to_vec());
                        },
                        (Some(no), None) => {
                            let block_no = u64::from_be_bytes(no.try_into().unwrap());
                            let prover_task = self.prover.execute(block_no);
                            tokio::select! {
                                result = prover_task => {
                                    match result {
                                        Ok(execute_result) => {
                                           log::info!("execute batch {} success: {:?}", block_no, execute_result);
                                            // TODO: send proof and public inputs to the settlement layer
                                            self.proof_sender.send(Vec::from(execute_result.proof)).await.unwrap();
                                        }
                                        Err(e) => {
                                            log::error!("execute batch {} failed: {:?}", block_no, e);
                                            // TODO: retry or skip?
                                        }
                                    }

                                    // trigger the next task
                                    let block_no_next = block_no + 1;
                                    self.db.put(batch_key.clone(), block_no_next.to_be_bytes().to_vec());
                                }

                                _ = stop_channel.recv() => {
                                    self.prover.stop().await.unwrap();
                                    log::info!("Operator stopped");
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
                proof_data = self.proof_receiver.recv() => {
                    log::debug!("fetch proof: {:?}", proof_data);
                    self.db.put(proof_key.clone(), proof_data.unwrap());

                    // TODO: verify the proof
                    let _ = self.settler.bridge_asset(0, H160::zero(), U256::zero(), H160::zero(), true, Bytes::default()).await;
                }
                _ = stop_channel.recv() => {
                    self.prover.stop().await.unwrap();
                }
            }
        }
    }
}
