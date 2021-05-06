use structopt::StructOpt;
use anyhow::{Context, Result};

#[derive(StructOpt)]
/// Remote helper to fetch and push git objects to a S3 bucket
struct Opts {
    #[structopt(short, long)]
    #[structopt(default_value = "~/.git-remote-s3.config", env = "GIT_S3_CONFIG")]
    /// Sets a custom config file
    config: String,
    /// Name of remote repository
    remoteName: String,
    /// Name of remote S3 bucket
    remoteBucket: String,
    #[structopt(short, long)]
    /// Enable verbose logging
    verbose: bool,
    #[structopt(short, long)]
    /// Enable debug logging
    debug: bool,
}

fn main() -> Result<()> {
    let opts: Opts = Opts::from_args();

    // Set logging level

    // Build git_s3 object

    // Lock stdin

    // Loop over commands on stdin, do work

    // Return
    Ok(())
}
