# minus

<p align="center">
    <img src="./minus.svg"/>
</p>

[![crates.io](https://img.shields.io/crates/v/minus?style=for-the-badge)](https://crates.io/crates/minus)
[![ci](https://img.shields.io/github/workflow/status/arijit79/minus/ci?label=CI&style=for-the-badge)](https://github.com/arijit79/minus/actions/workflows/ci.yml)
[![docs.rs](https://img.shields.io/docsrs/minus?label=docs.rs&style=for-the-badge)](https://docs.rs/minus)
[![chat](https://img.shields.io/badge/chat-on%20zulip-blue?style=for-the-badge)](https://minus.zulipchat.com/)
[![Crates.io](https://img.shields.io/crates/l/minus?style=for-the-badge)](https://github.com/arijit79/minus#license)

`minus` is a small terminal paging library for Rust. `minus` provides an intuitive API for easily embedding a pager in any terminal application. It does all the low level stuff for you like setting up the terminal on start, handling keyboard/mouse/terminal resize events etc.

<p align="center">
    <img src="./demo.png"/>
</p>

The basic thing that `minus` does is to take some string data and display it one page at a time. What makes `minus` unique is that it can allow the end-application to update it's data and configuration while running.

Every functionality in `minus` is gated on certain Cargo feature. By default `minus` comes with no features turned on. This is to prevent end-applications from getting useless dependencies.

## Usage
When adding `minus` to your `Cargo.toml` file, enable the features as necessory
* Using [`tokio`] for your application ? Use the `tokio_lib` feature.
* Using [`async-std`] for your application ? Use the `async_std_lib` feature.
* Using only static information ? Use the `static_output` feature.
* Want search capablities using regex? Enable the `search` feature.

```toml
[dependencies.minus]
version = "^4.0"
# For tokio
features = ["tokio_lib"]

# For async_std
features = ["async_std_lib"]

# For static output
features = ["static_output"]

# Search feature
features = ["tokio_lib", "search"]
```

## Examples
All examples are available in the `examples` directory and you can run them
using `cargo`. Remember to set the correct feature for the targeted example
(e.g.: `cargo run --example=dyn_tokio --features=tokio_lib`).

Using [`tokio`]:

```rust
use futures::join;
use tokio::time::sleep;
use minus::{Pager, async_std_updating};
use std::fmt::Write;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize a default dynamic configuration
    let pager = minus::Pager::new().unwrap().finish();

    // Asynchronously push numbers to the output
    let increment = async {
        for i in 0..=30_u32 {
            let mut guard = pager.lock().await;
            writeln!(guard, "{}", i)?;
            // Also you can use this syntax
            // guard.push_str(&format("{}\n", i));
            drop(guard);
            sleep(Duration::from_millis(100)).await;
        }
        // Dynamic paging should hint the pager that it's stream of data has
        // ended
        let mut guard.lock().await;
        guard.end_data_stream();
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
use minus::{Pager, async_std_updating};
use std::fmt::Write;
use std::time::Duration;

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize a default dynamic configuration
    let pager = minus::Pager::new().unwrap().finish();

    // Asynchronously push numbers to the output
    let increment = async {
        for i in 0..=30_u32 {
            let mut guard = pager.lock().await;
            writeln!(guard, "{}", i)?;
            // Also you can use this syntax
            // guard.push_str(&format("{}\n", i));
            drop(guard);
            sleep(Duration::from_millis(100)).await;
        }
        // Dynamic paging should hint the pager that it's stream of data has
        // ended
        let mut guard.lock().await;
        guard.end_data_stream();
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
use minus::{Pager, page_all};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize a default static configuration
    let mut output = Pager::new().unwrap();
    // Push numbers blockingly
    for i in 0..=30 {
        writeln!(output, "{}", i)?;
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

## Standard actions
Here is the list of default key/mouse actions handled by `minus`. Note that end-applications can change these bindings to better suit their needs.

| Action            | Description                                        |
| ----------        | -------------                                      |
| Ctrl+C/q          | Quit the pager                                     |
| Arrow Up/k        | Scroll up by one line                              |
| Arrow Down/j      | Scroll down by one line                            |
| Page Up           | Scroll up by entire page                           |
| Page Down         | Scroll down by entire page                         |
| Enter             | Scroll down by one line or clear prompt messages   |
| Space             | Scroll down by one page                            |
| Ctrl+U/u          | Scroll up by half a screen                         |
| Ctrl+D/d          | Scroll down by half a screen                       |
| g                 | Go to the very top of the output                   |
| G                 | Go to the very bottom of the output                |
| Mouse scroll Up   | Scroll up by 5 lines                               |
| Mouse scroll Down | Scroll down by 5 lines                             |
| Ctrl+L            | Toggle line numbers if not forced enabled/disabled |
| /                 | Start forward search                               |
| ?                 | Start backward search                              |
| Esc               | Cancel search input                                |
| n                 | Go to the next search match                        |
| p                 | Go to the next previous match                      |

## License
Unless explicitly stated, all works to `minus` are dual licensed under the
[MIT License](./LICENSE-MIT) and [Apache License 2.0](./LICENSE-APACHE)

## Contributing
Issues and pull requests are more than welcome.
See [CONTRIBUTING.md](CONTRIBUTING.md) on how to contribute to `minus`.

## Thanks
Thank you to everyone here for giving there time and contribution to `minus`
* @rezural
* @poliorcetics
* @danieleades
* @mark-a
* @mkatychev
* @tomstoneham
* @Hardy7cc
* @tomstoneham
* @iandwelker

## Get in touch
We are open to discussion and thoughts om improving `minus`. Join us at
[Zulip](https://minus.zulipchat.com)
