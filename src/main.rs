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
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();
    cli::Cli::parse().run().await
}
