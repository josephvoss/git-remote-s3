mod cli;
mod git_s3;

use structopt::StructOpt;
use anyhow::{Result, Error};
use log::{trace, debug, info, warn, error};
use std::env;
use std::path::PathBuf;

fn main() -> Result<()> {
    let opts = cli::Opts::from_args();
    info!("Remote URL is \"{}\"", opts.remote_url);

    // Set logging level - is this even supported?
    stderrlog::new()
        .module(module_path!())
        .verbosity(opts.verbose)
        .init()
        .unwrap();

    // Build git_s3 object
    let remote =
        match git_s3::Remote::new(opts) {
            Ok(content) => content,
            Err(err) => return Err(Error::msg(
                    format!("Unable to create remote: {:?}", err)
                )),
        };

    // Loop over commands on stdin, do work, return when done
    remote.run()
}
