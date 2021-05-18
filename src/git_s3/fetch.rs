use super::remote::Remote;

use log::{debug, info};
use anyhow::{Context, Error, Result};
use git_object::Kind;
use git_object::immutable::{Commit, Tree};
use git_hash::ObjectId;

impl Remote {
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
    fn fetch_object(&self, sha1: String, obj_type: Kind) -> Result<Option<Vec<u8>>> {
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

        Ok(Some(data))
    }
}
