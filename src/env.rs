//! This module contains the environment variables for the EigenZeth service

use once_cell::sync::Lazy;
use std::string::ToString;

/// EigenZethEnv is a struct that holds the environment variables
pub struct EigenZethEnv {
    pub db_path: String,
    pub l1addr: String,
    pub prover_addr: String,
    pub curve_type: String,
    pub host: String,
    pub zeth_db_path: String,
    pub chain_id: u64,
    pub program_name: String,
    pub settlement: SettlementEnv,
}

#[derive(Clone)]
pub struct SettlementEnv {
    pub eth_env: EthereumEnv,
}

#[derive(Clone)]
pub struct EthereumEnv {
    pub provider_url: String,
    pub local_wallet: LocalWalletConfig,
    pub l1_contracts_addr: EthContractsAddr,
}

#[derive(Clone)]
pub struct LocalWalletConfig {
    pub private_key: String,
    pub chain_id: u64,
}

// TODO: Fixme
#[allow(dead_code)]
#[derive(Clone)]
pub struct EthContractsAddr {
    pub eigen_bridge: String,
    pub eigen_global_exit: String,
    pub eigen_zkvm: String,
}

/// EIGEN_ZETH_ENV is a global variable that holds the environment variables,
/// it is lazy loaded and thread safe
pub static GLOBAL_ENV: Lazy<EigenZethEnv> = Lazy::new(|| EigenZethEnv {
    db_path: std::env::var("ZETH_OPERATOR_DB").unwrap_or("/tmp/operator".to_string()),
    l1addr: std::env::var("ZETH_L2_ADDR").unwrap_or("http://localhost:8546".to_string()),
    prover_addr: std::env::var("PROVER_ADDR").unwrap_or("http://127.0.0.1:50061".to_string()),
    curve_type: std::env::var("CURVE_TYPE").unwrap_or("BN128".to_string()),
    host: std::env::var("HOST").unwrap_or("0.0.0.0:8182".to_string()),
    zeth_db_path: std::env::var("ZETH_DB_PATH").unwrap_or("/tmp/chain".to_string()),
    chain_id: std::env::var("CHAIN_ID")
        .unwrap_or("12345".to_string())
        .parse::<u64>()
        .unwrap(),
    program_name: std::env::var("PROGRAM_NAME")
        .unwrap_or("EVM".to_string())
        .to_lowercase(),
    // TODO: too many env variables, I will move this to a config file,and use clap command to receive the path to the config file
    settlement: SettlementEnv {
        eth_env: EthereumEnv {
            provider_url: std::env::var("ETH_PROVIDER_URL")
                .unwrap_or("http://localhost:8546".to_string()),
            local_wallet: LocalWalletConfig {
                private_key: std::env::var("PRIVATE_KEY").unwrap_or("".to_string()),
                chain_id: std::env::var("CHAIN_ID")
                    .unwrap_or("12345".to_string())
                    .parse()
                    .unwrap(),
            },
            l1_contracts_addr: EthContractsAddr {
                eigen_bridge: std::env::var("EIGEN_BRIDGE_CONTRACT_ADDR")
                    .unwrap_or("0x".to_string()),
                eigen_global_exit: std::env::var("EIGEN_GLOBAL_EXIT_CONTRACT_ADDR")
                    .unwrap_or("0x".to_string()),
                eigen_zkvm: std::env::var("EIGEN_ZKVM_CONTRACT_ADDR").unwrap_or("0x".to_string()),
            },
        },
    },
});
