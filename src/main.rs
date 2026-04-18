mod dump;

use anyhow::Result;
use clap::Parser;

use crate::dump::{Cli, run};

#[tokio::main]
async fn main() -> Result<()> {
    run(Cli::parse()).await
}
