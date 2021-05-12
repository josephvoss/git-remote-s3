/// Module actually doing the heavy lifting

use crate::cli;

use log::{trace, debug, info, warn, error};

use anyhow::{Context, Result};

use std::io::{self, Read};
use std::path::{Path, PathBuf};

use s3::bucket::Bucket;
use s3::creds::Credentials;
use s3::region::Region;
use s3::S3Error;

/// Struct containing data needed for methods
pub struct Remote {
    // Stream of commands being run (needed?)
    //stdin_stream: String,
    /// Path to local git object store we're reading from
    git_dir: PathBuf,
    /// Bucket we're communicating with
    bucket: Bucket,
}

impl Remote {

    pub fn new(opts: cli::Opts) -> Result<Self> {
        info!("Creating new remote with opts: {:?}", opts);

        let git_dir = PathBuf::from(opts.git_dir);
        info!("GIT_DIR is \"{}\"", git_dir.to_str().unwrap());

        let (profile_name, bucket_name, endpoint_url) = parse_remote_url(opts.remote_url)
            .with_context(|| format!("Unable to parse remote URL"))?;
        // Cast from Option<String> to Option<&str>
        let profile_name = match &profile_name {
            Some(s) => Some(s.as_str()),
            None => None,
        };
        Ok(Remote {
            git_dir: git_dir,
            bucket: new_bucket(&bucket_name, profile_name, &endpoint_url)?
        })
    }
    // List supported commands
    pub fn capabilities(&self) -> Result<()> {
        println!("option");
        println!("fetch");
        println!("push");
        Ok(())
    }
    /*
     * list
     *
     * Lists the refs, one per line, in the format "<value> <name> [<attr> ...]". The value may
     * be a hex sha1 hash, "@<dest>" for a symref, ":<keyword> <value>" for a key-value pair,
     * or "?" to indicate that the helper could not get the value of the ref. A space-separated
     * list of attributes follows the name; unrecognized attributes are ignored. The list ends
     * with a blank line.
     *
     * Needed by fetch.
     */
    pub fn list(&self) -> Result<()> {
        Ok(())
    }
    /*
     * option <name> <value>
     *
     * Sets the transport helper option <name> to <value>. Outputs a single line containing one
     * of ok (option successfully set), unsupported (option not recognized) or error <msg>
     * (option <name> is supported but <value> is not valid for it). Options should be set
     * before other commands, and may influence the behavior of those commands.
     *
     * Needed by option.
     */
    pub fn option(&self) -> Result<()> {
        Ok(())
    }
    /*
     * fetch <sha1> <name>
     *
     * Fetches the given object, writing the necessary objects to the database. Fetch commands
     * are sent in a batch, one per line, terminated with a blank line. Outputs a single blank
     * line when all fetch commands in the same batch are complete. Only objects which were
     * reported in the output of list with a sha1 may be fetched this way.
     *
     * Needed by fwtch
     */
    pub fn fetch(&self) -> Result<()> {
        Ok(())
    }
    /*
     * list for-push
     *
     * Similar to list, except that it is used if and only if the caller wants to the resulting
     * ref list to prepare push commands. A helper supporting both push and fetch can use this
     * to distinguish for which operation the output of list is going to be used, possibly
     * reducing the amount of work that needs to be performed.
     *
     * Needed by push
     */
    pub fn list_for_push(&self) -> Result<()> {
        Ok(())
    }
    /*
     * push +<src>:<dst>
     *
     * Pushes the given local <src> commit or branch to the remote branch described by <dst>. A
     * batch sequence of one or more push commands is terminated with a blank line (if there is
     * only one reference to push, a single push command is followed by a blank line). For
     * example, the following would be two batches of push, the first asking the remote-helper
     * to push the local ref master to the remote ref master and the local HEAD to the remote
     * branch, and the second asking to push ref foo to ref bar (forced update requested by the
     * +).
     *
     * Needed by push
     */
    pub fn push(&self) -> Result<()> {
        Ok(())
    }

