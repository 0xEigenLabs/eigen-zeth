use anyhow::Result;

/// ChainInfoCmd used to print the Eigen-Zeth information.
#[derive(clap::Parser, Debug, Clone, PartialEq, Eq)]
pub struct ChainInfoCmd {}

impl ChainInfoCmd {
    pub async fn run(&self) -> Result<()> {
        unimplemented!("TODO: implement ChainInfoCmd::run()")
    }
}
