mod cli;
mod git_s3;

use structopt::StructOpt;
use anyhow::{Context, Result, Error};
use log::info;
use std::env;

fn main() -> Result<()> {
    let opts = cli::Opts::from_args();
    info!("Remote URL is \"{}\"", opts.remote_url);

    // Set logging level - priority to env if cli is 0
    let verbose: usize = match env::var("GIT_S3_LOG_LEVEL") {
        Ok(s) if opts.verbose == 0 => s.parse()
            .with_context(|| format!("Unable to parse `GIT_S3_LOG_LEVEL={}` to usize", s))?,
        Ok(_) => opts.verbose,
        Err(e) => return Err(e).context("Error parsing log level"),
    };

    stderrlog::new()
        .module(module_path!())
        .verbosity(verbose)
//        .color(stderrlog::ColorChoice::Never)
        .init()
        .unwrap();

    // Build git_s3 object
    let remote =
        match git_s3::remote::Remote::new(opts) {
            Ok(content) => content,
            Err(err) => return Err(Error::msg(
                    format!("Unable to create remote: {:?}", err)
                )),
        };

    // Loop over commands on stdin, do work, return when done
    remote.run()
}
