// TODO: Fix me
#![allow(dead_code)]

mod libmdbx;
mod mem;

use crate::db::Database as EigenDB;

pub(crate) enum DBConfig {
    /// memory kv-database
    Memory,
    /// libmdbx database
    Mdbx {
        /// Path to the mdbx database
        // path: PathBuf,
        path: String,
        /// Maximum number of databases
        max_dbs: usize,
    },
}

pub(crate) fn open_db(config: DBConfig) -> Result<Box<dyn EigenDB>, ()> {
    match config {
        DBConfig::Memory => mem::open_memory_db(),
        DBConfig::Mdbx { path, max_dbs } => libmdbx::open_mdbx_db(&path, max_dbs),
    }
}
