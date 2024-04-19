//! Aggregate the proof
//!

use reth_libmdbx::*;
use std::{
    borrow::Cow,
    io::Write,
    sync::{Arc, Barrier},
    thread::{self, JoinHandle},
};
use tokio::select;
use tokio::signal::unix::signal;
use tokio::signal::unix::SignalKind;
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    oneshot, watch,
};
use tokio::task;
use tokio::time::{interval, sleep, Duration, Instant};

pub(crate) struct Aggregator {
    //txn: Transaction<RW>,
    //db: Database,
}

async fn fetch_block(sx: &mut Sender<String>) {
    // async work
    // send batch id
    println!("Send");
    sx.send("111".to_string());
}

async fn update_proof_status(proof_data: Option<Vec<u8>>) {
    // more here
    println!("update_proof_status: {:?}", proof_data);
}

impl Aggregator {
    pub fn new(dir: &str) -> Self {
        //let env = Environment::builder().open(std::path::Path::new(dir)).unwrap();
        //let txn = env.begin_rw_txn().unwrap();
        //let db = txn.open_db(Some("aggregator")).unwrap();

        //Aggregator{ txn, db}
        Aggregator {}
    }

    pub async fn start(&mut self, mut stop_channel: Receiver<()>) {
        // pick suitable queue sizes for these channels
        let (mut block_sender, mut block_receiver) = mpsc::channel(16);
        let (mut proof_sender, mut proof_receiver) = mpsc::channel::<Vec<u8>>(16);
        // TODO: init prover client and pass the block_receiver and proof_sender.

        let mut ticker = interval(Duration::from_millis(100));

        let forever = task::spawn(async move {
            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        fetch_block(&mut block_sender);
                    }
                    proof_data = proof_receiver.recv() => {
                        update_proof_status(proof_data);
                    }
                    _ = stop_channel.recv() => {
                        break;
                    }
                };
            }
        });

        forever.await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_start() {
        let mut agg = Aggregator::new("/tmp/eigen-reth");

        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        let mut sigint = signal(SignalKind::interrupt()).unwrap();

        let (stop_tx, mut stop_rx) = mpsc::channel::<()>(1);

        tokio::spawn(async move {
            let mut sigterm = signal(SignalKind::terminate()).unwrap();
            let mut sigint = signal(SignalKind::interrupt()).unwrap();
            loop {
                select! {
                    _ = sigterm.recv() => println!("Recieve SIGTERM"),
                    _ = sigint.recv() => println!("Recieve SIGTERM"),
                };
                stop_tx.send(()).await.unwrap();
            }
        });

        agg.start(stop_rx).await;
    }

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
