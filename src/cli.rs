/// Create cli options struct in module so can be imported in multiple mods
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
/// Remote helper to fetch and push git objects to a S3 bucket
pub struct Opts {
    #[structopt(short, long)]
    #[structopt(default_value = "~/.git-remote-s3.config", env = "GIT_S3_CONFIG")]
    /// Sets a custom config file
    pub config: String,
    /// Name of remote repository
    pub remote_name: String,
    /// Name of remote S3 bucket
    pub remote_bucket: String,
    /// Enable verbose logging (-v, -vv, -vvv, etc)
    #[structopt(short, long, parse(from_occurrences))]
    pub verbose: usize,
}
