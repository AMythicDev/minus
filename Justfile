_prechecks:
  -cargo hack 2> /dev/null

  if [ $? == 101 ]; then \
    cargo install cargo-hack; \
  fi

fmt:
 cargo fmt --all

check-fmt:
 cargo fmt --all -- --check

build: _prechecks
  cargo hack --feature-powerset build

tests:
 cargo test --all-features --no-run
 cargo test --all-features

examples:
 cargo check --example=dyn_tokio --features=dynamic_output
 cargo check --example=msg-tokio --features=dynamic_output
 cargo check --example=static --features=static_output
 cargo check --example=less-rs --features=dynamic_output,search

lint: _prechecks
  cargo hack --feature-powerset clippy
  
verify-all: check-fmt build tests examples lint
 @echo "Ready to go"
