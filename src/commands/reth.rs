use reth_node_core::args::utils::{chain_help, genesis_value_parser, SUPPORTED_CHAINS};
use reth_node_core::args::{DevArgs, NetworkArgs, RpcServerArgs};
use reth_node_core::dirs::{DataDirPath, MaybePlatformPath};
use reth_primitives::ChainSpec;
use std::sync::Arc;

#[derive(Debug, Clone, clap::Args)]
pub struct RethCmd {
    #[arg(long, value_name = "DATA_DIR", verbatim_doc_comment, default_value_t)]
    pub datadir: MaybePlatformPath<DataDirPath>,

    /// The chain this node is running.
    ///
    /// Possible values are either a built-in chain or the path to a chain specification file.
    #[arg(
        long,
        value_name = "CHAIN_OR_PATH",
        long_help = chain_help(),
        default_value = SUPPORTED_CHAINS[0],
        default_value_if("dev", "true", "dev"),
        value_parser = genesis_value_parser,
        required = false,
    )]
    pub chain: Arc<ChainSpec>,

    #[command(flatten)]
    pub rpc: RpcServerArgs,

    #[command(flatten)]
    pub network: NetworkArgs,

    #[command(flatten)]
    pub dev: DevArgs,
}
