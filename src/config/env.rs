//! This module contains the environment variables for the EigenZeth service

use once_cell::sync::Lazy;
use std::string::ToString;

/// EigenZethEnv is a struct that holds the environment variables
pub struct GlobalEnv {
    pub l1addr: String,
    pub prover_addr: String,
    pub curve_type: String,
    pub host: String,
    pub zeth_db_path: String,
    pub chain_id: u64,
    pub program_name: String,
}

/// GLOBAL_ENV is a global variable that holds the environment variables,
/// it is lazy loaded and thread safe
pub static GLOBAL_ENV: Lazy<GlobalEnv> = Lazy::new(|| GlobalEnv {
    l1addr: std::env::var("ZETH_L2_ADDR").unwrap_or("http://localhost:8546".to_string()),
    prover_addr: std::env::var("PROVER_ADDR").unwrap_or("http://127.0.0.1:50061".to_string()),
    curve_type: std::env::var("CURVE_TYPE").unwrap_or("BN128".to_string()),
    host: std::env::var("HOST").unwrap_or("0.0.0.0:8182".to_string()),
    zeth_db_path: std::env::var("ZETH_DB_PATH")
        .unwrap_or("/tmp/chain".to_string()),
    chain_id: std::env::var("CHAIN_ID")
        .unwrap_or("12345".to_string())
        .parse::<u64>()
        .unwrap(),
    program_name: std::env::var("PROGRAM_NAME")
        .unwrap_or("EVM".to_string())
        .to_lowercase(),
});
