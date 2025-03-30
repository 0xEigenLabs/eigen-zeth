use crate::custom_reth::MyCustomNode;
use reth::chainspec::ChainSpecBuilder;
use reth_db::DatabaseEnv;
use reth::builder::{NodeTypes, NodeTypesWithDBAdapter};
use reth::providers::providers::StaticFileProvider;
use anyhow::anyhow;
use reth::providers::ProviderFactory;
use anyhow::Result;
use jsonrpsee::tracing::info;
use reth_db::init_db;
use reth_db::mdbx::DatabaseArguments;
use reth_node_core::dirs::{DataDirPath, MaybePlatformPath};
use reth::chainspec::ChainSpec;
use std::sync::Arc;
use reth_db_common::init::init_genesis;
use reth_node_core::args::{DatadirArgs, LogArgs};
use std::fmt;

use reth_ethereum_cli::chainspec::EthereumChainSpecParser;
use reth_cli::chainspec::{ChainSpecParser};
use clap::Args;

#[derive(Args, Clone, Debug)]
pub struct NoArgs;

#[derive(Debug, Clone, Args)]
pub struct InitCmd<
    //C: ChainSpecParser = EthereumChainSpecParser,
    //Ext: Args + fmt::Debug = NoArgs,
> {
    #[arg(long, value_name = "DATA_DIR", verbatim_doc_comment, default_value_t)]
    datadir: DatadirArgs,

    /// The chain this node is running.
    ///
    /// Possible values are either a built-in chain or the path to a chain specification file.
    #[arg(
        long,
        value_name = "CHAIN_OR_PATH",
        long_help = C::help_message(),
        default_value = C::SUPPORTED_CHAINS[0],
        value_parser = C::parser() 
    )]
    chain: Arc<ChainSpec>,
}

impl InitCmd {
    pub async fn run(&self) -> Result<()> {
        info!(target: "zeth::cli", "zeth's layer2 chain init starting");

        // add network name to data dir
        let data_dir = self.datadir.clone().resolve_datadir(self.chain.chain());
        let db_path = data_dir.db();
        info!(target: "zeth::cli", path = ?db_path, "Opening database");
        let db_arguments = DatabaseArguments::default();
        let db = Arc::new(
            init_db(db_path.clone(), db_arguments)
                .map_err(|e| anyhow!(e))?
                .with_metrics(),
        );
        info!(target: "zeth::cli", "Database opened");

        let spec = Arc::new(ChainSpecBuilder::mainnet().build());
        let provider_factory =
            ProviderFactory::<NodeTypesWithDBAdapter<MyCustomNode, Arc<DatabaseEnv>>>::new(db, spec.clone(),
                                 StaticFileProvider::read_write(self.datadir.static_files_path.as_ref().unwrap())?);
        info!(target: "zeth::cli", "Writing genesis block");

        let hash = init_genesis(&provider_factory)?;

        info!(target: "zeth::cli", hash = ?hash, "Genesis block written");

        Ok(())
    }
}
