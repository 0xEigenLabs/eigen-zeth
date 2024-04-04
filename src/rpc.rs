// Reth block related imports
use reth_primitives::Block;
use reth_provider::BlockReaderIdExt;

// Rpc related imports
use jsonrpsee::proc_macros::rpc;
use reth_rpc::eth::error::EthResult;

/// trait interface for a custom rpc namespace: `EigenRpc`
///
/// This defines an additional namespace where all methods are configured as trait functions.
#[rpc(server, namespace = "eigenrpc")]
pub trait EigenRpcExtApi {
    /// Returns block 0.
    #[method(name = "customMethod")]
    fn custom_methhod(&self) -> EthResult<Option<Block>>;
    #[method(name = "getBlockByNumber")]
    fn get_block_by_number(&self, block_no: u64) -> EthResult<Option<Block>>;
}

/// The type that implements `EigenRpc` rpc namespace trait
pub struct EigenRpcExt<Provider> {
    pub provider: Provider,
}

impl<Provider> EigenRpcExtApiServer for EigenRpcExt<Provider>
where
    Provider: BlockReaderIdExt + 'static,
{
    /// Showcasing how to implement a custom rpc method
    /// using the provider.
    fn custom_methhod(&self) -> EthResult<Option<Block>> {
        let block = self.provider.block_by_number(0)?;
        // check if its confirmed on L1 and update the block's status
        Ok(block)
    }

    // TODO: override the eth_get_block_by_hash to check if the block has been confirmed by L1
    fn get_block_by_number(&self, block_no: u64) -> EthResult<Option<Block>> {
        let block = self.provider.block_by_number(block_no)?;
        Ok(block)
    }
}
