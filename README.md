# git-remote-s3

[Git remote helper](https://git-scm.com/docs/gitremote-helpers) to communicate to an S3 backend directly. Able to fetch and
push git objects to S3-compatible object stores.

Heavily leverages [gitoxide](https://github.com/Byron/gitoxide) and
[rust-s3](https://crates.io/crates/rust-s3) to do the heavy lifting talking to
the respective backends. Inspired by
[git-remote-dropbox](https://github.com/anishathalye/git-remote-dropbox),
[git-remote-ipfs](https://github.com/cryptix/git-remote-ipfs), ["How to Write a New Git Protocol"](https://rovaughn.github.io/2015-2-9.html), and ["Developing
a Custom Remote Git
Helper"](https://www.apriorit.com/dev-blog/715-virtualization-git-remote-helper)

## Examples

Credentials either saved in env vars `AWS_ACCESS_KEY_ID` and
`AWS_SECRET_ACCESS_KEY` or the AWS credentials file `~/.aws/credentials`.

```
# Path style bucket
$ git clone s3://play.min.io/git-remote-s3
# Virtual-hosted-style bucket
$ git clone s3://s3.Region.amazonaws.com:git-remote-s3
# Increase log level (1-6)
$ export GIT_S3_LOG_LEVEL=3
# Specify AWS profile
$ git clone s3://non-default-creds@s3.Region.amazonaws.com:git-remote-s3
```

## Installation

This will be published as a crate once it's in a stable v1 release, but until
then you can grab it from the latest Github release, or install it from
source. Git will use any binary with the name `git-remote-s3` in your path as
the s3 remote helper.

```
cargo install --git git@github.com:josephvoss/git-remote-s3.git
```

## Change log

### 0.1.0

Initial release.

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

## Format in s3

* Objects == objects, key is hash ID, content object
* Refs are "pointers" to objects. Key is `refs/<type>/<name>`, contents are key
  ID of object
* Ref dirs have an index at `refs/.`. List of key ids

* Fetch list refs, cat object

## TODO list

* Finish push (fast forward, safe ref updates)
  * think this is finished
* snappy compression for objects saved in s3
* implement list for-push and default remote heads
* Packfiles to speed up remote operations
* parallelize *all the things*
