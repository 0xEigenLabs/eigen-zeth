use crate::db::lfs::libmdbx;
use crate::db::Database as EigenDB;
use anyhow::{anyhow, Result};
use config::File;
use reth_libmdbx::*;
use serde::Deserialize;
use std::fs;
use std::ops::RangeInclusive;
use std::path::Path;

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

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub path: String,
    pub max_dbs: usize,
}

impl Config {
    pub fn from_conf_path(conf_path: &str) -> Result<Self> {
        log::info!("Load the Database config from: {}", conf_path);

        let config = config::Config::builder()
            .add_source(File::from(Path::new(conf_path)))
            .build()
            .map_err(|e| anyhow!("Failed to build config: {:?}", e))?;

        config
            .get("mdbx_config")
            .map_err(|e| anyhow!("Failed to parse Mdbx Config: {:?}", e))
    }
}

pub fn open_mdbx_db(config: Config) -> std::result::Result<Box<dyn EigenDB>, ()> {
    // create the directory if it does not exist
    // TODO: catch errors
    fs::create_dir_all(&config.path).unwrap();

    let env = match Environment::builder()
        .set_max_dbs(config.max_dbs)
        .set_geometry(Geometry::<RangeInclusive<usize>> {
            size: Some(0..=1024 * 1024 * 1024 * 1024), // Max 1TB
            ..Default::default()
        })
        .open(std::path::Path::new(&config.path))
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

    fn put(&self, key: Vec<u8>, value: Vec<u8>) {
        let txn = self.0.env.begin_rw_txn().unwrap();
        txn.put(self.0.default_db.dbi(), key, value, WriteFlags::empty())
            .unwrap();
        txn.commit().unwrap();
    }

    fn del(&self, key: Vec<u8>) -> Option<Vec<u8>> {
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
        let config = Config {
            path: path.to_string(),
            max_dbs,
        };

        let _ = open_mdbx_db(config).unwrap();
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn test_mdbx() {
        let path = "tmp/test_mdbx_db";
        let max_dbs = 20;
        let config = Config {
            path: path.to_string(),
            max_dbs,
        };
        let db = open_mdbx_db(config).unwrap();

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
        use reth_libmdbx::WriteFlags;

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
