# minus
A fast, asynchronous terminal paging library for Rust. Minus provides a high level functions to easily write a pager for any binary application
Due to the asynchronous nature, data can be updated without going through hoops

Minus was started by me as my work on pijul. I had dissatisfaction with all existing options like *pager* and *moins*

* Pager:-
    * Only provides functions to join the standard output of the current program to the standard input of external pager like `more` or `less`
    * Due to this, for functioning in Windows, the external pagers need to be packaged along with the executable

* Moins
    * The output could only be defined once and for all. It is not asynchronous and does not support updating

## Installation
* If you use `tokio` for your application, use the `tokio_lib` feature
* If you use `async_std` for your application, use the `async_std_lib` feature

In your `Cargo.toml` file
```toml
[dependencies]
# For tokio
minus = { git = "https://github.com/arijit79/minus.git", features = ["tokio_lib"], tag = "v1.0.0" }

# For async_std
minus = { git = "https://github.com/arijit79/minus.git", features = ["async_std_lib"], tag = "v1.0.0" }
```

## Example
Using tokio

``` rust
use tokio::main;
use futures::join;
use std::sync::{Arc, Mutex};

#[main]
async fn main() {
    let output = Arc::new(Mutex::new(String::new()));
    // Asynchronously push numbers to a string
    let incrementor = async {
        for i in 0..100 {
            let guard = output.lock().unwrap();
            // Always use writeln to add a \n after the line
            writeln!(output, "{}", guard);
            // Drop here explicitly, if you have further asynchronous blocking code
            drop(borrow);
            // Some asynchronous blocking code
            tokio::task::sleep(std::Duration::new(1,0)).await;
        }
    }
    join!(minus::tokio_updating(output.clone()), push_data);
}
```

Using async_std

```rust
use async_std::main;
use futures::join;
use std::sync::{Arc, Mutex};
use std::fmt::Write;

#[main]
async fn main() {
    let output = Arc::new(Mutex::new(String::new()));
    // Asynchronously push numbers to a string
    let incrementor = async {
        for i in 0..100 {
            let guard = output.lock().unwrap();
            // Always use writeln to add a \n after the line
            writeln!(output, "{}", guard);
            // Drop here explicitly, if you have further asynchronous blocking code
            drop(borrow);
            // Some asynchronous blocking code
            async_std::task::sleep(std::Duration::new(1,0)).await;
        }
    }
    join!(minus::async_std_updating(output.clone()), push_data);
}
```

## Contributing
Issues, pull requests are more than welcome
Unless explicitly stated otherwise, all works to minus are dual licensed under the MIT and Apache License 2.0

See the licenses in their respective files