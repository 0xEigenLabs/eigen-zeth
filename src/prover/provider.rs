//! The Client of Eigen Proof Network
//! A standalone process is needed to finish:
//! 1) Get the Latest block that not proved, check if current node need to prove it. If so, submit
//!    the proof generation request to proof network;
//! 2) Keep polling if the task is finished.
//! 3) If the task is finished, update the status into proof database, hence the extended RPC module will fetch this and return it to SDK.

use crate::config::env::GLOBAL_ENV;
use crate::db::ProofResult;
use crate::prover::provider::prover_service::prover_request::RequestType;
use crate::prover::provider::prover_service::prover_response::ResponseType;
use crate::prover::provider::prover_service::prover_service_client::ProverServiceClient;
use crate::prover::provider::prover_service::{
    Batch, GenAggregatedProofRequest, GenBatchProofRequest, GenFinalProofRequest, ProofResultCode,
    ProverRequest,
};
use anyhow::{anyhow, bail, Result};
use std::fmt;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time;
use tokio_stream::wrappers::ReceiverStream;

pub mod prover_service {
    tonic::include_proto!("prover.v1"); // The string specified here must match the proto package name
}

/// ProverChannel ...
#[derive(Debug)]
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

    /// final proof
    // final_proof_sender: Sender<Vec<u8>>,

    /// used to stop the endpoint
    stop_endpoint_tx: Sender<()>,
    /// the address of the aggregator
    aggregator_addr: String,
}

type BlockNumber = u64;
type StartChunk = String;
type EndChunk = String;
type RecursiveProof = String;

type ErrMsg = String;

type BatchId = String;

#[derive(Debug, Clone)]
pub enum ExecuteResult {
    Success(ProofResult),
    Failed(ErrMsg),
}

/// ProveStep ...
#[derive(Debug)]
enum ProveStep {
    Start,
    // TODO: refactor to Batch
    Batch(BatchId, BlockNumber),
    Aggregate(BatchId, StartChunk, EndChunk),
    Final(BatchId, RecursiveProof),
    End(ExecuteResult),
}

impl fmt::Display for ProveStep {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ProveStep::Start => write!(f, "♥ Start"),
            ProveStep::Batch(_batch_id, no) => write!(f, "♦ Batch: {}", no),
            ProveStep::Aggregate(_batch_id, s, e) => write!(f, "♠ Agg: {} -> {}", s, e),
            ProveStep::Final(_batch_id, r) => write!(f, "♣ Final: {:?}", r),
            ProveStep::End(result) => write!(f, "🌹 End: {:?}", result),
        }
    }
}

