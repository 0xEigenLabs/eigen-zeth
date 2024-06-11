use anyhow::Result;
use ethers::middleware::SignerMiddleware;
use ethers::prelude::LocalWallet;
use ethers_contract::abigen;
use ethers_core::types::Address;
use ethers_providers::{Http, Provider};
use std::sync::Arc;

abigen!(
    EigenGlobalExitRoot,
    r#"[
        function lastRollupExitRoot() public view returns (bytes32)
    ]"#,
);

pub struct ZethGlobalExitRootContractClient {
    pub contract: EigenGlobalExitRoot<SignerMiddleware<Provider<Http>, LocalWallet>>,
}

impl ZethGlobalExitRootContractClient {
    pub fn new(contract_address: Address, provider: Provider<Http>, wallet: LocalWallet) -> Self {
        let client = SignerMiddleware::new(provider, wallet);
        let contract = EigenGlobalExitRoot::new(contract_address, Arc::new(client));
        ZethGlobalExitRootContractClient { contract }
    }

    pub async fn last_rollup_exit_root(&self) -> Result<[u8; 32]> {
        let last_global_exit_root = self.contract.last_rollup_exit_root().call().await?;
        Ok(last_global_exit_root)
    }
}
