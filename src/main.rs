mod cli;
mod error;
mod blockchain;
mod  block;
mod transaction;
mod tx;
mod wallet;

use crate::cli::Cli;
use error::Result;

fn main() -> Result<()> {
    let mut cli = Cli::new()?;
    cli.run()?;

    Ok(())
}