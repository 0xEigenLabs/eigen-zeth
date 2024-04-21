//! Initialize all components of the eigen-zeth full node.
//! They will be launched in Run CMD.
use crate::prover::ProverChannel;
use ethers_providers::{Http, Provider};

use tokio::sync::mpsc::{self, Receiver};
use tokio::time::{interval, Duration};

use crate::db::{lfs, Database};

pub(crate) struct Operator {
    db: Box<dyn Database>,
    prover: ProverChannel,
    _settler: Provider<Http>,
    rx_proof: Receiver<Vec<u8>>,
}

impl Operator {
    pub fn new(_db_path: &str, l1addr: &str, prover_addr: &str) -> Self {
        let (sx, rx) = mpsc::channel(10);
        let prover = ProverChannel::new(prover_addr, sx);
        let db = lfs::open_db(lfs::DBConfig::Memory).unwrap();
        // TODO: abstract this in the settlement
        let _settler = Provider::<Http>::try_from(l1addr).unwrap();

        Operator {
            prover,
            db,
            _settler,
            rx_proof: rx,
        }
    }

    pub async fn run(&mut self, mut stop_channel: Receiver<()>) {
        let mut ticker = interval(Duration::from_millis(1000));
        let batch_key = "next_batch".to_string().as_bytes().to_vec();
        let proof_key = "batch_proof".to_string().as_bytes().to_vec();

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    let next_batch = self.db.get(&batch_key);
                    let current_batch = self.prover.get_current_batch();
                    log::debug!("fetch block {:?}, {:?}", next_batch, current_batch);

                    match (next_batch, current_batch){
                        (None, None) => {
                            // insert the first block
                            self.db.put(batch_key.clone(), 1_u64.to_be_bytes().to_vec());
                        },
                        (Some(no), None) => {
                            let block_no = u64::from_be_bytes(no.try_into().unwrap());
                            self.prover.execute(block_no).await.unwrap();
                        },
                        (None, Some(no)) => todo!("Invalid branch, block: {no}"),
                        (Some(next), Some(cur)) => {
                            let block_no = u64::from_be_bytes(next.try_into().unwrap());
                            log::debug!("next: {block_no}, current: {cur}");
                        },
                    };
                }
                proof_data = self.rx_proof.recv() => {
                    log::debug!("fetch proof: {:?}", proof_data);
                    self.db.put(proof_key.clone(), proof_data.unwrap());

                    // trigger the next task
                    if let Some(current_batch) = self.db.get(&batch_key) {
                        let block_no = u64::from_be_bytes(current_batch.try_into().unwrap());
                        self.prover.execute(block_no).await.unwrap();
                        let block_no_next = block_no + 1;
                        self.db.put(batch_key.clone(), block_no_next.to_be_bytes().to_vec());
                    } else {
                        log::debug!("Wait for the new task coming in");
                    }
                }
                _ = stop_channel.recv() => {
                    break;
                }
            };
        }
    }
}
