//! Initialize all components of the eigen-zeth full node.
//! They will be launched in Run CMD.

use crate::batch_proposer::L2Watcher;
use crate::prover::ProverChannel;
use crate::settlement::{init_settlement_provider, NetworkSpec};
use anyhow::{anyhow, Result};
// use ethers_core::types::{Bytes, H160, U256};
use ethers_providers::{Http, Provider};
// use serde::Serialize;
use crate::db::Database;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Receiver;

use crate::settlement::worker::Settler;
use crate::settlement::Settlement;

pub(crate) struct Operator {
    l1_settler: Arc<Box<dyn Settlement>>,
    l2_provider: Provider<Http>,
    aggregator_addr: String,
}

impl Operator {
    pub fn new(settlement_spec: NetworkSpec, l2addr: &str, aggregator_addr: String) -> Self {
        let settlement_provider = init_settlement_provider(settlement_spec)
            .unwrap_or_else(|e| panic!("Failed to init settlement, {:?}", e));

        // TODO: is There A Hook in reth That Can Replace This?
        log::info!("Initializing reth Provider with address: {}", l2addr);
        let l2provider = Provider::<Http>::try_from(l2addr)
            .unwrap_or_else(|e| panic!("Failed to init l2 provider: {:?}", e));
        Self {
            l1_settler: Arc::new(settlement_provider),
            l2_provider: l2provider,
            aggregator_addr,
        }
    }

    pub async fn run(
        &self,
        prover_addr: &str,
        rollup_db: Arc<Box<dyn Database>>,
        mut stop_rx: Receiver<()>,
        mut reth_started_signal_rx: Receiver<()>,
    ) -> Result<()> {
        // initialize all components of the eigen-zeth full node
        // initialize the prover
        let prover = ProverChannel::new(prover_addr, &self.aggregator_addr);

        // wait for the reth to start
        reth_started_signal_rx
            .recv()
            .await
            .ok_or(anyhow!("RETH not started"))?;

        // initialize the L2Watcher
        // TODO: is There A Hook in reth That Can Replace This?
        let mut l2watcher = L2Watcher::new(rollup_db.clone(), self.l2_provider.clone());

        // start all components of the eigen-zeth full node
        // start the L2Watcher
        l2watcher.start().await.unwrap();

        // start the verify worker
        let arc_db_for_verify_worker = rollup_db.clone();
        let (verify_stop_tx, verify_stop_rx) = mpsc::channel::<()>(1);
        let arc_provider = self.l1_settler.clone();
        tokio::spawn(async move {
            Settler::verify_worker(arc_db_for_verify_worker, arc_provider, verify_stop_rx).await
        });

        // start the proof worker
        let arc_db_for_proof_worker = rollup_db.clone();
        let (proof_stop_tx, proof_stop_rx) = mpsc::channel::<()>(1);
        tokio::spawn(async move {
            Settler::proof_worker(arc_db_for_proof_worker, prover, proof_stop_rx).await
        });

        // wait for the stop signal
        tokio::select! {
            _ = stop_rx.recv() => {
                l2watcher.stop().await.unwrap();
                verify_stop_tx.send(()).await.unwrap();
                proof_stop_tx.send(()).await.unwrap();
                log::info!("Operator stopped");
                Ok(())
            }
        }
    }

    // https://github.com/0xEigenLabs/eigen-bridge-contracts/blob/feature/bridge_contract/src/EigenGlobalExitRootL2.sol#L46
    pub async fn create_emt_sync_tx(&self) -> Result<()> {
        let emt_root = self.l1_settler.get_global_exit_root().await?;
        // TODO update the L2 emt root
        Ok(())
    }
}
