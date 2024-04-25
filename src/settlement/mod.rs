//! This is a module that contains the settlement logic for the Eigen network.
//! including the following settlement api:
//! 1. get_state: get the latest state of the settlement layer, including the state root and block number.
//! 2. update_state: update the state of the settlement layer with the given proof and public input.
// TODO: Fix me
#![allow(dead_code)]

use anyhow::Result;
use ethers_core::types::{Address, Bytes, U256};
use ethers_providers::{Http, Provider};
use std::sync::Arc;

pub(crate) mod ethereum;

// TODO: Fixme
#[allow(clippy::too_many_arguments)]
pub trait Settlement {
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
    ) -> Result<()>;

    // TODO: add more interfaces
}

pub enum NetworkSpec {
    Ethereum,
    Optimism,
}

//pub fn init_settlement(spec: NetworkSpec) -> Box<dyn Settlement> {
//    match spec {
//        NetworkSpec::Ethereum => {
//            Box::new(ethereum::EthereumSettlement{})
//        },
//        _ => todo!("Not supported network")
//    }
//}
