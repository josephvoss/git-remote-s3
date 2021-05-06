//extern crate clap;
//use clap::{AppSettings, Clap, Arg, App, SubCommand, crate_authors, crate_version};
use clap::{AppSettings, Clap, crate_authors, crate_version};
use anyhow::{Context, Result};

#[derive(Clap)]
// Remote helper to fetch and push git objects to a S3 bucket
struct Opts {
    #[clap(short, long)]
    #[clap(default_value = "~/.git-remote-s3.config", env = "GIT_S3_CONFIG")]
    // Sets a custom config file
    config: String,
    // Name of remote repository
    remoteName: String,
    // Name of remote S3 bucket
    remoteBucket: String,
    #[clap(short, long)]
    verbose: bool,
    #[clap(short, long)]
    debug: bool,
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    // Load vars
//jkk    let config = matches.value_of("config").unwrap_or("~/.git-remote-s3.config");
//jkk    let remote = matches.value_of("remote-name")
//jkk        .with_context(|| format!("No remote-name argument specifed!"))?;
//jkk    let bucket = matches.value_of("remote-bucket")
//jkk        .with_context(|| format!("No remote-bucket argument specifed!"))?;
    //println!("Value for config: {}", config);

    // Set logging level

    // Build git_s3 object

    // Lock stdin

    // Loop over commands on stdin, do work

    // Return
    Ok(())
}
