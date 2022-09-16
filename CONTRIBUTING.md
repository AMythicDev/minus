# How to contribute
First of all, we want to thank you for taking the time to contribute to this project. 

Contributing to a `minus` is pretty straight forward. If this is you're first time, these are the steps you should take.

- Create an issue describing your feature/bug/enhancement and how it will be beneficial.
- State that you are currently working on implementing/fixing the feature/bug/enhancement
- Fork this repo.
- Start from **main** branch and create a seperate branch for making changes.
- Read the code available and make your changes.
- When you're done, submit a pull request for one of the maintainers to check it out. We would let you know if there is
  any problem or any changes that should be considered.
  

## Maintaining code quality and best practices
- Your code should be formatted with rustfmt and should be free from clippy warnings.
- If you're adding/making changes to the public API, write/change the documentation appropriately. Put documentation
  examples where possible. If the code returns a `Result`, describe the Error in the documentation. If it can panic,
  describe that too in the documentation.
  
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
You should follow the convention from the [Git Book](https://git-scm.com/book/ch5-2.html), which states this format for
writing commit messages:

```
Capitalized, short summary of 50 chars or less

More detailed explanatory text, if necessary.  Wrap it to about 72
characters or so.  In some contexts, the first line is treated as the
subject of an email and the rest of the text as the body. The blank
line separating the summary from the body is critical (unless you omit
the body entirely)

Write your commit message in the imperative: "Fix bug" and not "Fixed bug"
or "Fixes bug." 

Further paragraphs come after blank lines.

- Bullet points are okay, too

- Typically a hyphen or asterisk is used for the bullet, followed by a
  single space, with blank lines in between, but conventions vary here

- Use a hanging indent

If the commit closes a issue or part of a pull request, mention it here with these keywords
> close, closes, closed, fixes, fixed
```

For example, you may write a message like this

```
Search: Fix bug where search crashes on pressing /

Fix a bug where the search crashes when user presses the / key.

Closes [Issue ID]
```

## License
Unless explicitly stated otherwise, all code written to this project is dual licensed under the MIT and Apache license
2.0.

The copyrights of `minus` are retained by their contributors and no copyright assignment is required to contribute to
the project.
