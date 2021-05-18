use super::remote::Remote;

use anyhow::{Context, Result, Error};
use log::{debug, info, error};
use std::io;

impl Remote {
    /// List commands supported by this helper. Currently fetch and push.
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
    pub fn run(&self) -> Result<()> {
        loop {
            info!("Reading new line from stdin");
            let mut buf = String::new();

            // Read next line from stdin
            io::stdin().read_line(&mut buf)
                .with_context(|| format!("Could not read line from stdin"))?;
            info!("Line is: {:?}", &buf);

            // Split it by space, trim whitespace
            let mut line_vec = buf
                .split(" ")
                .map(|x| x.trim());
            let command = line_vec.next()
                .ok_or(Error::msg(format!("Invalid command: {}", buf)))?;

            // Run it
            let result = match command {
                "capabilities" => {
                    info!("Starting capabilities");
                    self.capabilities()
                },
                "list" => {
                    info!("Starting list");
                    let for_push: bool = match line_vec.next() {
                        Some(s) => s == "for-push",
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
                    let fetch_err = "Fetch command has invalid arg";
                    let sha = line_vec.next()
                        .ok_or(Error::msg(format!("{} for sha: {}", fetch_err, buf)))?;
                    let name = line_vec.next()
                        .ok_or(Error::msg(format!("{} for name: {}", fetch_err, buf)))?;
                    self.fetch(sha, name)
                },
                "push" => {
                    info!("Starting push");

                    // Parse for push
                    let push_err = "Push command has invalid arg";
                    let mut colon_iter = line_vec.next()
                        .ok_or(Error::msg(format!("{} from colon split: {}", push_err, buf)))?
                        .split(':');
                    // Get src w/ unknown force prefix
                    let src_str_unk = colon_iter.next()
                        .ok_or(Error::msg(format!("{} from src parsing: {}", push_err, buf)))?;
                    // Key off force push
                    let force_push = match src_str_unk.chars().next() {
                        Some('+') => true,
                        Some(_) => false,
                        _ => return Err(Error::msg(format!("{} from force-push parsing: {}", push_err, buf))),
                    };
                    // Remove src prefix if it exists
                    let src_str = match src_str_unk.strip_prefix('+') {
                        Some(s) => s,
                        None => src_str_unk
                    };
                    // Get regular dst
                    let dst_str = match colon_iter.next() {
                        Some(s) => s,
                        _ => return Err(Error::msg(format!("{} from dst parsing: {}", push_err, buf))),
                    };

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
