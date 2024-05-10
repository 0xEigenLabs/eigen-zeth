//! Rust contract client for https://github.com/0xEigenLabs/eigen-bridge-contracts/blob/feature/bridge_contract/src/EigenBridge.sol
use anyhow::{anyhow, Result};
use ethers::middleware::SignerMiddleware;
use ethers::prelude::LocalWallet;
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
        function claimMessage(bytes32[32] calldata smtProof, uint32 index, bytes32 mainnetExitRoot, bytes32 rollupExitRoot, uint32 originNetwork, address originAddress, uint32 destinationNetwork, address destinationAddress, uint256 amount, bytes calldata metadata) external
    ]"#,
);

pub struct BridgeContractClient {
    contract: EigenBridge<SignerMiddleware<Provider<Http>, LocalWallet>>,
}

impl BridgeContractClient {
    pub fn new(contract_address: Address, provider: Provider<Http>, wallet: LocalWallet) -> Self {
        let client = SignerMiddleware::new(provider, wallet);
        let contract = EigenBridge::new(contract_address, Arc::new(client));
        BridgeContractClient { contract }
    }

    // TODO: Fixme
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn bridge_asset(
        &self,
        destination_network: u32,
        destination_address: Address,
        amount: U256,
        token: Address,
        force_update_global_exit_root: bool,
        calldata: Bytes,
    ) -> Result<()> {
        self.contract
            .bridge_asset(
                destination_network,
                destination_address,
                amount,
                token,
                force_update_global_exit_root,
                calldata,
            )
            .send()
            .await
            .map_err(|e| anyhow!("send bridge asset transaction failed, {:?}", e))?
            .inspect(|pending_tx| {
                log::info!("pending bridge asset transaction: {:?}", **pending_tx)
            })
            .await
            .map_err(|e| anyhow!("execute bridge asset transaction failed, {:?}", e))?
            .inspect(|tx_receipt| {
                log::debug!(
                    "The bridge asset transaction was successfully confirmed on the ethereum, tx receipt: {:#?}",
                    tx_receipt
                )
            });

        Ok(())
    }

    pub(crate) async fn bridge_message(
        &self,
        destination_network: u32,
        destination_address: Address,
        force_update_global_exit_root: bool,
        calldata: Bytes,
    ) -> Result<()> {
        self.contract
            .bridge_message(
                destination_network,
                destination_address,
                force_update_global_exit_root,
                calldata,
            )
            .send()
            .await
            .map_err(|e| anyhow!("send bridge message transaction failed, {:?}", e))?
            .inspect(|pending_tx| {
                log::info!("pending bridge message transaction: {:?}", **pending_tx)
            })
            .await
            .map_err(|e| anyhow!("execute bridge message transaction failed, {:?}", e))?
            .inspect(|tx_receipt| {
                log::debug!(
                    "The bridge message transaction was successfully confirmed on the ethereum, tx receipt: {:#?}",
                    tx_receipt
                )
            });

        Ok(())
    }

    // TODO: Fixme
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn claim_asset(
        &self,
        smt_proof: [[u8; 32]; 32],
        index: u32,
        mainnet_exit_root: [u8; 32],
        rollup_exit_root: [u8; 32],
        origin_network: u32,
        origin_token_address: Address,
        destination_network: u32,
        destination_address: Address,
        amount: U256,
        metadata: Bytes,
    ) -> Result<()> {
        self.contract
            .claim_asset(
                smt_proof,
                index,
                mainnet_exit_root,
                rollup_exit_root,
                origin_network,
                origin_token_address,
                destination_network,
                destination_address,
                amount,
                metadata,
            )
            .send()
            .await
            .map_err(|e| anyhow!("claim asset failed, {:?}", e))?
            .inspect(|pending_tx| log::info!("pending claim asset transaction: {:?}", **pending_tx))
            .await
            .map_err(|e| anyhow!("execute claim asset transaction failed, {:?}", e))?
            .inspect(|tx_receipt| {
                log::debug!("The claim asset transaction was successfully confirmed on the ethereum, tx receipt: {:#?}", tx_receipt)
            });

        Ok(())
    }

    // TODO: Fixme
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn claim_message(
        &self,
        smt_proof: [[u8; 32]; 32],
        index: u32,
        mainnet_exit_root: [u8; 32],
        rollup_exit_root: [u8; 32],
        origin_network: u32,
        origin_address: Address,
        destination_network: u32,
        destination_address: Address,
        amount: U256,
        metadata: Bytes,
    ) -> Result<()> {
        self.contract
            .claim_message(
                smt_proof,
                index,
                mainnet_exit_root,
                rollup_exit_root,
                origin_network,
                origin_address,
                destination_network,
                destination_address,
                amount,
                metadata,
            )
            .send()
            .await
            .map_err(|e| anyhow!("claim message failed, {:?}", e))?
            .inspect(|pending_tx| {
                log::info!("pending claim message transaction: {:?}", **pending_tx)
            })
            .await
            .map_err(|e| anyhow!("execute claim message transaction failed, {:?}", e))?
            .inspect(|tx_receipt| {
                log::debug!(
                    "The claim message transaction was successfully confirmed on the ethereum, tx receipt: {:#?}",
                    tx_receipt
                )
            });

        Ok(())
    }
}

#[cfg(test)]
mod tests {}