impl ProverChannel {
    pub fn new(addr: &str, aggregator_addr: &str) -> Self {
        let (response_sender, response_receiver) = mpsc::channel(10);
        let (request_sender, request_receiver) = mpsc::channel(10);
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
            )),
            request_sender,
            response_receiver,
            stop_endpoint_tx: stop_tx,
            aggregator_addr: aggregator_addr.to_string(),
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        log::info!("Prover Endpoint started");
        // start the endpoint
        // self.endpoint.launch().await;

        // take the endpoint, and spawn a new task
        // the self.endpoint will be None after this
        // TODO: handle the error, and relaunch the endpoint
        let mut endpoint = self.endpoint.take().unwrap();
        // loop {
        //     tokio::select! {
        //         r = endpoint.launch() => {
        //             match r {
        //                 Ok(_) => {
        //                     // stop with the signal
        //                     return Ok(())
        //                 }
        //                 Err(e) => {
        //                     // stop with the error
        //                     // TODO: relaunch the endpoint
        //                     log::error!("ProverEndpoint error: {:?}", e);
        //                 }
        //             }
        //         }
        //     }
        // }

        tokio::spawn(async move {
            loop {
                match endpoint.launch().await {
                    Ok(_) => {
                        // stop with the signal
                        return;
                    }
                    Err(e) => {
                        // stop with the error
                        // TODO: relaunch the endpoint
                        log::error!("ProverEndpoint stopped with error, try again later, err: {:?}", e);
                        time::sleep(Duration::from_secs(10)).await;
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

    async fn entry_step(&mut self) -> Result<ProofResult> {
        loop {
            self.step = match &self.step {
                ProveStep::Start => {
                    let batch = self.current_batch.unwrap();
                    let batch_id = uuid::Uuid::new_v4().to_string();
                    ProveStep::Batch(batch_id, batch)
                }

                ProveStep::Batch(batch_id, batch) => {
                    let request = ProverRequest {
                        id: uuid::Uuid::new_v4().to_string(),
                        request_type: Some(RequestType::GenBatchProof(GenBatchProofRequest {
                            batch_id: batch_id.clone(),
                            batch: Some(Batch {
                                block_number: vec![*batch],
                            }),
                            chain_id: GLOBAL_ENV.chain_id,
                            program_name: GLOBAL_ENV.program_name.clone(),
                        })),
                    };
                    // send request to the endpoint
                    self.request_sender.send(request).await?;

                    // waiting for the response from the endpoint
                    if let Some(ResponseType::GenBatchProof(gen_batch_proof_response)) =
                        self.response_receiver.recv().await
                    {
                        if gen_batch_proof_response.result_code
                            == ProofResultCode::CompletedOk as i32
                        {
                            let chunks = gen_batch_proof_response
                                .batch_proof_result
                                .unwrap()
                                .chunk_proofs;
                            let start_chunk = chunks.first().unwrap().clone().proof;
                            let end_chunk = chunks.last().unwrap().clone().proof;
                            ProveStep::Aggregate(batch_id.clone(), start_chunk, end_chunk)
                        } else {
                            ProveStep::End(ExecuteResult::Failed(format!(
                                "gen batch proof failed, err: {}",
                                gen_batch_proof_response.error_message
                            )))
                        }
                    } else {
                        ProveStep::End(ExecuteResult::Failed(
                            "gen batch proof failed, err: invalid response".to_string(),
                        ))
                    }
                }

                ProveStep::Aggregate(batch_id, start_chunk, end_chunk) => {
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
                    if let Some(ResponseType::GenAggregatedProof(gen_aggregated_proof_response)) =
                        self.response_receiver.recv().await
                    {
                        if gen_aggregated_proof_response.result_code
                            == ProofResultCode::CompletedOk as i32
                        {
                            let recursive_proof = gen_aggregated_proof_response.result_string;
                            ProveStep::Final(batch_id.clone(), recursive_proof)
                        } else {
                            ProveStep::End(ExecuteResult::Failed(format!(
                                "gen aggregated proof failed, err: {}",
                                gen_aggregated_proof_response.error_message
                            )))
                        }
                    } else {
                        ProveStep::End(ExecuteResult::Failed(
                            "gen aggregated proof failed, err: invalid response".to_string(),
                        ))
                    }
                }

                ProveStep::Final(batch_id, recursive_proof) => {
                    let request = ProverRequest {
                        id: uuid::Uuid::new_v4().to_string(),
                        request_type: Some(RequestType::GenFinalProof(GenFinalProofRequest {
                            batch_id: batch_id.clone(),
                            recursive_proof: recursive_proof.clone(),
                            curve_name: GLOBAL_ENV.curve_type.clone(),
                            aggregator_addr: self.aggregator_addr.clone(),
                        })),
                    };
                    self.request_sender.send(request).await?;

                    // waiting for the response from the endpoint
                    if let Some(ResponseType::GenFinalProof(gen_final_proof_response)) =
                        self.response_receiver.recv().await
                    {
                        if gen_final_proof_response.result_code
                            == ProofResultCode::CompletedOk as i32
                        {
                            if let Some(final_proof) = gen_final_proof_response.final_proof {
                                ProveStep::End(ExecuteResult::Success(ProofResult {
                                    block_number: self.current_batch.unwrap(),
                                    proof: final_proof.proof,
                                    public_input: final_proof.public_input,
                                    pre_state_root: <[u8; 32]>::try_from(
                                        final_proof.pre_state_root,
                                    )
                                    .map_err(|_| anyhow!(""))?,
                                    post_state_root: <[u8; 32]>::try_from(
                                        final_proof.post_state_root,
                                    )
                                    .map_err(|_| anyhow!(""))?,
                                }))
                            } else {
                                ProveStep::End(ExecuteResult::Failed(
                                    "gen final proof failed, invalid response, proof is None"
                                        .to_string(),
                                ))
                            }
                        } else {
                            ProveStep::End(ExecuteResult::Failed(format!(
                                "gen final proof failed: {}",
                                gen_final_proof_response.error_message
                            )))
                        }
                    } else {
                        ProveStep::End(ExecuteResult::Failed(
                            "gen final proof failed, err: invalid response".to_string(),
                        ))
                    }
                }

                ProveStep::End(execute_result) => {
                    let result = (*execute_result).clone();
                    // reset smt state
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
    request_receiver: Option<Receiver<ProverRequest>>,
    // /// used to stop the endpoint
    // stop_endpoint_tx: Sender<()>,
    /// listen to the stop signal, and stop the endpoint loop
    stop_endpoint_rx: Receiver<()>,

    /// used to send response to the ProverChannel
    response_sender: Sender<ResponseType>,
}

impl ProverEndpoint {
    pub fn new(
        addr: &str,
        response_sender: Sender<ResponseType>,
        request_receiver: Receiver<ProverRequest>,
        stop_rx: Receiver<()>,
    ) -> Self {
        ProverEndpoint {
            addr: addr.to_string(),
            request_receiver: Some(request_receiver),
            stop_endpoint_rx: stop_rx,
            response_sender,
        }
    }

    /// launch the endpoint
    pub async fn launch(&mut self) -> Result<()> {
        let mut client = ProverServiceClient::connect(self.addr.clone()).await?;

        log::info!("ProverEndpoint connected to {}", self.addr);

        // take the request receiver, create a stream
        // self.request_receiver will be None after this
        let request = ReceiverStream::new(self.request_receiver.take().unwrap());

        // waiting for the first request
        let response = client.prover_stream(request).await?;
        let mut resp_stream = response.into_inner();

        loop {
            tokio::select! {
                _ = self.stop_endpoint_rx.recv() => {
                    log::info!("ProverEndpoint stopped");
                    return Ok(());
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
                            log::info!("Stream ended");
                            tokio::time::sleep(Duration::from_secs(10)).await; // add delay
                        }
                        Err(e) => {
                            log::error!("Error receiving message: {}", e);
                            tokio::time::sleep(Duration::from_secs(10)).await; // add delay
                        }
                    }
                }
            }
        }
    }
}
