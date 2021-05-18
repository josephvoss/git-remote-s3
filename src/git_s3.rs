/// Module actually doing the heavy lifting

use crate::cli;

use log::{trace, debug, info, warn, error};

use anyhow::{Context, Error, Result};

use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::fs;

use s3::bucket::Bucket;
use s3::creds::Credentials;
use s3::region::Region;
use s3::S3Error;

use git_object::immutable::{Blob, Commit, Object, Tree};
use git_object::Kind;
use git_hash::{ObjectId, oid};
use git_odb::compound::Db;

/// Struct containing data needed for methods
pub struct Remote {
    /// Path to local git object store we're reading from
    git_dir: PathBuf,
    /// Bucket we're communicating with
    bucket: Bucket,
    /// Git database we're saving data to
    git_db: Db,
}

#[derive(Debug,PartialEq)]
enum BucketStyle {
    Path,
    Subdomain,
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
    /// List supported commands
    pub fn capabilities(&self) -> Result<()> {
        // println!("option");
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
    /// Prints "<data> <key>"
    pub fn list(&self, include_head: bool) -> Result<()> {
        let results = self.bucket.list_blocking("refs/".to_string(), None)
            .with_context(|| "List command failed")?;
        for (r, code) in results {
            if code != 200 {
                return Err(Error::msg(format!("Non-okay list for \'{}\': {}", "refs/", code)))
            }
            debug!("Result in list is {:?}", r);
            for object in r.contents {
                debug!("Content in list is {:?}", object);
                let (data, code) = self.bucket.get_object_blocking(&object.key)
                    .with_context(|| format!("Unable to list content for ref \'{}\'", &object.key))?;
                if code != 200 {
                    return Err(Error::msg(format!("Non-okay cat for \'{}\': {}", &object.key, code)))
                }
                let string_data = std::str::from_utf8(&data)?;
                println!("{} {}", string_data.trim(), object.key);
            }
        }
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
    /*
     pub fn option(&self) -> Result<()> {
        Ok(())
    }
    */
    /*
     * fetch <sha1> <name>
     *
     * Fetches the given object, writing the necessary objects to the database. Fetch commands
     * are sent in a batch, one per line, terminated with a blank line. Outputs a single blank
     * line when all fetch commands in the same batch are complete. Only objects which were
     * reported in the output of list with a sha1 may be fetched this way.
     *
     * Needed by fetch
     */
    /// This is a mess of copys and string passing for what should be byte arrays. I have no idea
    /// how to clean it up at the moment
    pub fn fetch(&self, sha1: String, name: String) -> Result<()> {
        // Fetch commit
        self.fetch_commit(sha1)
    }
    /// Fetch a commit, and all objects it depends on
    fn fetch_commit(&self, sha1: String) -> Result<()> {
        let data: Vec<u8> = match self.fetch_object(sha1.clone(), Kind::Commit)
            .with_context(|| format!("Unable to fetch commit \'{}\'", &sha1))? {
            Some(d) => d,
            // If we returned ok but w/ empty data the object already exists. Exit
            None => return Ok(()),
        };

        debug!("{} was a commit. Parsing", sha1);
        // Parse object, fetch deps
        let commit_obj = Commit::from_bytes(&data)?;
        debug!("Searching for children of {}", sha1);
        self.fetch_tree(std::str::from_utf8(&commit_obj.tree().to_sha1_hex())?.to_string())
            .with_context(|| format!("Unable to fetch tree for commit \'{}\'", &sha1))?;
        commit_obj.parents()
            .map(|obj| self.fetch_commit(std::str::from_utf8(&obj.to_sha1_hex())?.to_string()))
            .collect::<Result<()>>()
            .with_context(|| format!("Unable to fetch parent for commit \'{}\'", &sha1))?;
        Ok(())
    }
    /// Fetch a tree recursively
    fn fetch_tree(&self, sha1: String) -> Result<()> {
        let data: Vec<u8> = match self.fetch_object(sha1.clone(), Kind::Tree)
            .with_context(|| format!("Unable to fetch tree \'{}\'", &sha1))? {
            Some(d) => d,
            // If we returned ok but w/ empty data the object already exists. Exit
            None => return Ok(()),
        };

        debug!("{} was a tree. Parsing", sha1);
        // Parse tree, fetch deps
        let tree_obj = Tree::from_bytes(&data)?;
        debug!("Searching for children of {}", sha1);
        // Iter over entries, fetch tree or object
        tree_obj.entries.iter()
            .map(|e| {
                 let sha1 = std::str::from_utf8(&e.oid.to_sha1_hex())?.to_string();
                 if e.mode.is_tree() {
                     self.fetch_tree(sha1.clone())
                 } else {
                     match self.fetch_object(sha1.clone(), Kind::Blob) {
                         Ok(_) => Ok(()),
                         Err(error) => Err(error),
                     }
                 }
            })
            .collect::<Result<()>>()
            .with_context(|| format!("Unable to fetch entries for tree \'{}\'", &sha1))?;
        Ok(())
    }
    /// Fetch an object from remote by SHA, save to local git object store.
    /// Blocks
    fn fetch_object(&self, sha1: String, obj_type: Kind) -> Result<(Option<(Vec<u8>)>)> {
        debug!("Fetching object {}", sha1);

        // Build oid
        let id = ObjectId::from_hex(sha1.as_bytes()).with_context(|| format!("Unable to load tree into ObjectId"))?;

        // Check ref if already exists, return None if true
        let mut buf = Vec::new();
        if self.git_db.find(id, &mut buf, &mut git_odb::pack::cache::Never)
            .context("Error found searching db prior to fetch")?.is_none()
        {
            return Ok(None)
        }

        // If not, get data
        let (data, code) = self.bucket.get_object_blocking(&sha1)
            .with_context(|| format!("Unable to fetch object\'{}\'", sha1))?;
        info!("Fetch for \'{}\': {}", sha1, code);
        if code != 200 {
            return Err(Error::msg(format!("Non-okay fetch for \'{}\': {}", sha1, code)))
        }

        // Save to git database
        {
            use git_odb::Write;
            use git_hash::Kind;
            let new_obj = self.git_db.write_buf(obj_type, &data, Kind::Sha1)
                .with_context(|| format!("Unable to write to git database"))?;
        };

        Ok((Some(data)))
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
    // Order of uploads should be blob -> tree -> commits -> refs
    // i.e. small atomic objects first, nested objects and references last
    pub fn push(&self, src_string: String, dst_string: String, force_push: bool) -> Result<()> {
        // Read local ref
        debug!("Reading local ref");
        // Build path
        let mut path = self.git_dir.clone(); path.push(&src_string);
        if !path.exists() {
            return Err(Error::msg(format!("Unable to find local ref for {}", &src_string)))
        }

        let push_sha = fs::read_to_string(path).with_context(|| format!("Unable to read ref {}", &src_string))?;
        let push_sha = push_sha.trim();
        debug!("Local ref: {} to {}", &src_string, push_sha);

        // Push this commit
        self.upload_commit(push_sha.to_string())
            .with_context(|| format!("Unable to upload commit for {}", &src_string))?;

        // Finally, update the ref
        // TODO - verify it's a fast forward
        let (_, code) = self.bucket.put_object_blocking(dst_string, push_sha.as_bytes())
            .with_context(|| format!("Unable to upload commit"))?;
        match code {
            200 => Ok(()),
            _ => Err(Error::msg(format!("Non-okay push for \'{}\': {}", push_sha, code))),
        }
    }
    /// Check if a passed sha exists in the configured bucket
    fn check_hash_remote(&self, sha1: String) -> Result<(bool)> {
        let results = self.bucket.list_blocking(sha1.clone(), None)
            .with_context(|| format!("Check existence of remote object {} failed", &sha1))?;
        debug!("Results of list is {:?}", &results);
        for (r, code) in results {
            if code != 200 {
                return Err(Error::msg(format!("Non-okay list for \'{}\': {}", &sha1, code)))
            }
            debug!("Result in check is {:?}", r);
            if r.contents.len() != 0 {
                info!("Object {} exists remotely, exitting", &sha1);
                return Ok(true)
            }
        }
        Ok(false)
    }

    /// Upload a commit if it doesn't exist remotely. Also verify all objects it describes exists
    /// (parents, tree)
    fn upload_commit(&self, sha1: String) -> Result<()> {
        // Load commit from sha
        info!("Uploading {}", &sha1);
        let mut buf = Vec::new();
        let id = ObjectId::from_hex(sha1.as_bytes()).with_context(|| format!("Unable to load commit into ObjectId"))?;
        debug!("Object id is {:?}", id);
        let new_obj = self.git_db.find(id, &mut buf, &mut git_odb::pack::cache::Never)
            .with_context(|| "Unable to search local database")?;
        let new_obj = match new_obj {
            Some(s) => s,
            None => return Err(Error::msg("object not found in database")),
        };

        // Check if exists. If so, exit
        if self.check_hash_remote(sha1.clone())
            .with_context(|| "Unable to check state of commit")? {
            return Ok(())
        }

        // Parse the object
        let commit_obj = Commit::from_bytes(new_obj.data)
            .with_context(|| "Unable to parse commit")?;
        // Upload commit

        // Parse tree, fetch deps
        self.upload_tree(commit_obj.tree().to_sha1_hex_string())
            .with_context(|| "Unable to upload tree")?;
        commit_obj.parents()
            .map(|obj| self.upload_commit(obj.to_sha1_hex_string()))
            .collect::<Result<()>>()
            .with_context(|| format!("Unable to fetch parent for commit \'{}\'", &sha1))?;

        // Now upload
        let (_, code) = self.bucket.put_object_blocking(&sha1, &new_obj.data)
            .with_context(|| format!("Unable to upload commit"))?;
        match code {
            200 => Ok(()),
            _ => Err(Error::msg(format!("Non-okay push for commit \'{}\': {}", &sha1, code))),
        }
    }
    /// Upload a tree if it doesn't exist remotely. Also verify all objects it describes exists
    /// (subtrees, blobs)
    fn upload_tree(&self, sha1: String) -> Result<()> {
        info!("Uploading tree {}", &sha1);
        // Load tree from sha
        let mut buf = Vec::new();
        let id = ObjectId::from_hex(sha1.as_bytes()).with_context(|| format!("Unable to load tree into ObjectId"))?;
        debug!("Object id is {:?}", id);
        let new_obj = self.git_db.find(id, &mut buf, &mut git_odb::pack::cache::Never)
            .with_context(|| "Unable to search local database")?;
        let new_obj = match new_obj {
            Some(s) => s,
            None => return Err(Error::msg("object not found in database")),
        };

        // Check if exists. If so, exit
        if self.check_hash_remote(sha1.clone())
            .with_context(|| "Unable to check state of tree")? {
            return Ok(())
        }

        // Parse the object
        let tree_obj = Tree::from_bytes(new_obj.data)?;
        debug!("Searching for children of {}", &sha1);
        // Iter over entries, fetch tree or object
        tree_obj.entries.iter()
            .map(|e| {
                 let sha1 = std::str::from_utf8(&e.oid.to_sha1_hex())?.to_string();
                 if e.mode.is_tree() {
                     self.upload_tree(sha1.clone())
                 } else {
                     match self.upload_blob(sha1) {
                         Ok(_) => Ok(()),
                         Err(error) => Err(error),
                     }
                 }
            })
            .collect::<Result<()>>()
            .with_context(|| format!("Unable to push entries for tree \'{}\'", &sha1))?;
        // Now upload
        let (_, code) = self.bucket.put_object_blocking(&sha1, &new_obj.data)
            .with_context(|| format!("Unable to upload tree"))?;
        match code {
            200 => Ok(()),
            _ => Err(Error::msg(format!("Non-okay push for tree \'{}\': {}", &sha1, code))),
        }
    }
    /// Upload a blob if it doesn't exist remotely
    fn upload_blob(&self, sha1: String) -> Result<()> {
        info!("Uploading blob {}", &sha1);
        // Load blob from sha
        let mut buf = Vec::new();
        let id = ObjectId::from_hex(sha1.as_bytes()).with_context(|| format!("Unable to load tree into ObjectId"))?;
        debug!("Object id is {:?}", id);
        let new_obj = self.git_db.find(id, &mut buf, &mut git_odb::pack::cache::Never)
            .with_context(|| "Unable to search local database")?;
        let new_obj = match new_obj {
            Some(s) => s,
            None => return Err(Error::msg("object not found in database")),
        };

        // Check if exists. If so, exit
        if self.check_hash_remote(sha1.clone())
            .with_context(|| "Unable to check state of commit")? {
            return Ok(())
        }
        // Otherwise, upload
        let (_, code) = self.bucket.put_object_blocking(&sha1, &new_obj.data)
            .with_context(|| format!("Unable to upload blob"))?;
        match code {
            200 => Ok(()),
            _ => Err(Error::msg(format!("Non-okay push for blob \'{}\': {}", &sha1, code))),
        }
    }

    pub fn run(&self) -> Result<()> {
        loop {
            info!("Reading new line from stdin");
            let mut buffer = String::new();

            // Read next line from stdin
            io::stdin().read_line(&mut buffer)
                .with_context(|| format!("Could not read line from stdin"))?;
            info!("Line is: {:?}", &buffer);

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
                /*
                "option" => {
                    info!("Starting option");
                    self.option()
                },
                */
                "fetch" => {
                    info!("Starting fetch");
                    // Parse for fetch
                    // TODO error catch
                    if line_vec.len() < 3 {
                        return Err(Error::msg(format!("Fetch command has invalid args: {:?}", line_vec)))
                    }
                    let sha = line_vec[1].to_string();
                    let name = line_vec[2].to_string();
                    self.fetch(sha, name)
                },
                "push" => {
                    info!("Starting push");

                    // Parse for push
                    // TODO anyway to do this w/ less :gross: matches?
                    let parsing_err = Err(Error::msg(format!("Push command has invalid args: {:?}", line_vec)));
                    if line_vec.len() < 2 {
                        return parsing_err
                    }
                    let mut colon_iter = line_vec[1].split(':');
                    // Get src w/ unknown force prefix
                    let src_str_unk = match colon_iter.next() {
                        Some(s) => s,
                        _ => return parsing_err
                    };
                    // Key off force push
                    let force_push = match src_str_unk.chars().next() {
                        Some('+') => true,
                        Some(_) => false,
                        _ => return parsing_err
                    };
                    // Remove src prefix if it exists
                    let src_str = match src_str_unk.strip_prefix('+') {
                        Some(s) => s,
                        None => src_str_unk
                    }.to_string();
                    // Get regular dst
                    let dst_str = match colon_iter.next() {
                        Some(s) => s,
                        _ => return parsing_err
                    }.to_string();

                    info!("Pushing {} to {} {}", src_str, dst_str, if force_push {"forcefully"} else {""});
                    self.push(src_str, dst_str, force_push)
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
                    println!();
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

/// Build git ref path from sha
/// join git_dir w/ sha1[0:2], sha1[2:]

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
