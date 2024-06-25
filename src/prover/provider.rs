//! The Client of Eigen Proof Network
//! A standalone process is needed to finish:
//! 1) Get the Latest block that not proved, check if current node need to prove it. If so, submit
//!    the proof generation request to proof network;
//! 2) Keep polling if the task is finished.
//! 3) If the task is finished, update the status into proof database, hence the extended RPC module will fetch this and return it to SDK.

use crate::config::env::GLOBAL_ENV;
use crate::db::ProofResult;
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
use std::fmt;
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
    /// TODO: 是否需要详细信息
    endpoint_restart_signal_receiver: Receiver<()>,

    //
    // request_sender2: tokio::sync::broadcast::Sender<ProverRequest>,
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
type ChunkCount = u64;
type L2BatchData = String;

type TaskId = String;

type ErrMsg = String;

type BatchId = String;

#[derive(Debug, Clone)]
pub enum ExecuteResult {
    Success(ProofResult),
    Failed(ErrMsg),
}

/// ProveStep ...
/// 其中只有batch 有两个阶段：1. 生成chunk 2. 为chunk生成proof
/// 我们应当记录batch在每个阶段生成的中间数据，以便在失败时使用当前阶段的数据，从当前阶段继续重试，直到成功，不应跳过任何一个阶段
/// 每个阶段缓存的数据都是为后续阶段提供的，所以每个阶段执行的结果缓存到下个阶段的tuple中，如果当前阶段执行失败则使用当前阶段tuple中的数据继续执行即可
#[derive(Debug, Clone)]
enum ProveStep {
    Start,
    // TODO: refactor to Batch
    Batch(BatchStep),
    Aggregate(BatchId, StartChunk, EndChunk),
    Final(BatchId, RecursiveProof),
    End(ExecuteResult),
}

#[derive(Debug, Clone)]
enum BatchStep {
    // TODO: refactor to Batch
    GenChunk(BatchId, BlockNumber),
    GenProof(BatchId, TaskId, ChunkCount, L2BatchData),
}

impl fmt::Display for ProveStep {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ProveStep::Start => write!(f, "♥ Start"),
            // ProveStep::Batch(_batch_id, no) => write!(f, "♦ Batch: {}", no),
            ProveStep::Batch(b) => match b {
                BatchStep::GenChunk(_batch_id, no) => {
                    write!(f, "♦ Batch: GenChunk({})", no)
                }
                BatchStep::GenProof(_batch_id, task_id, chunk_count, l2_batch_data) => {
                    write!(
                        f,
                        "♦ Batch: GenChunkProof(task_id: {}, ChunkCount: {}, L2BatchData: {})",
                        task_id, chunk_count, l2_batch_data
                    )
                }
            },
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

