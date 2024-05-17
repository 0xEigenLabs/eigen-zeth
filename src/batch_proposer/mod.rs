use crate::db::Database;
use crate::db::{keys, prefix, Status};
use anyhow::{anyhow, bail, Result};
use ethers_providers::{Http, Middleware, Provider};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::{select, time};

const L2_WATCHER_INTERVAL: Duration = Duration::from_secs(1);
pub struct L2Watcher {
    pub db: Arc<Box<dyn Database>>,
    pub l2provider: Provider<Http>,
    // stop_rx: broadcast::Receiver<()>,
    stop_tx: broadcast::Sender<()>,
    // pub ticker: time::Interval,
}

impl L2Watcher {
    pub fn new(db: Arc<Box<dyn Database>>, l2provider: Provider<Http>) -> Self {
        let (stop_tx, _) = broadcast::channel(1);

        L2Watcher {
            db,
            l2provider,
            // stop_rx,
            stop_tx,
            // ticker: time::interval(L2_WATCHER_INTERVAL),
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        // TODO: restart
        let mut ticker = time::interval(L2_WATCHER_INTERVAL);
        let mut stop_rx = self.stop_tx.subscribe();
        let db = self.db.clone();
        let l2provider = self.l2provider.clone();
        let mut stop_fetch_tx = self.stop_tx.subscribe();

        tokio::spawn(async move {
            loop {
                select! {
                    _ = stop_rx.recv() => {
                        log::info!("L2Watcher stopped");
                        break;
                    }
                    _ = ticker.tick() => {
                       if let Err(e) = Self::fetch_latest_block(&db, &l2provider, &mut stop_fetch_tx).await{
                           log::error!("L2Watcher failed to fetch latest block, try again later, err: {:?}", e);
                       }
                    }
                }
            }
        });

        log::info!("L2Watcher started");
        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        self.stop_tx
            .send(())
            .map_err(|e| anyhow!("Failed to stop the L2Watcher: {:?}", e))
            .map(|_| log::info!("Exit signal successfully sent"))
    }

    pub async fn fetch_latest_block(
        db: &Arc<Box<dyn Database>>,
        l2provider: &Provider<Http>,
        stop_rx: &mut broadcast::Receiver<()>,
    ) -> Result<()> {
        select! {
            result = l2provider.get_block_number() => {
                result
                .map_err(|e| anyhow!("{:?}", e))
                .map(|number| {
                    log::info!("L2Watcher fetched block({})", number);

                    let last_fetched_block = match db.get(keys::KEY_LAST_SEQUENCE_FINALITY_BLOCK_NUMBER) {
                        None => {
                            db.put(keys::KEY_LAST_SEQUENCE_FINALITY_BLOCK_NUMBER.to_vec(), number.as_u64().to_be_bytes().to_vec());
                            0
                        }
                        Some(block_number_bytes) => {
                            u64::from_be_bytes(block_number_bytes.try_into().unwrap())
                        }
                    };

                    db.put(keys::KEY_LAST_SEQUENCE_FINALITY_BLOCK_NUMBER.to_vec(), number.as_u64().to_be_bytes().to_vec());
                    for number in last_fetched_block+1..number.as_u64()+1 {
                        // update block status to sequenced
                        let status_key = format!("{}{}", std::str::from_utf8(prefix::PREFIX_BLOCK_STATUS).unwrap(), number);
                        let status = Status::Sequenced;
                        let encoded_status = serde_json::to_vec(&status).unwrap();
                        db.put(status_key.as_bytes().to_vec(), encoded_status);

                    }
                })
            },
            _ = stop_rx.recv() => {
                log::info!("Early exit signal received during block fetch.");
                bail!("Early exit signal received during block fetch.");
            }
        }
    }
}
