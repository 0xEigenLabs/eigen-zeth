//! The Client of Eigen Proof Network
//! A standalone process is needed to finish:
//! 1) Get the Latest block that not proved, check if current node need to prove it. If so, submit
//!    the proof generation request to proof network;
//! 2) Keep polling if the task is finished.
//! 3) If the task is finished, update the status into proof database, hence the extended RPC module will fetch this and return it to SDK.

use crate::config::env::GLOBAL_ENV;
use crate::db::keys::KEY_PROVE_STEP_RECORD;
use crate::db::{Database, ProofResult};
use crate::prover::provider::prover_service::gen_batch_proof_response;
use crate::prover::provider::prover_service::prover_request::RequestType;
use crate::prover::provider::prover_service::prover_response::ResponseType;
use crate::prover::provider::prover_service::prover_service_client::ProverServiceClient;
use crate::prover::provider::prover_service::{gen_batch_proof_request, GenChunkProof};
use crate::prover::provider::prover_service::{
    Batch, GenAggregatedProofRequest, GenBatchChunks, GenBatchProofRequest, GenFinalProofRequest,
    ProofResultCode, ProverRequest,
};
use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time;
use tokio_stream::wrappers::ReceiverStream;
#[allow(unused_imports)]
use tonic::transport::{Channel, Error};

pub mod prover_service {
    tonic::include_proto!("prover.v1"); // The string specified here must match the proto package name
}

/// ProverChannel ...
// #[derive(Debug)]
pub struct ProverChannel {
    step: ProveStep,
    /// the current batch to prove
    current_batch: Option<BlockNumber>,
    parent_batch: Option<BlockNumber>,
    /// the endpoint to communicate with the prover
    endpoint: Option<ProverEndpoint>,

    request_sender: Sender<ProverRequest>,
    /// used to receive response from the endpoint
    response_receiver: Receiver<ResponseType>,
    endpoint_restart_signal_receiver: Receiver<()>,

    //
    // request_sender2: tokio::sync::broadcast::Sender<ProverRequest>,
    /// final proof
    // final_proof_sender: Sender<Vec<u8>>,

    /// used to stop the endpoint
    stop_endpoint_tx: Sender<()>,
    /// the address of the aggregator
    aggregator_addr: String,
    // TODO: Record state data in ProveStep to allow zeth to continue the current batch upon restart
    #[allow(dead_code)]
    db: Arc<Box<dyn Database>>,
}

type BlockNumber = u64;
type StartChunk = String;
type EndChunk = String;
type RecursiveProof = String;
type ChunkCount = u64;
type L2BatchData = String;

type TaskId = String;

type ErrMsg = String;

