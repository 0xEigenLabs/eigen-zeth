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

/// Used to represent different tables or columns or databases
/// Which is more appropriate to use among tables, columns, and databases to represent our data?
pub(crate) mod columns {
    /// The number of columns in the DB
    pub const TOTAL_COLUMNS: usize = 2;

    /// The column for DEFAULT
    pub const DEFAULT: usize = 0;

    // TODO: others
    /// The column for DATA_AVAILABILITY
    pub const DATA_AVAILABILITY: usize = 1;
}
