mod data_availability_db;
mod db_utils;

/// TODO: we need a trait to abstract the database operations in order to support multiple databases?

pub enum ChangeOpt {
    Put(Vec<u8>, Vec<u8>),
    Del(Vec<u8>),
}

pub struct Transaction(pub Vec<ChangeOpt>);

impl Transaction {
pub fn new() -> Self {
        Transaction(Vec::new())
    }

    pub fn put(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.0.push(ChangeOpt::Put(key, value));
    }

    pub fn del(&mut self, key: Vec<u8>) {
        self.0.push(ChangeOpt::Del(key));
    }
}

pub trait Database {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>>;
    fn put(&mut self, key: Vec<u8>, value: Vec<u8>);
    fn del(&mut self, key: Vec<u8>);
    fn commit(&mut self, transaction: Transaction);
}
/// key prefixes for different databases
pub(crate) mod columns{
    pub const NUM_COLUMNS: u8 = 0;
}