use kvdb_memorydb::InMemory;

use crate::db::Database as EigenDB;

/*
pub struct InMemory {
	columns: RwLock<HashMap<u32, BTreeMap<Vec<u8>, DBValue>>>,
}
 */

pub struct Db(InMemory);

impl EigenDB for Db {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.0.get(key).unwrap()
    }

    fn put(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.0.put(key, value).unwrap()
    }

    fn del(&mut self, key: Vec<u8>) {
        self.0.del(key).unwrap()
    }

    fn commit(&mut self, transaction: db::Transaction) {
        for change in transaction.0 {
            match change {
                db::ChangeOpt::Put(key, value) => self.put(key, value),
                db::ChangeOpt::Del(key) => self.del(key),
            }
        }
    }
}