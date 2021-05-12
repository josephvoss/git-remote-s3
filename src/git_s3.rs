/// Module actually doing the heavy lifting

use crate::cli;

use log::{trace, debug, info, warn, error};

use anyhow::{Context, Error, Result};

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

#[derive(Debug,PartialEq)]
enum BucketStyle {
    Path,
    Subdomain,
}

impl Remote {

    pub fn new(opts: cli::Opts) -> Result<Self> {
        info!("Creating new remote with opts: {:?}", opts);

        let git_dir = PathBuf::from(opts.git_dir);
        info!("GIT_DIR is \"{}\"", git_dir.to_str().unwrap());

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
        Ok( Remote { git_dir: git_dir, bucket: bucket } )
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
    /// List refs that this bucket knows about. Returns all objects in s3 prefaced with `refs/`.
    /// Takes a parameter to return default remote branch (`HEAD`)
    pub fn list(&self, include_head: bool) -> Result<()> {
        let result = self.bucket.list_blocking("refs/".to_string(), None)
            .with_context(|| "List command failed")?;
        /*
        for r in results {
            for object in results.contents {
            }
        }
        */
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
                    let for_push: bool = match line_vec.last() {
                        Some(s) => s.to_string() == "for-push",
                        None => false,
                    };
                    if for_push {info!("For-push")};
                    self.list(for_push)
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
/// * Bucket style to use (true for <remote>/<bucket>, false for <bucket>.<remote>
fn new_bucket(
    //bucket_name: &str, git_object_dir: String, profile: String, endpoint_url: String,
    bucket_name: &str, profile: Option<&str>, region: &str, bucket_style: BucketStyle
) -> Result<Bucket, anyhow::Error>{

    info!("Building new bucket");
    // Parse config
    // git config --get s3.bucket ? How do remotes?
    // Just using s3 profiles for now
    // TODO - parse profile from remote URL (<profile>@<region>?)

    let r = region.parse()
            .with_context(|| format!("Could not create region for \"{}\"", region))?;
    info!("Loaded region is {}", r);
    let c =  Credentials::new(None, None, None, None, profile)
            .with_context(|| format!(
                "Could not load S3 credentials for profile \"{}\"",
                match profile {
                    Some(content) => content,
                    None => "default",
                }
            ))?;
    match bucket_style {
        BucketStyle::Path => Bucket::new_with_path_style(bucket_name, r, c),
        BucketStyle::Subdomain => Bucket::new(bucket_name, r, c),
    }.with_context(|| format!("Could not load S3 bucket \"{}\"", bucket_name))
}

/// Parse remote_url string into optional profile name, and mandatory remote URL and bucket name.
/// Key off `/` or `:` for path style to use
///
/// Ex;
/// s3://<profile_name>@<region>/<bucket>
/// s3://<region>/<bucket>
/// s3://example.com/s3/url/<bucket>
/// s3://s3.example.com/<bucket>
/// s3://<region>:<bucket>
/// s3://s3.example.com:<bucket>
fn parse_remote_url(remote_url: String) -> Result<(Option<String>, String, String, BucketStyle)> {
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

    // Find region. From the remaining url, split on / or :
    // later, and join to the bucket name

    // Index changes if profile exists or not
    let start_index: usize = match profile {
        Some(_) => 1,
        None => 0,
    };
    let remaining_str = v[start_index];
    // Split on last / or :
    let region_bucket: Vec<&str> = remaining_str.rsplitn(2,&['/',':'][..]).collect();
    debug!("Split region_bucket vector is {:?}", region_bucket);
    let region: String = region_bucket.last().unwrap().to_string();
    info!("Parsed region \"{}\" from {}", region, remote_url);

    // Find bucket name (last /).
    let bucket: String = region_bucket.first().unwrap().to_string();
    info!("Parsed bucket \"{}\" from {}", bucket, remote_url);

    // Get path style from sep (length of split region)
    let sep = remaining_str.chars().nth(region.len())
        .with_context(|| format!("Unable to get seperator from {}",remaining_str))?;
    debug!("Sep is {}", sep);
    let style = match sep {
        '/' => BucketStyle::Path,
        ':' => BucketStyle::Subdomain,
        _ => return Err(Error::msg(
                format!("No matching bucket style for {}", sep)
            ))
    };
    info!("Parsed style \"{:?}\" from {}", style, remote_url);

    Ok((profile, region, bucket, style))
}

#[cfg(test)]
mod tests {
    use super::*;

// s3://<profile_name>@<region>/<bucket>
// s3://<region>/<bucket>
// s3://example.com/s3/url/<bucket>
// s3://s3.example.com/<bucket>
// s3://<region>:<bucket>
// s3://s3.example.com:<bucket>
    #[test]
    fn test_parse_remote_url() {
        assert_eq!(parse_remote_url("s3://profile@region/bucket".to_string()).unwrap(),
        (Some("profile".to_string()),"region".to_string(),"bucket".to_string(),BucketStyle::Path))
    }
    #[test]
    fn test_no_profile_parse_remote_url() {
        assert_eq!(parse_remote_url("s3://region/bucket".to_string()).unwrap(),
        (None,"region".to_string(),"bucket".to_string(),BucketStyle::Path))
    }
    #[test]
    #[should_panic]
    fn test_no_prefix_parse_remote_url() {
        let _ = parse_remote_url("region/bucket".to_string()).unwrap();
    }
    #[test]
    fn test_path_with_port_no_profile_parse_remote_url() {
        assert_eq!(parse_remote_url("s3://localhost:9000/bucket12345".to_string()).unwrap(),
        (None, "localhost:9000".to_string(),"bucket12345".to_string(),BucketStyle::Path))
    }
    #[test]
    fn test_url_subdomain_no_profile_parse_remote_url() {
        assert_eq!(parse_remote_url("s3://example.com/long/url:bucket12345".to_string()).unwrap(),
        (None, "example.com/long/url".to_string(),"bucket12345".to_string(),BucketStyle::Subdomain))
    }
    #[test]
    fn test_url_port_subdomain_parse_remote_url() {
        assert_eq!(parse_remote_url("s3://example.com:60000:bucket12345".to_string()).unwrap(),
        (None, "example.com:60000".to_string(),"bucket12345".to_string(),BucketStyle::Subdomain))
    }
}
