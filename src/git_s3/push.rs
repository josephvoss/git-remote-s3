use super::remote::Remote;

use log::{debug, info};
use anyhow::{Context, Error, Result};
use std::fs;

use git_object::immutable::{Commit, Tree};
use git_hash::ObjectId;

impl Remote {
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
    fn check_hash_remote(&self, sha1: String) -> Result<bool> {
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
}
