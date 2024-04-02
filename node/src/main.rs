extern crate core;

use anyhow::Result;
use clap::Parser;
use log::info;
use zeth_node::{
    cli::{Cli, Network},
    operations::{build},
};
use zeth_lib::{
    builder::EthereumStrategy,
    consts::ETH_MAINNET_CHAIN_SPEC,
};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    // execute the command
    let build_args = cli.build_args();
    let (_, stark) = match build_args.network {
        Network::Ethereum => {
            let rpc_url = build_args.eth_rpc_url.clone();
            (
                0,
                build::build_block::<EthereumStrategy>(
                    &cli,
                    rpc_url,
                    &ETH_MAINNET_CHAIN_SPEC,
                    &[], //TODO: ELF
                )
                .await?,
            )
        }
        _ => todo!(),
    };

    // Create/verify Groth16 SNARK
    //if cli.snark() {
    //}

    Ok(())
}
