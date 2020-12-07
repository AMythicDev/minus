# minus

A fast, asynchronous terminal paging library for Rust. `minus` provides high
level functionalities to easily write a pager for any terminal application. Due
to the asynchronous nature of `minus`, the pager's data can be **updated**.

`minus` supports both [`tokio`] as well as [`async-std`] runtimes. What's more,
if you only want to use `minus` for serving static output, you can simply opt
out of these dynamic features, see the **Usage** section below.

## Why this crate ?

`minus` was started by me for my work on [`pijul`]. I was unsatisfied with the 
existing options like `pager` and `moins`.

* `pager`:
    * Only provides functions to join the standard output of the current
      program to the standard input of external pager like `more` or `less`.
    * Due to this, to work within Windows, the external pagers need to be
      packaged along with the executable.

* `moins`:
    * The output could only be defined once and for all. It is not asynchronous
      and does not support updating.

[`tokio`]: https://crates.io/crates/tokio
[`async-std`]: https://crates.io/crates/async-std

## Usage

* Using [`tokio`] for your application ? Use the `tokio_lib` feature.
* Using [`async_std`] for your application ? Use the `async_std_lib` feature.
* Using only static information ? Use the `static_output` feature.

In your `Cargo.toml` file:

```toml
[dependencies.minus]
version = "^1.0" 
# For tokio
features = ["tokio_lib"]

# For async_std
features = ["async_std_lib"]

# For static output
features = ["static_output"]
```

## Examples

All examples are available in the `examples` directory and you can run them
using `cargo`. Remember to set the correct feature for the targeted example
(e.g.: `cargo run --example=dyn_tokio --features=tokio_lib`).

Using [`tokio`]:

```rust
use futures::join;
use tokio::time::sleep;

use std::fmt::Write;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = Arc::new(Mutex::new(String::new()));

    let increment = async {
        let mut counter: u8 = 0;
        while counter <= 30 {
            let mut output = output.lock().unwrap();
            writeln!(output, "{}", counter.to_string())?;
            counter += 1;
            drop(output);
            sleep(Duration::from_millis(100)).await;
        }
        Result::<_, std::fmt::Error>::Ok(())
    };

    let (res1, res2) = join!(minus::tokio_updating(output.clone()), increment);
    res1?;
    res2?;
    Ok(())
}
```

Using [`async-std`]:

```rust
use async_std::task::sleep;
use futures::join;

use std::fmt::Write;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = Arc::new(Mutex::new(String::new()));

    let increment = async {
        let mut counter: u8 = 0;
        while counter <= 30 {
            let mut output = output.lock().unwrap();
            writeln!(output, "{}", counter.to_string())?;
            counter += 1;
            drop(output);
            sleep(Duration::from_millis(100)).await;
        }
        Result::<_, std::fmt::Error>::Ok(())
    };

    let (res1, res2) = join!(minus::async_std_updating(output.clone()), increment);
    res1?;
    res2?;
    Ok(())
}
```

Some static output:

```rust
use std::fmt::Write;

fn main() -> minus::Result<(), Box<dyn std::error::Error>> {
    let mut output = String::new();

    for i in 1..=30 {
        writeln!(output, "{}", i)?;
    }

    minus::page_all(&output)?;
    Ok(())
}
```

If there are more rows in the terminal than the number of lines in the given
data, `minus` will simply print the data and quit. This only works in static
paging since asynchronous paging could still receive more data that makes it 
pass the limit.

## Contributing

Issues and pull requests are more than welcome. Unless explicitly stated
otherwise, all works to `minus` are dual licensed under the MIT and Apache
License 2.0.

See [CONTRIBUTING.md](CONTRIBUTING.md) on how to contribute to `minus`.

See the licenses in their respective files at the root of the project.
