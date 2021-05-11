mod cli;
mod git_s3;

use structopt::StructOpt;
use anyhow::{Result, Error};
use log::{trace, debug, info, warn, error};
use std::env;
use std::path::PathBuf;

fn main() -> Result<()> {
    let opts = cli::Opts::from_args();
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        return Err(Error::msg(
                format!("Invalid args: Expected 2, got {}", args.len())
            ));
    }
    let remote_url = &args[2];
    info!("Remote URL is \"{}\"", remote_url);

    let git_dir = match env::var("GIT_DIR") {
        Ok(content) => content,
        Err(err) => return Err(Error::msg(
                format!("Unable to read GIT_DIR from env: {:?}", err)
            )),
    };
    let git_dir = PathBuf::from(git_dir);
    info!("GIT_DIR is \"{}\"", git_dir.to_str().unwrap());

    // Set logging level - is this even supported?
    stderrlog::new()
        .module(module_path!())
        .verbosity(opts.verbose)
        .init()
        .unwrap();

    // Build git_s3 object
    let remote =
        match git_s3::Remote::new(remote_url.to_string(), git_dir, opts) {
            Ok(content) => content,
            Err(err) => return Err(Error::msg(
                    format!("Unable to create remote: {:?}", err)
                )),
        };

    // Loop over commands on stdin, do work, return when done
    remote.run()
}
