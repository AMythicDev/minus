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
[`pijul`]: https://pijul.org/

## Usage

* Using [`tokio`] for your application ? Use the `tokio_lib` feature.
* Using [`async-std`] for your application ? Use the `async_std_lib` feature.
* Using only static information ? Use the `static_output` feature.

In your `Cargo.toml` file:

```toml
[dependencies.minus]
version = "^2.0" 
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
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize a default dynamic configuration
    let pager = minus::Pager::new().finish();

    // Asynchronously push numbers to the output
    let increment = async {
        for i in 0..=30_u32 {
            let mut guard = pager.lock().unwrap();
            writeln!(guard.lines, "{}", i)?;
            drop(guard);
            sleep(Duration::from_millis(100)).await;
        }
        // Return an Ok result
        Result::<_, std::fmt::Error>::Ok(())
    };

    // Join the futures
    let (res1, res2) = join!(
        minus::tokio_updating(pager.clone()),
        increment
    );
    // Check for errors
    res1?;
    res2?;
    // Return Ok result
    Ok(())
}
```

Using [`async-std`]:

```rust
use async_std::task::sleep;
use futures::join;

use std::fmt::Write;
use std::time::Duration;

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize a default dynamic configuration
    let pager = minus::Pager::new().finish();

    // Asynchronously push numbers to the output
    let increment = async {
        for i in 0..=30_u32 {
            let mut guard = output.lock().unwrap();
            writeln!(guard.lines, "{}", i)?;
            drop(guard);
            sleep(Duration::from_millis(100)).await;
        }
        // Return an Ok result
        Result::<_, std::fmt::Error>::Ok(())
    };
    // Join the futures
    let (res1, res2) = join!(
        minus::async_std_updating(guard.clone()), increment);

    // Check for errors
    res1?;
    res2?;
    // Return Ok result
    Ok(())
}
```

Some static output:

```rust
use std::fmt::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize a default static configuration
    let mut output = Pager::new();
    // Push numbers blockingly
    for i in 0..=30 {
        writeln!(output.lines, "{}", i)?;
    }
    // Run the pager
    minus::page_all(output)?;
    // Return Ok result
    Ok(())
}
```

If there are more rows in the terminal than the number of lines in the given
data, `minus` will simply print the data and quit. This only works in static
paging since asynchronous paging could still receive more data that makes it 
pass the limit.

## End user help
Here is some help for the end user using an application that depends on minus

| Action            |   Description|
| ----------        | -------------|
| Ctrl+C/q          | Quit the pager|
| Arrow Up/k        | Scroll up by one line|
| Arrow Down/j      | Scroll down by one line|
| Page Up           | Scroll up by entire page|
| Page Down         | Scroll down by entire page|
| g                 | Go to the very the top of the output|
| G                 | Go to the very the bottom of the output|
| Mouse scroll Up   | Scroll up by 5 lines
| Mouse scroll Down | Scroll down by 5 lines
| Ctrl+L            | Toggle line numbers if not forced enabled/disabled|

## Contributing

Issues and pull requests are more than welcome. Unless explicitly stated
otherwise, all works to `minus` are dual licensed under the MIT and Apache
License 2.0.

See [CONTRIBUTING.md](CONTRIBUTING.md) on how to contribute to `minus`.

See the licenses in their respective files at the root of the project.
