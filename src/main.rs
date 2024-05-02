use anyhow::Result;
use clap::Parser;

mod batch_proposer;
mod cli;
mod commands;
mod config;
mod custom_reth;
mod db;
mod operator;
mod prover;
mod rpc;
mod settlement;

#[tokio::main]
async fn main() -> Result<()> {
    cli::Cli::parse().run().await
}
