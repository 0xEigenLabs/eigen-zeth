use crate::db::lfs::mem;
use crate::db::Database as EigenDB;
use std::collections::HashMap;

#[derive(Default)]
pub struct Db(HashMap<Vec<u8>, Vec<u8>>);

impl EigenDB for Db {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.0.get(key).cloned()
    }

    fn put(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.0.insert(key, value);
    }

    fn del(&mut self, key: Vec<u8>) -> Option<Vec<u8>> {
        self.0.remove(&key)
    }
}

pub fn open_memory_db() -> Result<Box<dyn EigenDB>, ()> {
    Ok(Box::new(mem::Db::default()))
}
