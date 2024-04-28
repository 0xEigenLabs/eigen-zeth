//! This module contains the environment variables for the EigenZeth service

use once_cell::sync::Lazy;

/// EigenZethEnv is a struct that holds the environment variables
pub struct EigenZethEnv {
    pub db_path: String,
    pub l1addr: String,
    pub prover_addr: String,
    pub curve_type: String,
    pub host: String,
    pub chain_id: u64,
    pub chain_vm_type: String,
}

/// EIGEN_ZETH_ENV is a global variable that holds the environment variables,
/// it is lazy loaded and thread safe
pub static GLOBAL_ENV: Lazy<EigenZethEnv> = Lazy::new(|| EigenZethEnv {
    db_path: std::env::var("ZETH_OPERATOR_DB").unwrap(),
    l1addr: std::env::var("ZETH_L2_ADDR").unwrap(),
    prover_addr: std::env::var("PROVER_ADDR").unwrap_or("http://127.0.0.1:50061".to_string()),
    curve_type: std::env::var("CURVE_TYPE").unwrap_or("BN128".to_string()),
    host: std::env::var("HOST").unwrap_or(":8545".to_string()),
    chain_id: std::env::var("CHAIN_ID")
        .unwrap_or("12345".to_string())
        .parse::<u64>()
        .unwrap(),
    chain_vm_type: std::env::var("CHAIN_VM_TYPE")
        .unwrap_or("EVM".to_string())
        .to_lowercase(),
});
