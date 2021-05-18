use s3::bucket::Bucket;

use log::{debug, info};
use anyhow::{Context, Error, Result};

use s3::creds::Credentials;

#[derive(Debug,PartialEq)]
pub enum BucketStyle {
    Path,
    Subdomain,
}

/// Instantiate new connection
/// Params:
/// * Name of bucket
/// * Name of S3 profile to use. Reads from default creds file or environment
/// * Endpoint URL
/// * Bucket style to use (true for <remote>/<bucket>, false for <bucket>.<remote>
pub fn new_bucket(
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
pub fn parse_remote_url(remote_url: String) -> Result<(Option<String>, String, String, BucketStyle)> {
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
