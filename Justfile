fmt:
 cargo fmt --all

check-fmt:
 cargo fmt --all -- --check

build:
 cargo build --no-default-features
 cargo build --features "dynamic_output"
 cargo build --features "static_output"
 cargo build --features "search"
 cargo build --features "dynamic_output,search"
 cargo build --features "static_output,search"
 cargo build --features "static_output,dynamic_output"
 cargo build --all-features

tests:
 cargo test --all-features --no-run
 cargo test --all-features

examples:
 cargo check --example=dyn_tokio --features=dynamic_output
 cargo check --example=msg-tokio --features=dynamic_output
 cargo check --example=static --features=static_output
 cargo check --example=less-rs --features=dynamic_output,search

lint:
 cargo clippy --no-default-features --tests --examples
 cargo clippy --features "dynamic_output" --tests --examples
 cargo clippy --features "static_output" --tests --examples
 cargo clippy --features "search" --tests --examples
 cargo clippy --features "dynamic_output,search" --tests --examples
 cargo clippy --features "static_output,search" --tests --examples
 cargo clippy --features "static_output,dynamic_output" --tests --examples
 cargo clippy --all-features --tests --examples

verify-all: check-fmt build tests examples lint
 @echo "Ready to go"
