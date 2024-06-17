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

pub(crate) struct Operator;

impl Operator {
    pub async fn run(
        l2addr: &str,
        prover_addr: &str,
        settlement_spec: NetworkSpec,
        rollup_db: Arc<Box<dyn Database>>,
        aggregator_addr: &str,
        mut stop_rx: Receiver<()>,
        mut reth_started_signal_rx: Receiver<()>,
    ) -> Result<()> {
        // initialize all components of the eigen-zeth full node
        // initialize the prover
        let prover = ProverChannel::new(prover_addr, aggregator_addr);

        // initialize the settlement layer
        let settlement_provider = init_settlement_provider(settlement_spec)
            .map_err(|e| anyhow!("Failed to init settlement: {:?}", e))?;
        let arc_settlement_provider = Arc::new(settlement_provider);

        // wait for the reth to start
        reth_started_signal_rx
            .recv()
            .await
            .ok_or(anyhow!("RETH not started"))?;

        // initialize the L2Watcher
        // TODO: is There A Hook in reth That Can Replace This?
        log::info!("Initializing reth Provider with address: {}", l2addr);
        let l2provider = Provider::<Http>::try_from(l2addr)
            .map_err(|e| anyhow!("Failed to init l2 provider: {:?}", e))?;
        let l2provider_clone = l2provider.clone();
        let mut l2watcher = L2Watcher::new(rollup_db.clone(), l2provider_clone);

        // start all components of the eigen-zeth full node
        // start the L2Watcher
        l2watcher.start().await.unwrap();

        // start the verify worker
        let arc_db_for_verify_worker = rollup_db.clone();
        let settlement_provider_for_verify_worker = arc_settlement_provider.clone();
        let (verify_stop_tx, verify_stop_rx) = mpsc::channel::<()>(1);
        tokio::spawn(async move {
            Settler::verify_worker(
                arc_db_for_verify_worker,
                settlement_provider_for_verify_worker,
                verify_stop_rx,
            )
            .await
        });

        // start the proof worker
        let arc_db_for_proof_worker = rollup_db.clone();
        let (proof_stop_tx, proof_stop_rx) = mpsc::channel::<()>(1);
        tokio::spawn(async move {
            Settler::proof_worker(arc_db_for_proof_worker, prover, proof_stop_rx).await
        });
        
        let arc_db_for_submit_worker = rollup_db.clone();
        let settlement_provider_for_submit_worker = arc_settlement_provider.clone();
        let l2provider_for_submit_worker = l2provider.clone();
        let (submit_stop_tx, submit_stop_rx) = mpsc::channel::<()>(1);
        tokio::spawn(async move {
            Settler::rollup(
                arc_db_for_submit_worker,
                l2provider_for_submit_worker,
                settlement_provider_for_submit_worker,
                submit_stop_rx,
            )
            .await
        });

        // wait for the stop signal
        tokio::select! {
            _ = stop_rx.recv() => {
                l2watcher.stop().await.unwrap();
                verify_stop_tx.send(()).await.unwrap();
                proof_stop_tx.send(()).await.unwrap();
                submit_stop_tx.send(()).await.unwrap();
                log::info!("Operator stopped");
                Ok(())
            }
        }
    }
}
