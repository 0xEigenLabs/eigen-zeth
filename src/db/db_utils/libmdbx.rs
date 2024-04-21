use reth_libmdbx::*;
use db::Database as EigenDB;
use crate::db;

pub struct Bd(Database);

impl EigenDB for Bd {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_open() {
        let dir = "/tmp/eigen-reth2";
        let env = Environment::builder()
            .open(std::path::Path::new(dir))
            .unwrap();
        let txn = env.begin_rw_txn().unwrap();
        let db = txn.open_db(None).unwrap();
        txn.put(db.dbi(), b"key1", b"val1", WriteFlags::empty())
            .unwrap();
        txn.put(db.dbi(), b"key2", b"val2", WriteFlags::empty())
            .unwrap();
        txn.put(db.dbi(), b"key3", b"val3", WriteFlags::empty())
            .unwrap();
        txn.commit().unwrap();

        let txn = env.begin_rw_txn().unwrap();
        let db = txn.open_db(None).unwrap();
        assert_eq!(txn.get(db.dbi(), b"key1").unwrap(), Some(*b"val1"));
        assert_eq!(txn.get(db.dbi(), b"key2").unwrap(), Some(*b"val2"));
        assert_eq!(txn.get(db.dbi(), b"key3").unwrap(), Some(*b"val3"));
        assert_eq!(txn.get::<()>(db.dbi(), b"key").unwrap(), None);

        txn.del(db.dbi(), b"key1", None).unwrap();
        assert_eq!(txn.get::<()>(db.dbi(), b"key1").unwrap(), None);
        txn.commit().unwrap();
    }

}