type BatchId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BatchStateRoot {
    pre_state_root: [u8; 32],
    post_state_root: [u8; 32],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecuteResult {
    Success(ProofResult),
    // TODO: Fixme
    #[allow(dead_code)]
    Failed(ErrMsg),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProverStepRecord {
    // refactor to Batch
    block_number: Option<BlockNumber>,
    step: Option<ProveStep>,
}

/// ProveStep ...
#[derive(Debug, Clone, Serialize, Deserialize)]
enum ProveStep {
    Start,
    // TODO: refactor to Batch
    Batch(BatchStep),
    Aggregate(AggregateStep),
    Final(FinalStep),
    End(ExecuteResult),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum BatchStep {
    // TODO: refactor to Batch
    GenChunk(BatchId, BlockNumber),
    GenProof(BatchId, TaskId, ChunkCount, L2BatchData, BatchStateRoot),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum AggregateStep {
    Aggregate(BatchId, StartChunk, EndChunk, BatchStateRoot),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum FinalStep {
    Final(BatchId, RecursiveProof, BatchStateRoot),
}

impl fmt::Display for ProveStep {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ProveStep::Start => write!(f, "♥ Start"),
            ProveStep::Batch(b) => match b {
                BatchStep::GenChunk(_batch_id, no) => {
                    write!(f, "♦ Batch: GenChunk({})", no)
                }
                BatchStep::GenProof(
                    _batch_id,
                    task_id,
                    chunk_count,
                    l2_batch_data,
                    batch_state_root,
                ) => {
                    write!(
                        f,
                        "♦ Batch: GenChunkProof(task_id: {}, ChunkCount: {}, L2BatchData: {}, BatchStateRoot: {:?})",
                        task_id, chunk_count, l2_batch_data, batch_state_root
                    )
                }
            },
            ProveStep::Aggregate(a) => match a {
                AggregateStep::Aggregate(_batch_id, s, e, _batch_state_root) => {
                    write!(f, "♠ Agg: {} -> {}", s, e)
                }
            },
            ProveStep::Final(final_step) => match final_step {
                FinalStep::Final(_batch_id, r, _batch_state_root) => write!(f, "♣ Final: {:?}", r),
            },
            ProveStep::End(result) => write!(f, "🌹 End: {:?}", result),
        }
    }
}

impl ProverChannel {
    pub fn new(addr: &str, aggregator_addr: &str, db: Arc<Box<dyn Database>>) -> Self {
        let (response_sender, response_receiver) = mpsc::channel(10);
        let (request_sender, request_receiver) = mpsc::channel(10);
        let (endpoint_restart_signal_sender, endpoint_restart_signal_receiver) = mpsc::channel(1);
        let (stop_tx, stop_rx) = mpsc::channel(1);
        ProverChannel {
            step: ProveStep::Start,
            current_batch: None,
            parent_batch: None,
            endpoint: Some(ProverEndpoint::new(
                addr,
                response_sender,
                request_receiver,
                stop_rx,
                endpoint_restart_signal_sender,
            )),
            request_sender,
            response_receiver,
            stop_endpoint_tx: stop_tx,
            aggregator_addr: aggregator_addr.to_string(),
            endpoint_restart_signal_receiver,
            db,
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        log::info!("Prover Endpoint started");
        // start the endpoint
        // take the endpoint, and spawn a new task
        // the self.endpoint will be None after this
        let mut endpoint = self.endpoint.take().unwrap();

        tokio::spawn(async move {
            loop {
                match endpoint.launch().await {
                    // Only returns Ok when a stop signal is received
                    Ok(_) => {
                        // stop with the signal
                        return;
                    }
                    Err(e) => {
                        // stop with the error, relaunch the endpoint
                        log::error!("ProverEndpoint stopped with error: {:?}", e);
                        log::info!("restarting ProverEndpoint");
                    }
                }
            }
        });

        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        // stop the endpoint
        self.stop_endpoint_tx
            .send(())
            .await
            .map_err(|e| anyhow!("Failed to stop the endpoint: {:?}", e))
            .map(|_| log::info!("ProverChannel stopped"))
    }

    pub async fn execute(&mut self, batch: BlockNumber) -> Result<ProofResult> {
        log::debug!("execute batch {batch}");
        self.set_current_batch(batch)?;

        // return proof for the batch
        let result = self.entry_step().await;
        self.clean_current_batch()?;
        result.map_err(|e| anyhow!("execute batch:{} failed: {:?}", batch, e))
    }
    fn record_prove_step(&mut self, step: ProveStep) -> Result<()> {
        self.db.put(
            KEY_PROVE_STEP_RECORD.to_vec(),
            serde_json::to_vec(&ProverStepRecord {
                block_number: self.current_batch,
                step: Some(step),
            })?,
        );
        Ok(())
    }

    async fn entry_step(&mut self) -> Result<ProofResult> {
        // load the ProverStepRecord
        match self.db.get(KEY_PROVE_STEP_RECORD) {
            Some(record) => {
                let record: Result<ProverStepRecord, _> = serde_json::from_slice(&record);
                match record {
                    Ok(record) => {
                        if record.block_number.is_some() && record.step.is_some() {
                            if record.block_number.unwrap() == self.current_batch.unwrap() {
                                // execute from the record step
                                self.step = record.step.unwrap();
                                log::info!("execute from the record step: {:?}", self.step);
                            } else {
                                log::info!("invalid ProverStepRecord, ignore it, execute from the start step");
                            }
                        } else {
                            log::info!(
                                "invalid ProverStepRecord, ignore it, execute from the start step"
                            );
                        }
                    }
                    Err(_) => {
                        log::info!(
                            "parse ProverStepRecord failed, ignore it, execute from the start step"
                        );
                    }
                }
            }
            None => {
                log::info!("ProverStepRecord not found, execute from the start step");
            }
        };

        loop {
            self.step = match &self.step {
                ProveStep::Start => {
                    let batch = self.current_batch.unwrap();
                    // TODO: remove the batch_id
                    let batch_id = uuid::Uuid::new_v4().to_string();

                    let next_step = ProveStep::Batch(BatchStep::GenChunk(batch_id, batch));
                    self.record_prove_step(next_step.clone())?;

                    next_step
                }

                ProveStep::Batch(batch_step) => {
                    match batch_step {
                        BatchStep::GenChunk(batch_id, batch) => {
                            let request = ProverRequest {
                                id: uuid::Uuid::new_v4().to_string(),
                                request_type: Some(RequestType::GenBatchProof(
                                    GenBatchProofRequest {
                                        step: Some(gen_batch_proof_request::Step::GenBatchChunks(
                                            GenBatchChunks {
                                                batch_id: batch_id.clone(),
                                                batch: Some(Batch {
                                                    block_number: vec![*batch],
                                                }),
                                                chain_id: GLOBAL_ENV.chain_id,
                                                program_name: GLOBAL_ENV.program_name.clone(),
                                            },
                                        )),
                                    },
                                )),
                            };
                            // send request to the endpoint
                            self.request_sender.send(request).await?;

                            tokio::select! {
                               resp = self.response_receiver.recv() => {
                                // waiting for the response from the endpoint
                                if let Some(ResponseType::GenBatchProof(gen_batch_proof_response)) = resp {
                                        if let Some(gen_batch_proof_response::Step::GenBatchChunks(gen_batch_chunks_result)) = gen_batch_proof_response.step {
                                            if gen_batch_chunks_result.result_code == ProofResultCode::CompletedOk as i32 {

                                                let task_id = gen_batch_chunks_result.task_id;
                                                let chunk_count = gen_batch_chunks_result.chunk_count;
                                                let l2_batch_data = gen_batch_chunks_result.batch_data;
                                                let batch_state_root = BatchStateRoot {
                                                    pre_state_root: <[u8; 32]>::try_from(gen_batch_chunks_result.pre_state_root).map_err(|_| anyhow!(""))?,
                                                    post_state_root: <[u8; 32]>::try_from(gen_batch_chunks_result.post_state_root).map_err(|_| anyhow!(""))?,
                                                };


                                                let next_step = ProveStep::Batch(BatchStep::GenProof(batch_id.clone(), task_id, chunk_count, l2_batch_data, batch_state_root));
                                                                    self.record_prove_step(next_step.clone())?;
                                                next_step

                                            } else {
                                                log::error!("gen batch chunk failed, err: {}, try again", gen_batch_chunks_result.error_message);
                                                ProveStep::Batch(batch_step.clone())
                                            }
                                        } else {
                                            log::info!("gen batch chunk failed, err: invalid response, try again");
                                            ProveStep::Batch(batch_step.clone())
                                        }
                                } else {
                                    log::info!("gen batch chunk failed, err: invalid response, try again");
                                    ProveStep::Batch(batch_step.clone())
                                }
                              }
                               _ = self.endpoint_restart_signal_receiver.recv() => {
                                    log::error!("gen batch chunk failed, err: endpoint restart, try again later");
                                    ProveStep::Batch(batch_step.clone())
                               }
                            }
                        }
                        BatchStep::GenProof(
                            batch_id,
                            task_id,
                            chunk_count,
                            l2_batch_data,
                            batch_state_root,
                        ) => {
                            let request = ProverRequest {
                                id: uuid::Uuid::new_v4().to_string(),
                                request_type: Some(RequestType::GenBatchProof(
                                    GenBatchProofRequest {
                                        step: Some(gen_batch_proof_request::Step::GenChunkProof(
                                            GenChunkProof {
                                                batch_id: batch_id.clone(),
                                                task_id: task_id.clone(),
                                                chunk_count: *chunk_count,
                                                chain_id: GLOBAL_ENV.chain_id,
                                                program_name: GLOBAL_ENV.program_name.clone(),
                                                batch_data: l2_batch_data.clone(),
                                            },
                                        )),
                                    },
                                )),
                            };

                            // send request to the endpoint
                            self.request_sender.send(request).await?;

                            tokio::select! {
                               resp = self.response_receiver.recv() => {
                                if let Some(ResponseType::GenBatchProof(gen_batch_proof_response)) = resp {
                                    if let Some(gen_batch_proof_response::Step::GenChunkProof(gen_chunk_proof_result)) = gen_batch_proof_response.step {
                                       if gen_chunk_proof_result.result_code == ProofResultCode::CompletedOk as i32 {
                                          let chunks = gen_chunk_proof_result.batch_proof_result.unwrap().chunk_proofs;

                                          let start_chunk = chunks.first().unwrap().clone().proof;
                                          let end_chunk = chunks.last().unwrap().clone().proof;
                                          let next_step = ProveStep::Aggregate(AggregateStep::Aggregate(batch_id.clone(), start_chunk, end_chunk, batch_state_root.clone()));
                                                                    self.record_prove_step(next_step.clone())?;
                                            next_step

                                       } else {
                                          log::error!("gen chunk proof failed, err: {}, try again", gen_chunk_proof_result.error_message);
                                          ProveStep::Batch(batch_step.clone())
                                       }
                                    } else {
                                        log::error!("gen chunk proof failed, err: invalid response, try again");
                                        ProveStep::Batch(batch_step.clone())
                                    }
                                } else {
                                    log::error!("gen chunk proof failed, err: invalid response, try again");
                                    ProveStep::Batch(batch_step.clone())
                                }
                              }
                               _ = self.endpoint_restart_signal_receiver.recv() => {
                                    log::error!("gen chunk proof failed, err: endpoint restart, try again later");
                                    ProveStep::Batch(batch_step.clone())
                               }
                            }
                        }
                    }
                }

                ProveStep::Aggregate(agg_step) => {
                    match agg_step {
                        AggregateStep::Aggregate(
                            batch_id,
                            start_chunk,
                            end_chunk,
                            batch_state_root,
                        ) => {
                            let request = ProverRequest {
                                id: uuid::Uuid::new_v4().to_string(),
                                request_type: Some(RequestType::GenAggregatedProof(
                                    GenAggregatedProofRequest {
                                        batch_id: batch_id.clone(),
                                        recursive_proof_1: start_chunk.clone(),
                                        recursive_proof_2: end_chunk.clone(),
                                    },
                                )),
                            };
                            // send request to the endpoint
                            self.request_sender.send(request).await?;

                            // waiting for the response from the endpoint
                            if let Some(ResponseType::GenAggregatedProof(
                                gen_aggregated_proof_response,
                            )) = self.response_receiver.recv().await
                            {
                                if gen_aggregated_proof_response.result_code
                                    == ProofResultCode::CompletedOk as i32
                                {
                                    let recursive_proof =
                                        gen_aggregated_proof_response.result_string;
                                    let next_step = ProveStep::Final(FinalStep::Final(
                                        batch_id.clone(),
                                        recursive_proof,
                                        batch_state_root.clone(),
                                    ));
                                    self.record_prove_step(next_step.clone())?;
                                    next_step
                                } else {
                                    log::error!(
                                        "gen aggregated proof failed, err: {}, try again",
                                        gen_aggregated_proof_response.error_message
                                    );
                                    ProveStep::Aggregate(agg_step.clone())
                                }
                            } else {
                                log::error!(
                                    "gen aggregated proof failed, err: invalid response, try again"
                                );
                                ProveStep::Aggregate(agg_step.clone())
                            }
                        }
                    }
                }

                ProveStep::Final(final_step) => {
                    match final_step {
                        FinalStep::Final(batch_id, recursive_proof, batch_state_root) => {
                            let request = ProverRequest {
                                id: uuid::Uuid::new_v4().to_string(),
                                request_type: Some(RequestType::GenFinalProof(
                                    GenFinalProofRequest {
                                        batch_id: batch_id.clone(),
                                        recursive_proof: recursive_proof.clone(),
                                        curve_name: GLOBAL_ENV.curve_type.clone(),
                                        aggregator_addr: self.aggregator_addr.clone(),
                                    },
                                )),
                            };
                            self.request_sender.send(request).await?;

                            // waiting for the response from the endpoint
                            if let Some(ResponseType::GenFinalProof(gen_final_proof_response)) =
                                self.response_receiver.recv().await
                            {
                                if gen_final_proof_response.result_code
                                    == ProofResultCode::CompletedOk as i32
                                {
                                    if let Some(final_proof) = gen_final_proof_response.final_proof
                                    {
                                        let next_step =
                                            ProveStep::End(ExecuteResult::Success(ProofResult {
                                                block_number: self.current_batch.unwrap(),
                                                proof: final_proof.proof,
                                                public_input: final_proof.public_input,
                                                pre_state_root: batch_state_root.pre_state_root,
                                                post_state_root: batch_state_root.post_state_root,
                                            }));
                                        self.record_prove_step(next_step.clone())?;
                                        next_step
                                    } else {
                                        log::error!(
                                            "gen final proof failed, err: {}, try again",
                                            gen_final_proof_response.error_message
                                        );
                                        ProveStep::Final(final_step.clone())
                                    }
                                } else {
                                    log::error!(
                                        "gen final proof failed, err: {}, try again",
                                        gen_final_proof_response.error_message
                                    );
                                    ProveStep::Final(final_step.clone())
                                }
                            } else {
                                log::error!(
                                    "gen final proof failed, err: invalid response, try again"
                                );
                                ProveStep::Final(final_step.clone())
                            }
                        }
                    }
                }

                ProveStep::End(execute_result) => {
                    let result = (*execute_result).clone();
                    // reset smt state
                    // clear the ProveStepRecord
                    self.db.del(KEY_PROVE_STEP_RECORD.to_vec());
                    self.step = ProveStep::Start;

                    return match result {
                        ExecuteResult::Success(r) => Ok(r),
                        ExecuteResult::Failed(err) => bail!("{}", err),
                    };
                }
            };
            log::debug!("Status: {:?}, {}", self.current_batch, self.step);
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    fn set_current_batch(&mut self, batch: BlockNumber) -> Result<()> {
        self.step = ProveStep::Start;
        self.parent_batch.clone_from(&self.current_batch);
        self.current_batch = Some(batch);
        Ok(())
    }

    fn clean_current_batch(&mut self) -> Result<()> {
        self.parent_batch.clone_from(&self.current_batch);
        self.current_batch = None;
        Ok(())
    }

    pub fn get_current_batch(&self) -> Option<u64> {
        self.current_batch
    }
}

/// ProverEndpoint used to communicate with the prover
#[derive(Debug)]
pub struct ProverEndpoint {
    /// the address of the prover
    addr: String,
    /// used to send request to the gRPC client
    // request_sender: Sender<ProverRequest>,
    /// used to receive request, and send to ProverServer
    // request_receiver: Option<Receiver<ProverRequest>>,
    request_receiver: Receiver<ProverRequest>,
    // /// used to stop the endpoint
    // stop_endpoint_tx: Sender<()>,
    /// listen to the stop signal, and stop the endpoint loop
    stop_endpoint_rx: Receiver<()>,

    /// used to send response to the ProverChannel
    response_sender: Sender<ResponseType>,

    endpoint_restart_signal_sender: Sender<()>,
}

impl ProverEndpoint {
    pub fn new(
        addr: &str,
        response_sender: Sender<ResponseType>,
        request_receiver: Receiver<ProverRequest>,
        stop_rx: Receiver<()>,
        endpoint_restart_signal_sender: Sender<()>,
    ) -> Self {
        ProverEndpoint {
            addr: addr.to_string(),
            request_receiver,
            stop_endpoint_rx: stop_rx,
            response_sender,
            endpoint_restart_signal_sender,
        }
    }

    /// launch the endpoint
    pub async fn launch(&mut self) -> Result<()> {
        let mut client;
        loop {
            match ProverServiceClient::connect(self.addr.clone()).await {
                Ok(c) => {
                    client = c;
                    log::info!("ProverEndpoint connected to {}", self.addr);
                    break;
                }
                Err(e) => {
                    log::error!(
                        "ProverEndpoint connect to server: {} failed: {:?}, try again later",
                        self.addr,
                        e
                    );
                    time::sleep(Duration::from_secs(5)).await;
                }
            }
        }

        let (tx, rx) = mpsc::channel(10);

        // take the request receiver, create a stream
        // self.request_receiver will be None after this
        // let request = ReceiverStream::new(self.request_receiver.take().unwrap());
        let request = ReceiverStream::new(rx);

        // waiting for the first request
        let response = client.prover_stream(request).await?;
        let mut resp_stream = response.into_inner();

        loop {
            tokio::select! {
                _ = self.stop_endpoint_rx.recv() => {
                    log::info!("ProverEndpoint stopped");
                    return Ok(());
                }
                // TODO: Optimize this
                prover_request = self.request_receiver.recv() => {
                    if let Some(request) = prover_request {
                        tx.send(request).await?;
                    }
                }
                recv_msg_result = resp_stream.message() => {
                    match recv_msg_result {
                        Ok(Some(recv_msg)) => {
                            if let Some(msg_type) = recv_msg.response_type {
                                match msg_type {
                                    ResponseType::GetStatus(r) => {
                                        // TODO: Get Prover Status
                                        log::info!("GetStatusResponse: {:?}", r);
                                    }
                                    ResponseType::GenBatchProof(r) => {
                                        log::info!("GenBatchProofResponse: {:?}", r);
                                        self.response_sender.send(ResponseType::GenBatchProof(r)).await?;
                                    }
                                    ResponseType::GenAggregatedProof(r) => {
                                        log::info!("GenAggregatedProofResponse: {:?}", r);
                                        self.response_sender.send(ResponseType::GenAggregatedProof(r)).await?;
                                    }
                                    ResponseType::GenFinalProof(r) => {
                                        log::info!("GenFinalProofResponse: {:?}", r);
                                        self.response_sender.send(ResponseType::GenFinalProof(r)).await?;
                                    }
                                }
                            }
                        }

                        Ok(None) => {
                            loop {
                                match self.request_receiver.try_recv() {
                                    Ok(req) => {
                                        log::info!("drop request: {:?}", req);
                                    },
                                    Err(_) => {
                                        log::info!("request channel is cleared");
                                        break;
                                    }
                                }
                            }
                            self.endpoint_restart_signal_sender.send(()).await?;
                            bail!("ProverEndpoint stopped, server closed the connection");
                        }
                        Err(e) => {
                            loop {
                                match self.request_receiver.try_recv() {
                                    Ok(req) => {
                                        log::info!("drop request: {:?}", req);
                                    },
                                    Err(_) => {
                                        log::info!("request channel is cleared");
                                        break;
                                    }
                                }
                            }
                            self.endpoint_restart_signal_sender.send(()).await?;
                            bail!("ProverEndpoint stopped with error: {:?}", e);
                        }
                    }
                }
            }
        }
    }
}
