//! Rust contract client for https://github.com/0xEigenLabs/eigen-bridge-contracts/blob/feature/bridge_contract/src/EigenBridge.sol
use anyhow::Result;
use ethers_contract::abigen;
use ethers_core::types::{Address, Bytes, U256};
use ethers_providers::{Http, Provider};
use std::sync::Arc;

// example: https://github.com/gakonst/ethers-rs/blob/master/examples/contracts/examples/abigen.rs#L55
abigen!(
    EigenBridge,
    r#"[
        function bridgeAsset(uint32 destinationNetwork, address destinationAddress, uint256 amount, address token, bool forceUpdateGlobalExitRoot, bytes calldata permitData) public payable virtual nonReentrant
        function bridgeMessage(uint32 destinationNetwork, address destinationAddress, bool forceUpdateGlobalExitRoot, bytes calldata metadata) external payable
        function claimAsset(bytes32[32] calldata smtProof, uint32 index, bytes32 mainnetExitRoot, bytes32 rollupExitRoot, uint32 originNetwork, address originTokenAddress, uint32 destinationNetwork, address destinationAddress, uint256 amount, bytes calldata metadata) external
    ]"#,
);

// TODO: Fixme
#[allow(clippy::too_many_arguments)]
pub(crate) async fn bridge_asset(
    address: Address,
    client: Arc<Provider<Http>>,
    destination_network: u32,
    destination_address: Address,
    amount: U256,
    token: Address,
    force_update_global_exit_root: bool,
    calldata: Bytes,
) -> Result<()> {
    let contract = EigenBridge::new(address, client);

    if let Ok(result) = contract
        .bridge_asset(
            destination_network,
            destination_address,
            amount,
            token,
            force_update_global_exit_root,
            calldata,
        )
        .call()
        .await
    {
        log::debug!("bridge asset {result:?}");
    }
    Ok(())
}
