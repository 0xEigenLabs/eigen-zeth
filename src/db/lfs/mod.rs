// TODO: Fix me
#![allow(dead_code)]

pub(crate) mod libmdbx;
mod mem;

use crate::db::Database as EigenDB;

pub(crate) enum DBConfig {
    /// memory kv-database
    Memory,
    /// libmdbx database
    Mdbx(libmdbx::Config),
}

pub(crate) fn open_db(config: DBConfig) -> Result<Box<dyn EigenDB>, ()> {
    match config {
        DBConfig::Memory => mem::open_memory_db(),
        DBConfig::Mdbx(config) => libmdbx::open_mdbx_db(config),
    }
}
