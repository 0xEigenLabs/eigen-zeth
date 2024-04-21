//! The Client of Eigen Proof Network
//! A standalone process is needed to finish:
//! 1) Get the Latest block that not proved, check if current node need to prove it. If so, submit
//!    the proof generation request to proof network;
//! 2) Keep polling if the task is finished.
//! 3) If the task is finished, update the status into proof database, hence the extended RPC module will fetch this and return it to SDK.
// TODO: Fix me
#![allow(dead_code)]

use crate::prover::provider::prover_service::prover_request::RequestType;
use crate::prover::provider::prover_service::prover_response::ResponseType;
use crate::prover::provider::prover_service::prover_service_client::ProverServiceClient;
use crate::prover::provider::prover_service::{
    Batch, GenAggregatedProofRequest, GenBatchProofRequest, GenFinalProofRequest, ProofResultCode,
    ProverRequest,
};
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio_stream::wrappers::ReceiverStream;

pub mod prover_service {
    tonic::include_proto!("prover.v1"); // The string specified here must match the proto package name
}

/// ProverProvider is used to generate proof for the batch
pub struct ProverProvider {
    /// prove SMT, Call the Prover step by step to generate proof for the batch
    inner: ProveSMT,
}

impl ProverProvider {
    pub fn new() -> Self {
        let addr = std::env::var("PROVER_ADDR").unwrap_or("http://127.0.0.1:50051".to_string());
        ProverProvider {
            inner: ProveSMT::new(addr),
        }
    }

    /// start the prover
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.inner.start().await
    }

    /// stop the prover
    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.inner.stop().await
    }

    /// generate proof for the batch
    pub async fn prove(&mut self, batch: BlockNumber) -> Result<(), Box<dyn std::error::Error>> {
        self.inner.execute(batch).await
    }
}

/// ProveSMT ...
pub struct ProveSMT {
    step: ProveStep,
    /// the current batch to prove
    current_batch: Option<BlockNumber>,
    /// the endpoint to communicate with the prover
    endpoint: ProverEndpoint,

    /// used to receive response from the endpoint
    response_receiver: Receiver<ResponseType>,
}

type BlockNumber = u64;
type StartChunk = String;
type EndChunk = String;
type RecursiveProof = String;

/// ProveStep ...
enum ProveStep {
    Start,
    // TODO: refactor to Batch
    Batch(BlockNumber),
    Aggregate(StartChunk, EndChunk),
    Final(RecursiveProof),
    End,
}

impl ProveSMT {
    pub fn new(addr: String) -> Self {
        let (response_sender, response_receiver) = mpsc::channel(10);
        ProveSMT {
            step: ProveStep::Start,
            current_batch: None,
            endpoint: ProverEndpoint::new(addr, response_sender),
            response_receiver,
        }
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // start the endpoint
        self.endpoint.launch().await
    }

    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // stop the endpoint
        Ok(())
    }

    pub async fn execute(&mut self, batch: BlockNumber) -> Result<(), Box<dyn std::error::Error>> {
        self.set_current_batch(batch).await?;

        // return proof for the batch
        self.entry_step().await?;

        self.clean_current_batch().await?;

        Ok(())
    }

    pub async fn entry_step(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            self.step = match &self.step {
                ProveStep::Start => {
                    let batch = self.current_batch.unwrap();
                    ProveStep::Batch(batch)
                }

                ProveStep::Batch(batch) => {
                    let request = ProverRequest {
                        id: "".to_string(),
                        request_type: Some(RequestType::GenBatchProof(GenBatchProofRequest {
                            id: uuid::Uuid::new_v4().to_string(),
                            batch: Some(Batch {
                                block_number: vec![*batch],
                            }),
                        })),
                    };
                    // send request to the endpoint
                    self.endpoint.send_request(request).await?;

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
                            let start_chunk = chunks.first().unwrap().clone().proof_key;
                            let end_chunk = chunks.last().unwrap().clone().proof_key;
                            ProveStep::Aggregate(start_chunk, end_chunk)
                        } else {
                            ProveStep::End
                        }
                    } else {
                        ProveStep::End
                    }
                }

                ProveStep::Aggregate(start_chunk, end_chunk) => {
                    let request = ProverRequest {
                        id: uuid::Uuid::new_v4().to_string(),
                        request_type: Some(RequestType::GenAggregatedProof(
                            GenAggregatedProofRequest {
                                recursive_proof_1: start_chunk.clone(),
                                recursive_proof_2: end_chunk.clone(),
                            },
                        )),
                    };
                    // send request to the endpoint
                    self.endpoint.send_request(request).await?;

                    // waiting for the response from the endpoint
                    if let Some(ResponseType::GenAggregatedProof(gen_aggregated_proof_response)) =
                        self.response_receiver.recv().await
                    {
                        if gen_aggregated_proof_response.result_code
                            == ProofResultCode::CompletedOk as i32
                        {
                            let recursive_proof = gen_aggregated_proof_response.result_string;
                            ProveStep::Final(recursive_proof)
                        } else {
                            ProveStep::End
                        }
                    } else {
                        ProveStep::End
                    }
                }

                ProveStep::Final(recursive_proof) => {
                    let request = ProverRequest {
                        id: uuid::Uuid::new_v4().to_string(),
                        request_type: Some(RequestType::GenFinalProof(GenFinalProofRequest {
                            recursive_proof: recursive_proof.clone(),
                            // TODO: from config or env
                            curve_name: "".to_string(),
                            // TODO: what's the aggregator address?
                            aggregator_addr: "".to_string(),
                        })),
                    };
                    self.endpoint.send_request(request).await?;

                    // waiting for the response from the endpoint
                    if let Some(ResponseType::GenFinalProof(gen_final_proof_response)) =
                        self.response_receiver.recv().await
                    {
                        if gen_final_proof_response.result_code
                            == ProofResultCode::CompletedOk as i32
                        {
                            ProveStep::End
                        } else {
                            // TODO: return error
                            log::error!(
                                "gen final proof failed, error: {:?}",
                                gen_final_proof_response.error_message
                            );
                            ProveStep::End
                        }
                    } else {
                        log::error!("gen final proof failed, no response");
                        ProveStep::End
                    }
                }

                ProveStep::End => {
                    // reset smt state
                    self.step = ProveStep::Start;
                    return Ok(());
                }
            };
        }
    }

    pub async fn set_current_batch(
        &mut self,
        batch: BlockNumber,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.current_batch = Some(batch);
        Ok(())
    }

    pub async fn clean_current_batch(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.current_batch = None;
        Ok(())
    }
}

