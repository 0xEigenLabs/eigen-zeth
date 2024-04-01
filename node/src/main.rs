extern crate core;

use anyhow::Result;
use clap::Parser;
use log::info;
use zeth::{
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
    //    let Some((stark_uuid, stark_receipt)) = stark else {
    //        panic!("No STARK data to snarkify!");
    //    };

    //    if !cli.submit_to_bonsai() {
    //        panic!("Bonsai submission flag required to create a SNARK!");
    //    }

    //    /*
    //    let image_id = Digest::from(image_id);
    //    let (snark_uuid, snark_receipt) = stark2snark(image_id, stark_uuid, stark_receipt).await?;

    //    info!("Validating SNARK uuid: {}", snark_uuid);

    //    verify_groth16_snark(&cli, image_id, snark_receipt).await?;
    //    */
    //}

    Ok(())
}
