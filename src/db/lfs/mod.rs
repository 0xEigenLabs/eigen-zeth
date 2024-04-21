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
    },
}

pub(crate) fn open_db(config: DBConfig) -> Result<Box<dyn EigenDB>, ()> {
    match config {
        DBConfig::Memory => open_memory_db(),
        DBConfig::Mdbx { .. } => {
            unimplemented!("open_mdbx_db")
        }
    }
}

fn open_memory_db() -> Result<Box<dyn EigenDB>, ()> {
    Ok(Box::new(mem::Db::default()))
}