/// ProverEndpoint used to communicate with the prover
pub struct ProverEndpoint {
    /// the address of the prover
    addr: String,
    /// used to send request to the gRPC client
    request_sender: Sender<ProverRequest>,
    /// used to receive request, and send to ProverServer
    request_receiver: Option<Receiver<ProverRequest>>,
    /// used to stop the endpoint
    stop_endpoint_tx: Sender<()>,
    /// listen to the stop signal, and stop the endpoint loop
    stop_endpoint_rx: Receiver<()>,

    /// used to send response to the ProveSMT
    response_sender: Sender<ResponseType>,
}

impl ProverEndpoint {
    pub fn new(addr: String, response_sender: Sender<ResponseType>) -> Self {
        let (request_sender, request_receiver) = mpsc::channel(10);
        let (stop_tx, stop_rx) = mpsc::channel(1);
        ProverEndpoint {
            addr,
            request_sender,
            request_receiver: Some(request_receiver),
            stop_endpoint_tx: stop_tx,
            stop_endpoint_rx: stop_rx,
            response_sender,
        }
    }

    /// send request to the gRPC Stream
    pub async fn send_request(
        &mut self,
        request: ProverRequest,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.request_sender.send(request).await?;
        Ok(())
    }

    /// launch the endpoint
    pub async fn launch(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut client = ProverServiceClient::connect(self.addr.clone()).await?;

        // take the request receiver, create a stream
        // self.request_receiver will be None after this
        let request = ReceiverStream::new(self.request_receiver.take().unwrap());

        // waiting for the first request
        let response = client.prover_stream(request).await?;
        let mut resp_stream = response.into_inner();

        loop {
            tokio::select! {
                _ = self.stop_endpoint_rx.recv() => {
                    return Ok(());
                }
                recv_msg_result = resp_stream.message() => {
                    if let Some(recv_msg) = recv_msg_result? {
                        if let Some(msg_type) = recv_msg.response_type {
                            match msg_type {
                                ResponseType::GetStatus(r) => {
                                    // TODO: Get Prover Status
                                    log::info!("GetStatusResponse: {:?}", r);
                                }
                                ResponseType::GenBatchProof(r) => {
                                    self.response_sender.send(ResponseType::GenBatchProof(r)).await?;
                                }
                                ResponseType::GenAggregatedProof(r) => {
                                    self.response_sender.send(ResponseType::GenAggregatedProof(r)).await?;
                                }
                                ResponseType::GenFinalProof(r) => {
                                    self.response_sender.send(ResponseType::GenFinalProof(r)).await?;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// stop the endpoint
    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.stop_endpoint_tx.send(()).await?;
        Ok(())
    }
}
