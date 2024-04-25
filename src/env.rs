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
});
