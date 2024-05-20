use std::fmt;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use tokio::select;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::mpsc;

use crate::commands::reth::RethCmd;
use crate::config::env::GLOBAL_ENV;
use crate::custom_reth;
use crate::db::lfs;
use crate::operator::Operator;
use crate::settlement::ethereum::EthereumSettlementConfig;
use crate::settlement::NetworkSpec;

/// The `RunCmd` struct is a command that runs the eigen-zeth.
#[derive(clap::Args, Debug, Clone)]
#[command(version, author, about, long_about)]
pub struct RunCmd {
    #[clap(flatten)]
    pub reth_cmd: RethCmd,
    /// The log level of the node.
    #[arg(
        long,
        value_name = "LOG_LEVEL",
        verbatim_doc_comment,
        default_value_t = LogLevel::Debug,
        ignore_case = true,
    )]
    pub log_level: LogLevel,

    /// The settlement layer to use.
    #[arg(
        long,
        default_value_t = SettlementLayer::Ethereum,
        verbatim_doc_comment
    )]
    pub settlement: SettlementLayer,

    /// Path to a file containing the settlement configuration.
    #[arg(
        long,
        value_name = "FILE",
        value_hint = clap::ValueHint::FilePath,
        requires = "settlement",
        default_value = "configs/settlement.toml"
    )]
    pub settlement_conf: Option<String>,

    #[clap(flatten)]
    pub base_params: BaseParams,
}

#[derive(clap::Args, Debug, Clone, PartialEq, Eq)]
pub struct BaseParams {
    #[clap(flatten)]
    pub databases: DatabaseParams,

    /// Aggregator's EOA address, used to prove batch by the aggregator.
    #[arg(
        long,
        value_name = "EOA_ADDR",
        verbatim_doc_comment,
        default_value = "479881985774944702531460751064278034642760119942"
    )]
    pub aggregator_addr: String,
}

#[derive(clap::Args, Debug, Clone, PartialEq, Eq)]
pub struct DatabaseParams {
    /// Choose a supported database.
    #[arg(
        long,
        value_name = "DB",
        verbatim_doc_comment,
        default_value_t = Database::Mdbx,
        ignore_case = true,
    )]
    pub database: Database,

    /// Path to a file containing the database configuration.
    #[arg(
        long,
        value_name = "FILE",
        value_hint = clap::ValueHint::FilePath,
        requires = "database",
        default_value = "configs/database.toml"
    )]
    pub database_conf: String,
}

#[derive(Debug, Clone, Eq, PartialEq, clap::ValueEnum)]
pub enum Database {
    Memory,
    Mdbx,
}

impl fmt::Display for Database {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Database::Memory => write!(f, "memory"),
            Database::Mdbx => write!(f, "mdbx"),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, clap::ValueEnum)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Debug => write!(f, "debug"),
            LogLevel::Info => write!(f, "info"),
            LogLevel::Warn => write!(f, "warn"),
            LogLevel::Error => write!(f, "error"),
            LogLevel::Trace => write!(f, "trace"),
        }
    }
}

impl fmt::Display for SettlementLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SettlementLayer::Ethereum => write!(f, "ethereum"),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, clap::ValueEnum)]
#[non_exhaustive]
pub enum SettlementLayer {
    Ethereum,
}

impl RunCmd {
    pub async fn run(&self) -> Result<()> {
        // initialize the logger
        std::env::set_var("RUST_LOG", self.log_level.to_string());
        env_logger::init();
        log::info!("Initialized logger with level: {}", self.log_level);

        // Load the settlement configuration
        let settlement_spec = match self.settlement {
            SettlementLayer::Ethereum => match &self.settlement_conf {
                None => {
                    log::info!("Using Ethereum SettlementLayer");
                    return Err(anyhow::anyhow!(
                        "Settlement configuration is required for Ethereum settlement layer"
                    ));
                }
                Some(settlement_conf_path) => {
                    log::info!("Using Ethereum SettlementLayer");
                    NetworkSpec::Ethereum(EthereumSettlementConfig::from_conf_path(
                        settlement_conf_path,
                    )?)
                }
            },
        };

        // Load the database configuration
        let db_config = match self.base_params.databases.database {
            Database::Memory => {
                log::info!("Using in-memory database");
                lfs::DBConfig::Memory
            }
            Database::Mdbx => {
                log::info!("Using mdbx database");
                lfs::DBConfig::Mdbx(lfs::libmdbx::Config::from_conf_path(
                    &self.base_params.databases.database_conf,
                )?)
            }
        };

        let aggregator_addr = &self.base_params.aggregator_addr;
        log::info!(
            "Load Aggregator address: {}",
            self.base_params.aggregator_addr
        );

        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        let mut sigint = signal(SignalKind::interrupt()).unwrap();

        // initialize the signal channel
        let (stop_tx, stop_rx) = mpsc::channel::<()>(1);
        let (reth_stop_tx, reth_stop_rx) = mpsc::channel::<()>(1);

        // Handle the SIGTERM and SIGINT signals
        tokio::spawn(async move {
            #[allow(clippy::let_underscore_future)]
            #[allow(clippy::never_loop)]
            loop {
                select! {
                    _ = sigterm.recv() => {
                        println!("Recieve SIGTERM");
                        break;
                    }
                    _ = sigint.recv() => {
                        println!("Recieve SIGTERM");
                        break;
                    }
                };
            }
            stop_tx.send(()).await.unwrap();
            reth_stop_tx.send(()).await.unwrap();
        });

        // initialize the database
        let rollup_db =
            lfs::open_db(db_config).map_err(|e| anyhow!("Failed to open db: {:?}", e))?;
        let arc_rollup_db = Arc::new(rollup_db);

        let (reth_started_signal_tx, reth_started_signal_rx) = mpsc::channel::<()>(1);
        let operator_rollup_db = arc_rollup_db.clone();
        let operator = Operator::new(
            settlement_spec.clone(),
            &GLOBAL_ENV.l2addr,
            aggregator_addr.clone(),
        );
        tokio::spawn(async move {
            // Run the operator
            operator
                .run(
                    &GLOBAL_ENV.prover_addr,
                    operator_rollup_db,
                    stop_rx,
                    reth_started_signal_rx,
                )
                .await
        });

        let chain_spec = self.reth_cmd.chain.clone();
        let rpc_args = self.reth_cmd.rpc.clone();
        let dev_args = self.reth_cmd.dev;
        let data_dir = self.reth_cmd.datadir.clone();
        let reth_rollup_db = arc_rollup_db.clone();

        // Launch the custom reth
        custom_reth::launch_custom_node(
            reth_stop_rx,
            reth_started_signal_tx,
            reth_rollup_db,
            chain_spec,
            rpc_args,
            data_dir,
            dev_args,
        )
        .await
    }
}
