# git-remote-s3

Git remote helper to communicate to an S3 backend directly.

Read from a config file for authentication tokens and keys

Need to implement the following

* list
  * List remote refs by "$objectname $refname\n"
* capabilities
  * List supported caps (mainly functions below)
* import
  * Import remote refs to local. Provided in batch w/ newline sep "import
    $refname" (git fast-import stream). Return git fast-export on stdout
* export
  * Export local refs to remote. Provided in batch w/ newline sep "import
    $refname"
* refspec
* import-marks
* export-marks
* options (Change config file settings? cli verbosity overwrites?)
* push (List remote refs, push local commits and history to them)
* fetch (Get remote refs, download objs referred to)

## Why this is *terrible*

* Git fast-import/export -> copies *entire repository* to remote storage on
  push. No diff generation?
* Dangling refs/no git-gc. We're treating the remote file store as a local git
  object dir that is addition only. Ballooning repos
* I.e. no `git upload-pack` or `git receive-pack`, it's dumb
  * Wasn't that the point? Treat origin as a remote file store?
  * :yesbutactuallyno:
  * How do we download partial commits? We fetch the refs. Git can't possibly
    walk each ref back to source and then req it (if it's doing that it does it
quickly).
  * Are we changing the commit object IDs? Hashes aren't saved anywhere from
    what I can tell
* import vs fetch!!!! We can just fetch refs we already have!

## Format in s3

* Objects == objects, key is hash ID, content object
* Refs are "pointers" to objects. Key is `refs/<type>/<name>`, contents are key
  ID of object
* Ref dirs have an index at `refs/.`. List of key ids

* Fetch list refs, cat object

## TODO list

* Finish push (fast forward, safe ref updates)
* snappy compression for objects saved in s3
* implement list for-push
* Packfiles
