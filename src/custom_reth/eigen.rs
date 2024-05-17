use jsonrpsee::core::Serialize;
use std::sync::Arc;
// Reth block related imports
use reth_primitives::{Block, B256};
use reth_provider::BlockReaderIdExt;

// Rpc related imports
use crate::db::{prefix, Database as RollupDatabase, ProofResult, Status};
use jsonrpsee::proc_macros::rpc;
use reth_interfaces::RethError;
use reth_rpc::eth::error::{EthApiError, EthResult};
use serde::Deserialize;

/// trait interface for a custom rpc namespace: `EigenRpc`
///
/// This defines an additional namespace where all methods are configured as trait functions.
#[rpc(server, namespace = "eigenrpc")]
pub trait EigenRpcExtApi {
    /// Returns block 0.
    #[method(name = "customMethod")]
    fn custom_methhod(&self) -> EthResult<Option<Block>>;
    #[method(name = "getBlockByNumber")]
    fn get_block_by_number(&self, block_no: u64) -> EthResult<Option<BlockExt>>;
    #[method(name = "traceTransaction")]
    fn trace_transaction(&self, hash: B256) -> EthResult<Option<()>>;
    #[method(name = "getBatchProof")]
    fn get_batch_proof(&self, block_no: u64) -> EthResult<Option<BatchProofInfo>>;
}

/// The type that implements `EigenRpc` rpc namespace trait
pub struct EigenRpcExt<Provider> {
    pub provider: Provider,
    pub rollup_db: Arc<Box<dyn RollupDatabase>>,
}

impl<Provider> EigenRpcExtApiServer for EigenRpcExt<Provider>
where
    Provider: BlockReaderIdExt + 'static,
{
    /// Showcasing how to implement a custom rpc method
    /// using the provider.
    fn custom_methhod(&self) -> EthResult<Option<Block>> {
        let block = self.provider.block_by_number(0)?;
        log::info!("custom method called, block: {:?}", block);
        // check if its confirmed on L1 and update the block's status
        Ok(block)
    }

    fn get_block_by_number(&self, block_no: u64) -> EthResult<Option<BlockExt>> {
        let block = self.provider.block_by_number(block_no)?;
        if let Some(block) = block {
            let status_key = format!(
                "{}{}",
                std::str::from_utf8(prefix::PREFIX_BLOCK_STATUS)
                    .map_err(|e| EthApiError::Internal(RethError::Custom(e.to_string())))?,
                block_no
            );
            let status = match self.rollup_db.get(status_key.as_bytes()) {
                Some(status_bytes) => serde_json::from_slice(&status_bytes)
                    .map_err(|e| EthApiError::Internal(RethError::Custom(e.to_string())))?,
                None => Status::Pending,
            };
            Ok(Some(BlockExt { block, status }))
        } else {
            Ok(None)
        }
    }

    // TODO return the pre and post data for zkvm
    fn trace_transaction(&self, _hash: B256) -> EthResult<Option<()>> {
        println!("{:?}", _hash);
        //let traces = self.provider.trace
        Ok(Some(()))
    }

    fn get_batch_proof(&self, block_no: u64) -> EthResult<Option<BatchProofInfo>> {
        let next_proof_key = format!(
            "{}{}",
            std::str::from_utf8(prefix::PREFIX_BATCH_PROOF)
                .map_err(|e| EthApiError::Internal(RethError::Custom(e.to_string())))?,
            block_no
        );
        if let Some(proof_bytes) = self.rollup_db.get(next_proof_key.as_bytes()) {
            let proof: ProofResult = serde_json::from_slice(&proof_bytes)
                .map_err(|e| EthApiError::Internal(RethError::Custom(e.to_string())))?;
            let proof_info = BatchProofInfo {
                block_number: proof.block_number,
                proof: proof.proof,
                public_input: proof.public_input,
                pre_state_root: format!("0x{}", hex::encode(proof.pre_state_root)),
                post_state_root: format!("0x{}", hex::encode(proof.post_state_root)),
            };
            Ok(Some(proof_info))
        } else {
            Ok(None)
        }
    }
}

/// A custom struct that extends the standard `reth::Block` struct with a `status` field.
/// This additional field represents the status of the block within the rollup process.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlockExt {
    pub block: Block,
    pub status: Status,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct BatchProofInfo {
    pub block_number: u64,
    pub proof: String,
    pub public_input: String,
    // 0x + hex([u8; 32])
    pub pre_state_root: String,
    // 0x + hex([u8; 32])
    pub post_state_root: String,
}
