use crate::commands::init::InitCmd;
use crate::commands::{chain_info::ChainInfoCmd, config::ConfigCmd, run::RunCmd};
use anyhow::{bail, Result};

/// Cli is the root command for the CLI.
#[derive(clap::Parser, Debug, Clone)]
#[command(version, author, about)]
pub struct Cli {
    #[command(subcommand)]
    pub subcommand: Option<SubCommand>,
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum SubCommand {
    Run(RunCmd),
    ChainInfo(ChainInfoCmd),
    Config(ConfigCmd),
    Init(InitCmd),
}

impl Cli {
    pub async fn run(&self) -> Result<()> {
        match &self.subcommand {
            Some(SubCommand::Run(cmd)) => cmd.run().await,
            Some(SubCommand::Init(cmd)) => cmd.run().await,
            Some(SubCommand::ChainInfo(cmd)) => cmd.run().await,
            Some(SubCommand::Config(cmd)) => cmd.run().await,
            None => {
                bail!("No subcommand provided")
            }
        }
    }
}
