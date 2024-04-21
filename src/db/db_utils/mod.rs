mod memory_kvdb;
mod libmdbx;

use std::path::PathBuf;

use db::Database as EigenDB;
use crate::db;

pub(crate) enum DBConfig {
    /// memory kv-database
    Memory,
    /// libmdbx database
    MDBX {
        /// Path to the mdbx database
        path: PathBuf,
        // path: String,
    },
}

// TODO: 


pub(crate) fn open_db(config: &DBConfig) -> Result<EigenDB, ()> {
    
}


fn open_mdbx_db(path: &PathBuf) -> Result<EigenDB, ()> {
    let db = libmdbx::Database::open(path);
    Ok(EigenDB::MDBX(db))
}

// 
// fn open_memory_db() -> Result<EigenDB, ()> {
// }