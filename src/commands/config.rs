use anyhow::Result;

/// ConfigCmd used to write configuration to the stdout.
#[derive(clap::Parser, Debug, Clone, PartialEq, Eq)]
pub struct ConfigCmd {}

impl ConfigCmd {
    pub async fn run(&self) -> Result<()> {
        unimplemented!("TODO: implement ConfigCmd::run()")
    }
}
