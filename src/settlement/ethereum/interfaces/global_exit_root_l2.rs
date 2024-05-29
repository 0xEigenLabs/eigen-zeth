//! Rust contract client for https://github.com/0xEigenLabs/eigen-bridge-contracts/blob/feature/bridge_contract/src/EigenGlobalExitRoot.sol
use anyhow::{anyhow, Result};
use ethers::middleware::SignerMiddleware;
use ethers::prelude::LocalWallet;
use ethers_contract::abigen;
use ethers_core::types::Address;
use ethers_providers::{Http, Provider};
use std::sync::Arc;

abigen!(
    EigenGlobalExitRootL2,
    r#"[
        function updateExitRoot(bytes32 newRollupExitRoot) external
        function updateGlobalExitRootMap(bytes32 lastMainnetExitRoot) external OnlyForDeployer
    ]"#,
);

pub struct GlobalExitRootL2ContractClient {
    contract: EigenGlobalExitRootL2<SignerMiddleware<Provider<Http>, LocalWallet>>,
}

impl GlobalExitRootL2ContractClient {
    pub fn new(contract_address: Address, provider: Provider<Http>, wallet: LocalWallet) -> Self {
        let client = SignerMiddleware::new(provider, wallet);
        let contract = EigenGlobalExitRootL2::new(contract_address, Arc::new(client));
        GlobalExitRootL2ContractClient { contract }
    }

    pub async fn update_exit_root(&self, new_rollup_exit_root: [u8; 32]) -> Result<()> {
        self
            .contract
            .update_exit_root(new_rollup_exit_root)
            .send()
            .await
            .map_err(|e| anyhow!("send update exit root transaction failed, {:?}", e))?
            .inspect(|tx| log::info!("pending update exit root transaction: {:?}", **tx))
            .await
            .map_err(|e| anyhow!("execute update exit root transaction failed, {:?}", e))?
            .inspect(|tx_receipt| {
                log::info!(
                    "The update exit root transaction was successfully confirmed on the ethereum, tx receipt: {:#?}",
                    tx_receipt
                )
            });

        Ok(())
    }

    pub async fn update_global_exit_root_map(
        &self,
        last_mainnet_exit_root: [u8; 32],
    ) -> Result<()> {
        self
            .contract
            .update_global_exit_root_map(last_mainnet_exit_root)
            .send()
            .await
            .map_err(|e| anyhow!("send update global exit root map transaction failed, {:?}", e))?
            .inspect(|tx| log::info!("pending update global exit root map transaction: {:?}", **tx))
            .await
            .map_err(|e| anyhow!("execute update global exit root map transaction failed, {:?}", e))?
            .inspect(|tx_receipt| {
                log::info!(
                    "The update global exit root map transaction was successfully confirmed on the ethereum, tx receipt: {:#?}",
                    tx_receipt
                )
            });

        Ok(())
    }
}
