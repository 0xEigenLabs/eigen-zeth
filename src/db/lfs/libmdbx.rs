// TODO: Fix me
#![allow(dead_code)]
#![allow(unused_imports)]

use crate::db::lfs::libmdbx;
use crate::db::Database as EigenDB;
use reth_libmdbx::*;
use std::fs;

pub struct Db(MdbxDB);

impl Db {
    pub fn new(env: Environment, default_db: Database) -> Self {
        Db(MdbxDB { env, default_db })
    }
}

pub struct MdbxDB {
    env: Environment,
    default_db: Database,
}

pub fn open_mdbx_db(path: &str, max_dbs: usize) -> std::result::Result<Box<dyn EigenDB>, ()> {
    // create the directory if it does not exist
    // TODO: catch errors
    fs::create_dir_all(path).unwrap();

    let env = match Environment::builder()
        .set_max_dbs(max_dbs)
        .open(std::path::Path::new(path))
    {
        Ok(env) => env,
        Err(e) => {
            println!("Error opening the environment: {:?}", e);
            return Err(());
        }
    };

    let txn_open_default_db = env.begin_rw_txn().unwrap();
    let default_db = match txn_open_default_db.create_db(None, reth_libmdbx::DatabaseFlags::empty())
    {
        Ok(db) => db,
        Err(e) => {
            println!("Error creating the default database: {:?}", e);
            return Err(());
        }
    };

    // TODO: catch errors
    txn_open_default_db.commit().unwrap();

    Ok(Box::new(libmdbx::Db::new(env, default_db)))
}

impl EigenDB for Db {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        let txn = self.0.env.begin_ro_txn().unwrap();
        let value: Option<Vec<u8>> = txn.get(self.0.default_db.dbi(), key).unwrap();
        value
    }

    fn put(&mut self, key: Vec<u8>, value: Vec<u8>) {
        let txn = self.0.env.begin_rw_txn().unwrap();
        txn.put(self.0.default_db.dbi(), key, value, WriteFlags::empty())
            .unwrap();
        txn.commit().unwrap();
    }

    fn del(&mut self, key: Vec<u8>) -> Option<Vec<u8>> {
        let txn = self.0.env.begin_rw_txn().unwrap();
        let value: Option<Vec<u8>> = txn.get(self.0.default_db.dbi(), &key).unwrap();
        let success = txn.del(self.0.default_db.dbi(), &key, None).unwrap();
        // TODO: catch errors
        txn.commit().unwrap();
        if !success {
            return None;
        }
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, string};

    #[test]
    fn test_open_mdbx_db() {
        let path = "tmp/test_open_mdbx_db";
        let max_dbs = 20;
        let _ = open_mdbx_db(path, max_dbs).unwrap();
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn test_eigen_mdbx() {
        let path = "tmp/test_eigen_mdbx_dn";
        let max_dbs = 20;
        let mut db = open_mdbx_db(path, max_dbs).unwrap();

        let key = b"key";
        // let key = string::String::from("value").into_bytes();
        let value = string::String::from(
            "value-11111111111111111111111111111111111111111111111111111111111111111111111111",
        )
        .into_bytes();

        // get a key-value pair from the default database
        // will return None because the key does not exist
        let get_val = db.get(key);
        assert_eq!(None, get_val);

        // put a key-value pair in the default database
        // get a key-value pair from the default database again
        // and check if the value is the same as the one we put
        db.put(key.to_vec(), value.clone());
        let get_val = db.get(key);
        assert_eq!(Some(value.clone()), get_val);

        // del a key-value pair from the default database
        // get a key-value pair from the default database again
        // will return None because the key was deleted
        db.del(key.to_vec());
        let get_val = db.get(key);
        assert_eq!(None, get_val);

        // remove test directory
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn test_reth_libmdbx() {
        use reth_libmdbx::{EnvironmentBuilder, WriteFlags};

        // path to the database
        let path = "tmp/test_mdbx";
        // create the directory if it does not exist
        fs::create_dir_all(path).unwrap();

        // initialize the environment
        let env = match Environment::builder()
            .set_max_dbs(20)
            .open(std::path::Path::new(path))
        {
            Ok(env) => env,
            Err(e) => {
                println!("Error opening the environment: {:?}", e);
                return;
            }
        };

        let txn_prepare = env.begin_rw_txn().unwrap();

        // use the default database
        let default_db = match txn_prepare.create_db(None, DatabaseFlags::empty()) {
            Ok(db) => db,
            Err(e) => {
                println!("Error creating the default database: {:?}", e);
                return;
            }
        };

        // create and use a new database
        let db1 = match txn_prepare.create_db(Some("my_database_1"), DatabaseFlags::empty()) {
            Ok(db) => db,
            Err(e) => {
                println!("Error creating the database: {:?}", e);
                return;
            }
        };

        txn_prepare.commit().unwrap();

        let key = b"key";
        // let key = string::String::from("value").into_bytes();
        let value = string::String::from(
            "value-11111111111111111111111111111111111111111111111111111111111111111111111111",
        )
        .into_bytes();

        // put and get a key-value pair from following databases: default, my_database_1
        let txn = env.begin_rw_txn().unwrap();

        // put and get a key-value pair in the default database
        txn.put(default_db.dbi(), key, value.clone(), WriteFlags::empty())
            .unwrap();
        let fetched_value_1: Option<Vec<u8>> = txn.get(default_db.dbi(), key).unwrap();
        assert_eq!(
            Some(
                b"value-11111111111111111111111111111111111111111111111111111111111111111111111111"
                    .to_vec()
            ),
            fetched_value_1
        );

        // put and get a key-value pair in the new database
        txn.put(db1.dbi(), key, value.clone(), WriteFlags::empty())
            .unwrap();
        let fetched_value_2: Option<Vec<u8>> = txn.get(db1.dbi(), key).unwrap();
        assert_eq!(
            Some(
                b"value-11111111111111111111111111111111111111111111111111111111111111111111111111"
                    .to_vec()
            ),
            fetched_value_2
        );

        txn.commit().unwrap();

        // del and get a key-value pair from the following databases: default, my_database_1
        let txn2 = env.begin_rw_txn().unwrap();

        txn2.del(default_db.dbi(), key, None).unwrap();
        let fetched_value_1_when_del: Option<Vec<u8>> = txn2.get(default_db.dbi(), key).unwrap();
        assert_eq!(None, fetched_value_1_when_del);

        txn2.del(db1.dbi(), key, None).unwrap();
        let fetched_value_2_when_del: Option<Vec<u8>> = txn2.get(db1.dbi(), key).unwrap();
        assert_eq!(None, fetched_value_2_when_del);

        txn2.commit().unwrap();

        // remove test directory
        fs::remove_dir_all(path).unwrap();
    }
}
