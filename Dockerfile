FROM rust:slim
WORKDIR /minus
COPY ./Cargo.toml /minus/
RUN mkdir src/ && \
  echo "pub fn test() { println!(\"This is a proxy library\"); }" > src/lib.rs
RUN cargo build --all-features
COPY ./ /minus/
CMD cargo check --all-features
