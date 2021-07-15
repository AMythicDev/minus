# Changelog
This file documents all noteable changes made to this project

## v4.0.0
### Added
* Introduced robust line wrappings using the textwrap crate
* Added a `Pager::send_message` function to send messages at the prompt site
* Added `Space` and `Enter` keybindings
* The docs now show feature tags on items that are gated on features
* The `Pager::set_prompt` function now panics if it contains a line with \n characters
* Added a Code of Conduct
* Implemented the `std::fmt::Write` trait directly on `Pager`
* Add new examples 3 new examples:- *color_output*, *msg-tokio*, *tokio-no-overflow*
* Expanded the test suite of minus

### Changed
* The `Pager::set_page_if_havent_overflowed` has been replaced with `Pager::set_run_no_overflow`
* The `Pager::set_data_finished` has been replaced with `Pager::end_data_stream`
* All fields inside the pager are now private and cannot be accessed or directly written to
* All tests now run without requiring `--all-features`
* The `Pager::new` function now returns a `Result<Pager, TermError>`
* The Pager API has changed from a builder pattern to a more programatic pattern like

  ```rust
  use minus::Pager;

  let mut pager = Pager::new().unwrap();
  pager.set_prompt("Example")
  ```

* Line Numbers displayed are now bold and have a little more left padding
* Next and Previous now simply move the display to the match line number rather than moving the cursor

### Fixed
* Prevent panic if invalid regex is given during search
* Fix run\_no\_overflow for static pager (#43)
  
   Previously, this setting had no effect if paging static output, due to an if condition in
   `static_pager.rs` which did not consider the setting. This commit makes
   this setting behave as expected. (@tomstoneham) 

* The cursor is hidden as soon as the search query entry is complete.
* Fix where color outputs get distorted after search matches

## v3.4.0 [2021-5-26]
### Added
* u and d keys for half page scrolling

### Fixed
* The reverse direction of j and k keys

## v3.3.3 [2021-4-29]
### Fixed
* Fixed warnings issued by clippy

## v3.3.2 [2021-4-28]
### Added
* Add docs for InputHandler and DefaultInputHandler

## v3.3.1 [2021-3-13]
### Fixed
* Documentation build failure

## v3.3.0 [2021-3-5]
### Added
* A trait to coustomize the default keybindings

### Fixed
* Fixed bug where cursor movement stops at last search instance

## v3.2.0 [2021-2-22]
### Added
* A new function to signal the end of data to the pager

### Changed
* Page cleanup when the pager needs to quit

## v3.1.2 [2021-2-16]
### Bug fixes
* Fixed bug where text with large amount of lines where not displayed
* Fixed bug last line of text was not displayed when the string is pushed with multiple lines

## v3.1.1 [2021-2-5]
### Documentation Fixes
* Added information about the backward search feature in README

### Bug fixes
* Removed the unstable `spin\_loop` function

## v3.1.0 [2021-1-14]
### Added
* Backward searching

### Bug fixes
* Fix bug where cursor is placed in wrong position if upper\_mark is not 1

## v3.0.2 [2021-1-12]
### Bug Fixes
* If two consecutive searches are done in a single session, the previous search
highlghts are not removed
* If the same search query is called twice, the matches are repeated

### Documentation Fixes
* Add info about `search` feature
* Add info about `searchable` field and `set_searchable` soft-deprecation

## v3.0.1 [2021-1-10]
### Documentation Fixes
* Fix README examples

## v3.0.0 [2021-1-10]
### Added
* A special mutex for the Pager that is runtime-agnostic and implements `Send` +
`Sync`
* Search using the `/` key
	* `n` and `p` to go to the next/previous match respectively

### Changed
* Simplification how text is output to the screen when the terminal is not filled

## v2.1.0 [2020-12-24]
### Added
* Ability to control the exit strategy

### Changed
* Minimum requirement for tokio is tokio 1.0

## v2.0.2 [2020-12-16]
### Documentation Fixes
* Fix docs where features are shown to enabled even when they are disabled
by default

## v2.0.1 [2020-12-16]
### Documentation Fixes
* Change version in README

## v2.0.0 [2020-12-16]
### Added
* New keybindings
	* `j` for going up and `k` for going down
	* `Page Up` and `Page Down` for scrolling by entire pages
	* `G` for going to the end and `g` for going at the very top

* Scrolling using mouse
* Add a `less` like program as an example
* Add a complete line numbering system
* A `Pager` struct for configuration with builder pattern configuration functions
* A new type `PagerMutex` which is just an alias for `Arc<Mutex<Pager>>`
* Error handling/propagating using `thiserror`

### Changed
* `tokio_updaring` and `async_std_updating` now take a `PagerMutex`
* `page_all` now takes a `Pager` instead of String

## v1.0.2 [2020-12-04]
### Bug fixes
* Fix compilation errors

## v1.0.1 [2020-12-04]
### Bug fixes
* Fix bug where `minus` does not redraw with terminal resize

## v1.0.0 [2020-12-03]
* Added all features
* Fixed all bugs