    pub fn run(&self) -> Result<()> {
        loop {
            info!("Reading new line from stdin");
            let mut buffer = String::new();

            // Read next line from stdin
            io::stdin().read_line(&mut buffer)
                .with_context(|| format!("Could not read line from stdin"))?;

            // Split it by space, trim whitespace, build into vector
            let line_vec = buffer.split(" ").map(|x| x.trim()).collect::<Vec<_>>();
            debug!("Line split vector is: {:?}", line_vec);
            let command = line_vec[0];

            // Run it
            let result = match command {
                "capabilities" => {
                    info!("Starting capabilities");
                    self.capabilities()
                },
                "list" => {
                    info!("Starting list");
                    self.list()
                },
                "option" => {
                    info!("Starting option");
                    self.option()
                },
                "fetch" => {
                    info!("Starting fetch");
                    self.fetch()
                },
                "push" => {
                    info!("Starting push");
                    self.push()
                },
                "list_for_push" => {
                    info!("Starting list_for_push");
                    self.list_for_push()
                },
                _ => {
                    info!("No matching command found for: {}", command);
                    info!("Exiting");
                    break Ok(())
                }
            };
            match result {
                Ok(()) => {
                    info!("Ran command {} successfully", command);
                }
                _ => {
                    error!("Error found running {}: {:?}", command, result);
                    break result
                }
            }

        }
    }

}

/// Instantiate new connection
/// Params:
/// * Name of bucket
/// * Name of S3 profile to use. Reads from default creds file or environment
/// * Endpoint URL
fn new_bucket(
    //bucket_name: &str, git_object_dir: String, profile: String, endpoint_url: String,
    bucket_name: &str, profile: Option<&str>, region: &str,
) -> Result<Bucket, anyhow::Error>{

    // Parse config
    // git config --get s3.bucket ? How do remotes?
    // Just using s3 profiles for now
    // TODO - parse profile from remote URL (<profile>@<region>?)

    Bucket::new(
        bucket_name,
        region.parse()
            .with_context(|| format!("Could not create region for \"{}\"", region))?,
        Credentials::new(None, None, None, None, profile)
            .with_context(|| format!(
                "Could not load S3 credentials for profile \"{}\"",
                match profile {
                    Some(content) => content,
                    None => "default",
                }
            ))?,
    ).with_context(|| format!("Could not load S3 bucket \"{}\"", bucket_name))
}

/// Parse remote_url string into optional profile name, and mandatory remote URL and bucket name
///
/// Ex;
/// s3://<profile_name>@<region>/bucket
/// s3://<region>/bucket
fn parse_remote_url(remote_url: String) -> Result<(Option<String>, String, String)> {
    info!("Parsing remote url {}", remote_url);

    // Remove prefix
    let prefix = "s3://";
    let remote_url = remote_url.strip_prefix(prefix)
        .with_context(|| format!(
            "Remote name \"{}\" does not start with {}", remote_url, prefix,
        ))?;

    // Find profile by tokenizing @ if it exists
    let v: Vec<&str> = remote_url.split('@').collect();
    debug!("Split profile vector is {:?}", v);
    let profile: Option<String> = match v.len() {
        1 => None,
        _ => Some(v[0].to_string()),
    };
    info!("Parsed profile \"{}\" from {}",
        match profile {
            Some(ref content) => content,
            None => "default",
        },
        remote_url
    );

    // Find region. From the remaining url, split on /. Region is everything before that
    //
    // Index changes if profile exists or not
    let start_index: usize = match profile {
        Some(_) => 1,
        None => 0,
    };
    // Split on /
    let rb: Vec<&str> = v[start_index].split('/').collect();
    // Join everything but bucket
    let region: String = rb[0..rb.len()-1].join("/").to_string();
    info!("Parsed region \"{}\" from {}", region, remote_url);

    // Find bucket name (last /)
    let bucket: String = rb.last().unwrap().to_string();
    info!("Parsed bucket \"{}\" from {}", bucket, remote_url);

    Ok((profile, region, bucket))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_remote_url() {
        assert_eq!(parse_remote_url("s3://profile@region/bucket".to_string()).unwrap(),
        (Some("profile".to_string()),"region".to_string(),"bucket".to_string()))
    }
    #[test]
    fn test_no_profile_parse_remote_url() {
        assert_eq!(parse_remote_url("s3://region/bucket".to_string()).unwrap(),
        (None,"region".to_string(),"bucket".to_string()))
    }
    #[test]
    #[should_panic]
    fn test_no_prefix_parse_remote_url() {
        let _ = parse_remote_url("region/bucket".to_string()).unwrap();
    }
    #[test]
    fn test_http_format_parse_remote_url() {
        assert_eq!(parse_remote_url("s3://profile_test@https://localhost:9000/bucket12345".to_string()).unwrap(),
        (Some("profile_test".to_string()), "https://localhost:9000".to_string(),"bucket12345".to_string()))
    }
    #[test]
    fn test_http_format_no_profile_parse_remote_url() {
        assert_eq!(parse_remote_url("s3://https://localhost:9000/bucket12345".to_string()).unwrap(),
        (None, "https://localhost:9000".to_string(),"bucket12345".to_string()))
    }
}
