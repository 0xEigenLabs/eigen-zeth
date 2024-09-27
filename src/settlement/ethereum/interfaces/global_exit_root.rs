//! Rust contract client for https://github.com/0xEigenLabs/eigen-bridge-contracts/blob/feature/bridge_contract/src/EigenGlobalExitRoot.sol
use anyhow::{anyhow, Result};
use ethers::middleware::SignerMiddleware;
use ethers::prelude::LocalWallet;
use ethers_contract::abigen;
use ethers_core::types::Address;
use ethers_providers::{Http, Provider};
use std::sync::Arc;

abigen!(
    EigenGlobalExitRoot,
    r#"[
        function updateExitRoot(bytes32 newRollupExitRoot) external
        function getLastGlobalExitRoot() public view returns (bytes32)
    ]"#,
);

pub struct GlobalExitRootContractClient {
    contract: EigenGlobalExitRoot<SignerMiddleware<Provider<Http>, LocalWallet>>,
}

impl GlobalExitRootContractClient {
    pub fn new(contract_address: Address, provider: Provider<Http>, wallet: LocalWallet) -> Self {
        let client = SignerMiddleware::new(provider, wallet);
        let contract = EigenGlobalExitRoot::new(contract_address, Arc::new(client));
        GlobalExitRootContractClient { contract }
    }

    pub async fn update_exit_root(
        &self,
        _network: u32,
        new_rollup_exit_root: [u8; 32],
    ) -> Result<()> {
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

    pub async fn get_last_global_exit_root(&self) -> Result<[u8; 32]> {
        let last_global_exit_root = self.contract.get_last_global_exit_root().call().await?;
        Ok(last_global_exit_root)
    }
}
