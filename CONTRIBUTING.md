# How to contribute
First of all, we want to thank you for taking the time to contribute to this project. 

Contributing to a `minus` is pretty straight forward. If this is you're first time, these are the steps you should take.

- Create an issue describing your feature/bug/enhancement and how it will be beneficial.
- State that you are currently working on implementing/fixing the feature/bug/enhancement
- Fork this repo.
- Start from **main** branch and create a seperate branch for making changes.
- Read the code available and make your changes.
- When you're done, submit a pull request for one of the maintainers to check it out. We would let you know if there is any problem or any changes that should be considered.

## Maintaining code quality and best practices
- Your code should be formatted with rustfmt and should be free from clippy warnings.
- If you're adding/making changes to the public API, write/change the documentation appropriately. Put documentation examples where possible. If the code returns a `Result`,
describe the Error in the documentation. If it can panic, describe that too in the documentation.
- Every chunk of code has some comments above it. If you write some new code or change some part of the existing code, you should write comments to explain it.
- If you're code only needs to compiled when dynamic features are needed, gate it on the `async_std_lib` feature and `tokio_lib` feature. Gate it on 
`search` feature, if it is required only when search features are needed. Gate it on `static_output`, if it's required when static data needs to be paged.
If you're code is specific to a runtime library, enable the appropriate feature.

## Tests and CI
Your code will automatically be tested by GitHub Actions. If you're code fails in CI, you should fix it appropriately and ensure all tests/examples are passing.

## License
Unless explicitly stated otherwise, all code written to this project is dual licensed under the MIT and Apache license 2.0
