# How to contribute
First of all, we want to thank you for taking the time to contribute to this project. 

Contributing to a `minus` is pretty straight forward. If this is you're first time, these are the steps you should take.

- Create an issue describing your feature/bug/enhancement and how it will be beneficial.
- State that you are currently working on implementing/fixing the feature/bug/enhancement
- Fork this repo.
- Start from **main** branch and create a separate branch for making changes.
- Read the code available and make your changes.
- When you're done, submit a pull request for one of the maintainers to check it out. We would let you know if there is
  any problem or any changes that should be considered.

## Maintaining code quality and best practices
- Your code should be formatted with rustfmt and should be free from clippy warnings.
- If you're adding/making changes to the public API, write/change the documentation appropriately. Put documentation
  examples where possible. If the code returns a `Result`, describe it in the the Error section of the item's documentation.
  If it can panic, describe that too in the documentation.
  
- Every chunk of code has some comments above it. If you write some new code or change some part of the existing code,
  you should write comments to explain it.

- Gate your code on appropriate Cargo features if it is required only by that functionality
  - Code related to dynamic paging should be gated on the `dynamic_pagiing` feature.
  - Code related to searching should be gated on the `search` feature.
  - Code related to static output display should be gated on `static_output` feature.

## Tests and CI
Your code will automatically be tested by GitHub Actions. If you're code fails in CI, you should fix it appropriately
and ensure all tests/examples are passing.

## Commit messages
To ensure quality commit messages which also help other fellow developers better understand changes, you should
write commit messages that strictly adhere to [Conventional Commits](https://conventionalcommits.org) v1.0.0. 

### Types
You commit must have a type associated with it. Here are all the types that we encourage people to use ensure commits
can be classified same for everyone contributing to minus.
- `ci` - Changes to GitHub Actions CI wofkflows file
- `chore`: Regular stuff that don't fall into any category like running `rustfmt` etc.
- `docs` - Improvements to documentation
- `feat` - Feature improvements
- `fix` - Bug fixes
- `perf` - Performance improvements
- `refactor` - Changes that don't fix bugs or add features but improves the overall quality of code base.
   You can use this for commits that fix cargo/clippy warnings
- `release` - Used to mark commits that make a new commit on crates.io
- `test`: Commits that touch examples/unit tests/doc tests.

### Scopes
Commit messages following Conventional Commits can optionally describe their **scope**. The scope broadly
describes which parts of the project you commit has touched.

In general, the Rust quailfied name of each file will be it's respect scope. For example `src/state.rs` will have the
`state` scope. Similarly `src/dynamic_pager.rs` will have have scope `dynamic_pager`. With all that, there are a few
exceptions that you should take care of.

- Use the word `manifest` rather than writing `Cargo.toml`
- Use the word `root` rather than writing `src/lib.rs`
- Do not mention the name of parent directories for modules. For example, use `keydefs` for 
  `src/input/definitions/keydefs.rs` or `display` for `src/core/utils/display/mod.rs`.
- Use the name of the module rather than writing the path to its `mod.rs` file. For example, write `core` rather than `src/core/mod.rs`
- Include the name of the parent module if the commit is related to a test. For example, use `display/tests` for `src/core/utils/display/tests.rs`.

## License
Unless explicitly stated otherwise, all code written to this project is dual licensed under the MIT and Apache license
2.0.

The copyrights of `minus` are retained by their contributors and no copyright assignment is required to contribute to
the project.
