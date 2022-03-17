check-fmt:
	cargo fmt --all -- --check

build:
	cargo build --all-features
	cargo build --features "async_std_lib"
	cargo build --features "tokio_lib"
	cargo build --features "static_output"
	# async_std_lib should implicitly check tokio_lib
	cargo build --features "async_std_lib,search"
	cargo build --features "static_output,search"

tests:
	cargo test --all-features --no-run
	cargo test --all-features

examples:
    - cargo check --example=dyn_tokio --features=tokio_lib
    - cargo check --example=dyn_async_std --features=async_std_lib
    - cargo check --example=static --features=static_output
    - cargo check --example=static_long --features=static_output
	
lint:
	cargo clippy --all-features --tests --examples
	cargo clippy --features "async_std_lib"
	cargo clippy --features "tokio_lib"
	cargo clippy --features "static_output"
	# async_std_lib should implicitly check tokio_lib
	cargo clippy --features "async_std_lib,search"
	cargo clippy --features "static_output,search"
