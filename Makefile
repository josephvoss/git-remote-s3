release:
	cargo build --release

debug:
	cargo build --debug

test: lint unittest

doc:
	cargo doc

lint:
	cargo clippy

unittest:
	cargo test git_s3
