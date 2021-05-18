use crate::cli;

use super::util::{new_bucket, parse_remote_url};

use log::{debug, info};
use anyhow::{Context, Result};

use std::path::PathBuf;
use s3::bucket::Bucket;
use git_odb::compound::Db;

/// Struct containing data needed for methods
pub struct Remote {
    /// Path to local git object store we're reading from
    pub git_dir: PathBuf,
    /// Bucket we're communicating with
    pub bucket: Bucket,
    /// Git database we're saving data to
    pub git_db: Db,
}

impl Remote {
    /// Create a new Remote object. Mostly just contains the s3::bucket::Bucket object and helper
    /// methods to access it
    pub fn new(opts: cli::Opts) -> Result<Self> {
        info!("Creating new remote with opts: {:?}", opts);

        // Build top level path
        let git_dir = PathBuf::from(opts.git_dir);
        info!("GIT_DIR is \"{}\"", git_dir.to_str().unwrap());

        // Build object DB
        let mut obj_dir = git_dir.clone(); obj_dir.push("objects");
        let db = Db::at(obj_dir)
            .context("Unable to create git db")?;

        // Build bucket
        let (profile_name, endpoint_url, bucket_name, bucket_style) =
            parse_remote_url(opts.remote_url)
            .with_context(|| format!("Unable to parse remote URL"))?;
        // Cast from Option<String> to Option<&str>
        let profile_name = match &profile_name {
            Some(s) => Some(s.as_str()),
            None => None,
        };
        let bucket = new_bucket(
            &bucket_name, profile_name, &endpoint_url, bucket_style
        )?;
        debug!("Bucket is {:?}", bucket);
        Ok( Remote { git_dir: git_dir.clone(), bucket: bucket, git_db: db})
    }
}
