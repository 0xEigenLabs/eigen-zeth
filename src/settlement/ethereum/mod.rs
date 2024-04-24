use crate::settlement::Settlement;
pub(crate) mod interfaces;
use anyhow::Result;
use ethers_core::types::{Address, Bytes, U256};
use ethers_providers::{Http, Provider};
use std::sync::Arc;

pub struct EthereumSettlement {}

impl Settlement for EthereumSettlement {
    async fn bridge_asset(
        &self,
        address: Address,
        client: Arc<Provider<Http>>,
        destination_network: u32,
        destination_address: Address,
        amount: U256,
        token: Address,
        force_update_global_exit_root: bool,
        calldata: Bytes,
    ) -> Result<()> {
        interfaces::eigen_bridge::bridge_asset(
            address,
            client,
            destination_network,
            destination_address,
            amount,
            token,
            force_update_global_exit_root,
            calldata,
        ).await
    }
}
