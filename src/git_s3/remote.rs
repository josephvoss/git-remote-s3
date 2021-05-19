use crate::cli;

use super::util::{new_bucket, parse_remote_url};

use log::{trace, debug};
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
    /// Create a new Remote object. Mostly just contains the s3::bucket::Bucket and git object
    /// store database, and helper methods to access them
    ///
    /// Reads in options passed by structopts cli input
    pub fn new(opts: cli::Opts) -> Result<Self> {
        debug!("Creating new remote with opts: {:?}", opts);

        // Build top level path
        let git_dir = PathBuf::from(opts.git_dir);
        debug!("GIT_DIR is \"{:?}\"", git_dir);

        // Build object DB
        let mut obj_dir = git_dir.clone(); obj_dir.push("objects");
        let db = Db::at(obj_dir)
            .context("Unable to create git db")?;

        // Build bucket
        let (profile_name, endpoint_url, bucket_name, bucket_style) =
            parse_remote_url(&opts.remote_url)
            .context("Unable to parse remote URL")?;
       let bucket = new_bucket(
            &bucket_name, profile_name, &endpoint_url, bucket_style
        )?;
        trace!("Bucket is {:?}", bucket);
        Ok( Remote { git_dir, bucket, git_db: db})
    }
}
