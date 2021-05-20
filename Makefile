release:
	cargo build --release

build:
	cargo build

test: lint unittest

doc:
	cargo doc

lint:
	cargo clippy

unittest:
	cargo test git_s3
