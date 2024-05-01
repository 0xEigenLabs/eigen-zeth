use crate::config::env::GLOBAL_ENV;
use crate::custom_reth;
use crate::operator;
use crate::settlement::ethereum::EthereumSettlementConfig;
use crate::settlement::NetworkSpec;
use anyhow::Result;
use std::fmt;
use std::str::FromStr;
use tokio::select;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::mpsc;

/// The `RunCmd` struct is a command that runs the eigen-zeth.
#[derive(clap::Parser, Debug, Clone, PartialEq, Eq)]
pub struct RunCmd {
    /// The log level of the node. Default is `debug`. Possible values are `Trace`, `debug`, `info`, `warn`, `error`.
    #[arg(
        long,
        value_name = "LOG_LEVEL",
        verbatim_doc_comment,
        default_value_t = LogLevel::Debug,
        value_parser = LogLevel::from_str
    )]
    pub log_level: LogLevel,

    #[arg(
        long,
        default_value_t = SettlementLayer::Ethereum,
        verbatim_doc_comment
    )]
    pub settlement: SettlementLayer,

    #[arg(
        long,
        value_hint = clap::ValueHint::FilePath,
        requires = "settlement",
        default_value = "configs/settlement.toml"
    )]
    pub settlement_conf: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, clap::ValueEnum)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl FromStr for LogLevel {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            "trace" => Ok(LogLevel::Trace),
            _ => Err(anyhow::Error::msg("invalid log level")),
        }
    }
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
        let settlement_spec = match self.settlement {
            SettlementLayer::Ethereum => match &self.settlement_conf {
                None => {
                    return Err(anyhow::anyhow!(
                        "Settlement configuration is required for Ethereum settlement layer"
                    ));
                }
                Some(settlement_conf_path) => NetworkSpec::Ethereum(
                    EthereumSettlementConfig::from_conf_path(settlement_conf_path)?,
                ),
            },
        };

        let mut op = operator::Operator::new(
            &GLOBAL_ENV.db_path,
            &GLOBAL_ENV.l1addr,
            &GLOBAL_ENV.prover_addr,
            settlement_spec,
        )
        .unwrap();

        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        let mut sigint = signal(SignalKind::interrupt()).unwrap();

        let (stop_tx, stop_rx) = mpsc::channel::<()>(1);

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
        });

        // Launch the custom reth service
        custom_reth::launch_custom_node().await?;

        // Run the operator
        op.run(stop_rx).await
    }
}
