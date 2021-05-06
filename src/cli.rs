/// Create cli options struct in module so can be imported in multiple mods
use structopt::StructOpt;

#[derive(StructOpt)]
/// Remote helper to fetch and push git objects to a S3 bucket
pub struct Opts {
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
