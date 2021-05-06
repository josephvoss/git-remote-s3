mod cli;
mod git_s3;

use structopt::StructOpt;
use anyhow::Result;
use log::{trace, debug, info, warn, error};

fn main() -> Result<()> {
    let opts = cli::Opts::from_args();

    // Set logging level - is this even supported?
    stderrlog::new()
        .module(module_path!())
        .verbosity(opts.verbose)
        .init()
        .unwrap();

    // Build git_s3 object
    let remote = git_s3::Remote::new(opts);

    // Lock stdin

    // Loop over commands on stdin, do work

    // Return
    Ok(())
}
