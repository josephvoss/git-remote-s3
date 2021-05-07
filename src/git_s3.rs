/// Module actually doing the heavy lifting

use crate::cli;

use log::{trace, debug, info, warn, error};

use anyhow::{Context, Result};

use std::io::{self, Read};

/// Struct containing data needed for methods
pub struct Remote {
    /// Path to auth creds, options set, etc.
    config_path: String,
    // Stream of commands being run (needed?)
    //stdin_stream: String,
    /// Path to local git object store we're reading from
    git_dir: String
}

impl Remote {

    pub fn new(opts: cli::Opts) -> Self {
        error!("error");
        warn!("warn");
        info!("info");
        debug!("debug");
        Remote {config_path: opts.config, git_dir: "hi".to_string()}
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

            // Split it by space
            let line_vec = buffer.split(" ").collect::<Vec<_>>();
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
