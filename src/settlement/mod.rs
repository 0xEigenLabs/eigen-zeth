//! This is a module that contains the settlement logic for the Eigen network.
//! including the following settlement api:
//! 1. get_state: get the latest state of the settlement layer, including the state root and block number.
//! 2. update_state: update the state of the settlement layer with the given proof and public input.
// TODO: Fix me
#![allow(dead_code)]
use anyhow::Result;
use async_trait::async_trait;
use ethers_core::types::{Address, Bytes, U256};

pub(crate) mod ethereum;

// TODO: Fixme
#[allow(clippy::too_many_arguments)]
#[async_trait]
pub trait Settlement {
    // bridge

    async fn bridge_asset(
        &self,
        destination_network: u32,
        destination_address: Address,
        amount: U256,
        token: Address,
        force_update_global_exit_root: bool,
        calldata: Bytes,
    ) -> Result<()>;

    async fn bridge_message(
        &self,
        destination_network: u32,
        destination_address: Address,
        force_update_global_exit_root: bool,
        calldata: Bytes,
    ) -> Result<()>;

    async fn claim_asset(
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
    ) -> Result<()>;

    async fn claim_message(
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
    ) -> Result<()>;

    // global_exit_root

    async fn update_global_exit_root(&self, new_root: [u8; 32]) -> Result<()>;

    async fn get_global_exit_root(&self) -> Result<[u8; 32]>;

    // TODO: add more interfaces
}

pub enum NetworkSpec {
    Ethereum(ethereum::EthereumSettlementConfig),
    Optimism,
}

pub fn init_settlement(spec: NetworkSpec) -> Box<dyn Settlement> {
    match spec {
        NetworkSpec::Ethereum(config) => Box::new(ethereum::EthereumSettlement::new(config)),
        _ => todo!("Not supported network"),
    }
}
