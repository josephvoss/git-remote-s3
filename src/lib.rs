// Module actually doing the heavy lifting
mod git_s3 {
    // Struct containing data needed for methods
    pub struct Remote {
        // Path to auth creds, options set, etc.
        config_path: String,
        // Stream of commands being run (needed?)
        stdin_stream: String,
        // Path to local git object store we're reading from
        git_dir: String
    }
    impl Remote {
        pub fn new(
            config_path: String, stdin_stream: String, git_dir: String,
        ) -> Remote {
            Remote {
                config_path: config_path,
                stdin_stream: stdin_stream,
                git_dir: git_dir,
            }
        }
        // List supported commands
        pub fn capabilities() {
            println!("option");
            println!("fetch");
            println!("push");
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
        pub fn list() {
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
        pub fn option () {
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
        pub fn fetch() {
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
        pub fn list_for_push() {
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
        pub fn push() {
        }
    }
}
