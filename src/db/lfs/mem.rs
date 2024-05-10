use crate::db::lfs::mem;
use crate::db::Database as EigenDB;
use std::collections::HashMap;
use std::sync::RwLock;

#[derive(Default)]
pub struct Db(RwLock<HashMap<Vec<u8>, Vec<u8>>>);

impl EigenDB for Db {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        let read_guard = self.0.read().unwrap();
        read_guard.get(key).cloned()
    }

    fn put(&self, key: Vec<u8>, value: Vec<u8>) {
        let mut write_guard = self.0.write().unwrap();

        write_guard.insert(key, value);
    }

    fn del(&self, key: Vec<u8>) -> Option<Vec<u8>> {
        let mut write_guard = self.0.write().unwrap();
        write_guard.remove(&key)
    }
}

pub fn open_memory_db() -> Result<Box<dyn EigenDB>, ()> {
    Ok(Box::new(mem::Db::default()))
}
