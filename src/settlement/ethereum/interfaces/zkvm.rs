//! Rust contract client for https://github.com/0xEigenLabs/eigen-bridge-contracts/blob/feature/bridge_contract/src/EigenZkVM.sol

use anyhow::{anyhow, Result};
use ethers::core::types::U256;
use ethers::middleware::SignerMiddleware;
use ethers::prelude::LocalWallet;
use ethers_contract::abigen;
use ethers_providers::{Http, Provider};
use std::sync::Arc;

abigen!(EigenZkVM, "contracts/EigenZkvm.json");

pub struct ZkVMContractClient {
    contract: EigenZkVM<SignerMiddleware<Provider<Http>, LocalWallet>>,
}

impl ZkVMContractClient {
    pub fn new(
        contract_address: ethers::types::Address,
        provider: Provider<Http>,
        wallet: LocalWallet,
    ) -> Self {
        let client = SignerMiddleware::new(provider, wallet);
        let contract = EigenZkVM::new(contract_address, Arc::new(client));
        ZkVMContractClient { contract }
    }

    /*
    pub fn sequence_batches(
    & self,
    batches: :: std :: vec::Vec<BatchData>,
    l_2_coinbase: :: ethers :: core :: types::Address
    ) -> :: ethers :: contract :: builders::ContractCall<M, ()>
     */
    // TODO: Fixme
    #[allow(clippy::too_many_arguments)]
    pub async fn sequence_batches(
        &self,
        batches: Vec<BatchData>,
        l2_coinbase: ethers::types::Address,
    ) -> Result<()> {
        // TODO: refactor gas_limit config
        let gas_limit = ethers::prelude::U256::from(5000000);

        self.contract
            .sequence_batches(batches, l2_coinbase)
            .gas(gas_limit)
            .send()
            .await
            .map_err(|e| anyhow!("send sequence batches transaction failed, {:?}", e))?
            .inspect(|pending_tx| {
                log::info!("pending sequence batches transaction: {:?}", **pending_tx)
            })
            .await
            .map_err(|e| anyhow!("execute sequence batches transaction failed, {:?}", e))?
            .inspect(|tx_receipt| {
                log::debug!(
                    "The sequence batches transaction was successfully confirmed on the ethereum, tx receipt: {:#?}",
                    tx_receipt
                )
            });

        Ok(())
    }

    /*
    pub fn verify_batches(
    & self, pending_state_num: u64,
    init_num_batch: u64,
    final_new_batch: u64,
    new_local_exit_root: [u8; 32],
    new_state_root: [u8; 32],
    proof: Proof,
    input: [:: ethers :: core :: types::U256; 1]
    ) -> :: ethers :: contract :: builders::ContractCall<M, (
     */
    // TODO: Fixme
    #[allow(clippy::too_many_arguments)]
    pub async fn verify_batches(
        &self,
        pending_state_num: u64,
        init_num_batch: u64,
        final_new_batch: u64,
        new_local_exit_root: [u8; 32],
        new_state_root: [u8; 32],
        proof: Proof,
        input: [U256; 1],
    ) -> Result<()> {
        // TODO: refactor gas_limit config
        let gas_limit = ethers::prelude::U256::from(5000000);

        self.contract
            .verify_batches(
                pending_state_num,
                init_num_batch,
                final_new_batch,
                new_local_exit_root,
                new_state_root,
                proof,
                input,
            )
            .gas(gas_limit)
            .send()
            .await
            .map_err(|e| anyhow!("verify batches failed, {:?}", e))?
            .inspect(|pending_tx| {
                log::info!("pending verify batches transaction: {:?}", **pending_tx)
            })
            .await
            .map_err(|e| anyhow!("execute verify batches transaction failed, {:?}", e))?
            .inspect(|tx_receipt| {
                log::debug!(
                    "The verify batches transaction was successfully confirmed on the ethereum, tx receipt: {:#?}",
                    tx_receipt
                )
            });

        Ok(())
    }

    /*
    pub fn verify_batches_trusted_aggregator(
    & self, pending_state_num: u64,
    init_num_batch: u64, final_new_batch:
    u64, new_local_exit_root: [u8; 32],
    new_state_root: [u8; 32],
    proof: Proof,
    input: [:: ethers :: core :: types::U256; 1]
    ) -> :: ethers :: contract :: builders::ContractCall<M, ()>
     */
    // TODO: Fixme
    #[allow(clippy::too_many_arguments)]
    pub async fn verify_batches_trusted_aggregator(
        &self,
        pending_state_num: u64,
        init_num_batch: u64,
        final_new_batch: u64,
        new_local_exit_root: [u8; 32],
        new_state_root: [u8; 32],
        proof: Proof,
        input: [U256; 1],
    ) -> Result<()> {
        // TODO: refactor gas_limit config
        let gas_limit = ethers::prelude::U256::from(5000000);

        self.contract
            .verify_batches_trusted_aggregator(
                pending_state_num,
                init_num_batch,
                final_new_batch,
                new_local_exit_root,
                new_state_root,
                proof,
                input,
            )
            .gas(gas_limit)
            .send()
            .await
            .map_err(|e| anyhow!("verify batches trusted aggregator failed, {:?}", e))?
            .inspect(|pending_tx| {
                log::info!(
                    "pending verify batches trusted aggregator transaction: {:?}",
                    **pending_tx
                )
            })
            .await
            .map_err(|e| {
                anyhow!(
                    "execute verify batches trusted aggregator transaction failed, {:?}",
                    e
                )
            })?
            .inspect(|tx_receipt| {
                log::debug!(
                    "The verify batches trusted aggregator was successfully upload: {:#?}",
                    tx_receipt
                )
            });

        Ok(())
    }
}