        // 如果endpoint 出现错误, 则销毁所有endpoint相关资源并在此重新launch
        // 正常情况下, endpoint 会一直运行, 直到收到 stop 信号会返回 Ok
        // 其他情况下, 会一直尝试重新连接, 直到连接成功, 并基于记录的step状态及相关数据继续prove
        tokio::spawn(async move {
            loop {
                match endpoint.launch().await {
                    // 只有在收到 stop 信号时, 才会返回 Ok
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

    async fn entry_step(&mut self) -> Result<ProofResult> {
        loop {
            self.step = match &self.step {
                ProveStep::Start => {
                    let batch = self.current_batch.unwrap();
                    let batch_id = uuid::Uuid::new_v4().to_string();
                    ProveStep::Batch(BatchStep::GenChunk(batch_id, batch))
                }

                // ProveStep::Batch(batch_id, batch) => {
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
                                // 这里应当添加一个异常情况的channel,同时监听两个channel, 一个是正常的response, 一个是异常的response
                                // 有可能发生三种情况
                                // 1. 正常返回，进入下一个阶段，正常发送request, 等待response
                                //      a. 返回success -> 进入下一个阶段
                                //      b. 返回failed -> 重新执行当前阶段，直到成功 TODO: 如果其中一个阶段执行prove失败，应当从当前阶段重试还是从头开始(感觉应当从当前阶段重试, 前面的步骤执行成功重试没有意义，结果应该是一样的)
                                // 2. 正常返回，但是返回之后prover server挂掉了，但是这里感知不到所以会继续进入下一阶段，然后发送request,
                                //      但是该Request不会被取出，会一直等待client重新连接，然后取出该request,执行并返回response,对于这里来讲和正常流程一样，感知不到server宕机
                                //      a. 同1.a一样 返回success -> 进入下一个阶段
                                //      b. 同1.b一样 返回failed -> 重新执行当前阶段，直到成功
                                // 3. 没有返回response, 发送request过程中挂掉或者发送完成之后没有返回response就宕机，endpoint会立即重启，重启后该request丢失，
                                //      所以需要重启
                                // waiting for the response from the endpoint
                                if let Some(ResponseType::GenBatchProof(gen_batch_proof_response)) = resp {
                                        if let Some(gen_batch_proof_response::Step::GenBatchChunks(gen_batch_chunks_result)) = gen_batch_proof_response.step {
                                            if gen_batch_chunks_result.result_code == ProofResultCode::CompletedOk as i32 {

                                                let task_id = gen_batch_chunks_result.task_id;
                                                let chunk_count = gen_batch_chunks_result.chunk_count;
                                                let l2_batch_data = gen_batch_chunks_result.batch_data;

                                                ProveStep::Batch(BatchStep::GenProof(batch_id.clone(), task_id, chunk_count, l2_batch_data))

                                            } else {
                                                log::error!("gen batch chunk failed, err: {}, try again", gen_batch_chunks_result.error_message);
                                                // 重新执行当前阶段
                                                ProveStep::Batch(batch_step.clone())
                                            }
                                        } else {
                                            log::info!("gen batch chunk failed, err: invalid response, try again");
                                            ProveStep::Batch(batch_step.clone())
                                        }
                                } else {
                                    log::info!("gen batch chunk failed, err: invalid response, try again");
                                    // 重新执行当前阶段
                                    ProveStep::Batch(batch_step.clone())
                                }
                              }
                               _ = self.endpoint_restart_signal_receiver.recv() => {
                                    // TODO: endpoint restart, 重试当前阶段
                                    // request_channel在endpoint发送endpoint_restart_signal之前已经清空
                                    // 这里只需进入下一轮循环重新发送该request即可
                                    ProveStep::Batch(batch_step.clone())
                               }
                            }
                        }
                        BatchStep::GenProof(batch_id, task_id, chunk_count, l2_batch_data) => {
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
                                          ProveStep::Aggregate(batch_id.clone(), start_chunk, end_chunk)

                                       } else {
                                          log::error!("gen chunk proof failed, err: {}, try again", gen_chunk_proof_result.error_message);
                                          // 重新执行当前阶段
                                          ProveStep::Batch(batch_step.clone())
                                       }
                                    } else {
                                        log::error!("gen chunk proof failed, err: invalid response, try again");
                                        ProveStep::Batch(batch_step.clone())
                                    }
                                } else {
                                    log::error!("gen chunk proof failed, err: invalid response, try again");
                                    // 重新执行当前阶段
                                    ProveStep::Batch(batch_step.clone())
                                }
                              }
                               _ = self.endpoint_restart_signal_receiver.recv() => {
                                    // TODO: endpoint restart, 重试当前阶段
                                    // request_channel在endpoint发送endpoint_restart_signal之前已经清空
                                    // 这里只需进入下一轮循环重新发送该request即可
                                    ProveStep::Batch(batch_step.clone())
                               }
                            }
                        }
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
        // let mut client = ProverServiceClient::connect(self.addr.clone()).await?;
        // 尝试连接到 prover server, 直到连接成功
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
                // TODO: 是否需要优化这里
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

                        // Ok(None) Err(e) 时都需要根据当前状态进行后续处理
                        // 所以处理逻辑相同
                        // 状态管理逻辑应当在 ProverChannel 中实现
                        // 所以这里只需要重启即可
                        // NOTE: 重启之前需要通知 ProverChannel，以便ProverChannel根据状态进行当前阶段的重新执行
                        // 所以这里需要一个额外的字段记录一下，当前ProverChannel发送过来的请求有没有返回对应的response
                        // 1. 已经返回，直接重启
                        // 2. 没有返回则返回一个错误信号，让ProverChannel进行后续处理
                        //
                        //
                        // 如果这里添加一个rpc error channel, 此时可能发生的情况
                        // 1. 正常返回response, 等待下一个请求的发送并等待response
                        //      此时error channel为空
                        //      此时response channel为正常返回的response，ProverChannel会接受到response会进入下一个阶段并发送下一个request
                        // 2. response返回之前rpc出现错误断开连接，此时发送一个错误信号到error channel, 然后尝试重连
                        //      a. request刚发送到 request channel, 并没有被取出, 所以该request并没有丢失, 然后发生了rpc错误，断开连接
                        //          此时response channel 为空, ProverChannel不会接受到任何response, 也不会发送任何request
                        //          此时error channel 有一个错误信号，ProverChannel会接收当前错误信号，并停止继续监听response channel等待该request的相应
                        //          TODO: 这里的处理方案：清空request channel, 丢弃当前request, 然后尝试重连
                        //      b. request已经被取出，但并没有等到对应的response返回, 此时发送一个错误信号到error channel, 然后尝试重连
                        //          此时response channel 为空, ProverChannel不会接受到任何response
                        //          此时error channel 有一个错误信号，ProverChannel会接收当前错误信号，并停止继续监听response channel等待该request的相应
                        //          TODO: 这里的处理方案：清空request channel, 丢弃当前request, 然后尝试重连
                        // 3. response返回之后，下一个request发送之前rpc出现错误断开连接，此时发送一个错误信号到error channel, 然后尝试重连
                        //      这里，开始监听response channel和error channel时会立即接收到一个错误信号，然后停止继续监听response channel等待该request的相应
                        //      TODO: 这里的处理方案：清空request channel, 丢弃当前request, 然后尝试重连
                        //      原因：在并发
                        Ok(None) => {
                            // TODO: 清空request channel, 丢弃当前request, 发送错误信号到error channel，然后尝试重连
                            self.endpoint_restart_signal_sender.send(()).await?;
                        }
                        Err(_e) => {
                            // TODO: 清空request channel, 丢弃当前request, 发送错误信号到error channel，然后尝试重连
                            self.endpoint_restart_signal_sender.send(()).await?;
                        }
                    }
                }
            }
        }
    }
}
