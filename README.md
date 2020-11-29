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
In your `Cargo.toml` file

```toml
[dependencies]
minus = { git = "https://github.com/arijit79/minus.git" }
```
