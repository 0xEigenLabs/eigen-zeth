// TODO: Fix me
#![allow(dead_code)]

mod data_availability_db;
pub(crate) mod lfs;

/// TODO: we need a trait to abstract the database operations in order to support multiple databases

pub trait Database {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>>;
    fn put(&mut self, key: Vec<u8>, value: Vec<u8>);
    fn del(&mut self, key: Vec<u8>) -> Option<Vec<u8>>;
}
