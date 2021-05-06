mod cli;

use structopt::StructOpt;
use anyhow::Result;

fn main() -> Result<()> {
    let opts = cli::Opts::from_args();

    // Set logging level - is this even supported?

    // Build git_s3 object

    // Lock stdin

    // Loop over commands on stdin, do work

    // Return
    Ok(())
}
