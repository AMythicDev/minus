# Changelog
This file documents all changes made to the project and is updated before each release.

## v5.6.1 [2024-03-31]
### Fixed
* [#133](https://github.com/pull/133): Updated the `flake.nix` file.
* [#136](https://github.com/pull/136): Fixed weird drawing issues when minus is ran multiple times in a row.

### Docs
* [#134](https://github.com/pull/134): Improve documentation for `error` and `input` modules.

## v5.6.0 [2024-03-05]
### Fixed
* [cd2b92](https://github.com/arijit79/minus/commits/cd2b92): Fixed bug where screen jumps to 2nd match when first match is present on the same line where upper mark is present
* [#120](https://github.com/arijit79/minus/issues/120), [#121](https://github.com/arijit79/minus/issues/121): Fixed screen flickering on WSL/Windows Terminal
* [3493e76d](https://github.com/commits/3493e76d): Fixed weird search jumps due to updating upper_mark in incremental search

# Performance
* Include a bunch of changes targeting text appending.
* Introduced `minus_core::utils::digits` which uses faster algorithm to calculate digits in line numbers
* [67772fc](https://github.com/commits/67772fc): Improve performance by Only reformatting the entire text when linen numbers are turned on
* Introduce caching for frequently used analysis about the data like like total line count, total row count, length of the longest line etc

### Docs
* [#117](https://github.com/pull/117), [#118](https://github.com/pull/118), [#123](https://github.com/pull/123), [#124](https://github.com/pull/124), [#132](https://github.com/pull/132): improvements to documentation in README and lib.rs
* [53922d348](https://github.com/commits/53922d348): Overhaul docs for input module

### Added
* [#114](https://github.com/arijit79/minus/pull/114): Added follow mode
* [#65](https://github.com/arijit79/minus/pull/65): Implement horizontal scrolling
* [#130](https://github.com/arijit79/minus/pull/130): Added new APIs to the screen module.
* [36da3ca6](https://github.com/commits/36da3ca6):  Added `add_key_events_checked` and `add_mouse_events_checked` functions in `HashedEventRegister` to prevent accidentally overriding already present bindings.
* [#112](https://github.com/arijit79/minus/pull/112): Added all the dependencies required to develop minus in the `flake.nix` file.

### Removed
* Removed the `initial_formatted_lines` and `initial_upper_mark` fields from `IncrementalSearchOpts`

### Changed
* [#130](https://github.com/arijit79/minus/pull/130): Merge the text module  with screen module.
* [cb991f1ac8](https://github.com/commits/cb991f1ac8): Use a better colorscheme for the prompt
* [4611a13](https://github.com/commits/4611a13): Use a Vec style type for `lines_to_row_map` field in `PagerState` which yields better
  performance

## v5.5.1 [2023-12-05]
### Fixed
* Version information in README
* Static mode specific functionality not working due to incorrect placing of `RUNMODE` setting code. ([#110](https://github.com/arijit79/minus/pull/110))

## v5.5.0 [2023-11-27]
### Fixed
* Prompt status not being updated when a new search is activated. (#102)
* Improved consistency of search highlighting.
* Fix a panic when using `delete` key when defining keybindings using the new input mapping method.
* Fix a bug where search character shown at prompt is `/` regardless of search direction.

### Added
* Added `Pager::show_prompt()` function to control whether to show prompt.
* Introudce s `state::SearchState` type to hold data specific to seaching only.
* Added a `PagerState::search_state` field to get the `SearchState` field.
* Introduce a feature to disable minus's method of search highlighting and get exact/accurate highlighting of the search matches.
* Added incremental searching support
* Introduced more keybindings on the search prompt. (#104)
* Added a `Pager::set_incremental_search_condition` function to set condition for running incremental search.
* Introduce `SearchOpts` and `IncrementalSearchOpts` types so that applications can control when to run incremental search.
* Added an experimental `screen` module and `Screen` type to get analysis of the data present in minus.

### Changed
* Make the `state` module public.
* Make the `search` module public.
* Rename `events` module to `commands`
* Changed the name of `Event` enum to `Command`
* Moved the `PagerState::lines` and `PagerState::formatted_lines` fields into the `Screen` type.
* Make the `RunMode` type public.

### Deprecated
* `InputEvent::NextMatch` has been deprecated in favour of `InputEvent::MoveToNextMatch`
* `InputEvent::PrevMatch` has been deprecated in favour of `InputEvent::MoveToPrevMatch`
* `PagerState::search_mode` hhas been deprecated in favour of `SearchState::search_mode`

### Removed
* Remove `futures-lite` from dependencies.

## v5.4.0 [2023-07-27]
### Fixed
* Unexpected line break where number of digits in line number increase (#66)
* Pressing `[n]G` takes to nth row on screen rather than the nth line in the actual text (#99)

### Changed
* `HashedInputRegister::default()` now inserts the default bindings before returning the `HashedInputRegister`.
    This better aligns with the docs.
* Massively simplify all the text-related functions and add better docs to all of them. (#97)

### Added
* `HashedInputRegister::new()` is now publicly available
* Added `HashedInputRegister::with_default_hasher()` to create a `HashedInputRegister` with the default hasher.
* Added a `Dockerfile` to build/test minus in isolated environment.

## v5.3.1 [2023-05-02]
### Fixed
* Passing `false` to PagerState::set_run_no_overflow() causes a full
  pager to start where in reality passing `true` should lead to this
  action. (#94)
* If a full pager isn't started in static output mode and another pager
  is started after the first one, minus would make a panic as the
  `RUNMODE` variable doesn't get reset. This patch also fixes this bug. (#94)

## v5.3.0 [2023-04-10]
### Added
* Add a new native API for defining key/mouse bindings (#93)

### Fixed
* Fixed a bug where screen would get broken if user scrolls with the mouse while simultaneously data in also being
  inserted onto the screen (#92)

### Misc
* Removed dependency from [crossbeam-utils](https://crates.io/crates/crossbeam-utils)

## v5.2.1 [2023-03-22]
### Fixed
* Fixed bug where copy pasting the default `DefaultInputClassifier` template causes a compile error(#90)
* Fixed clippy warnings

## v5.2.0 [2023-03-01]
### Added
* Added `AppendStyle` and `AppendProps` enums in the new `minus_core::utils::text` to control the append behaviour and 
  properties related to each append operation

### Changes
* Moved `make_append_str` function and related tests into new `minus_core::utils::text` module
* Bumped MSRV to Rust 1.67

### Fixed
* Fixed a bug where line number paddings didn't update properly when their number of digits changed.
* Fixed bug where screen would get redrawn after processing exit callbacks

## v5.1.0 [2022-11-10]
### Added
* Added new events to jump to next or previous nth search result.
  In the default keybindings these are mapped as:-
    - '10n' would go forward to the 10th search result from the current position
    - '10p' would go backward to the 10th search result from the current position

### Changed
* Draw the screen intelligently.
  On movement, minus now redraws only those lines of the screen that need to be updated rather than redrawing the entire
  screen. (#84)
* `parking_lot` has been added as a dependency and all sync locks from std have been migrated to `parking_lot`'s types'

### Fixed
* Fixed regression while quitting in static paging (#79)
* Fixed a bug where the screen would get redrawn after quitting in dynamic paging (#79)
* Fined bug where `dynamic_paging()` and `page_all()` functions can't be used multiple times. (#88)

### Removed
* Removed all usages of `NextMatch` and 'PrevMatch'. These types may be removed in future major releases

## v5.0.5 [2022-08-20]
### Fixed
* Fixed regression in quitting in v5.0.4 and cleanup issues from v5.0.0 (#79)

## v5.0.4 [2022-07-31]
### Added
* Added dependency on [crossbeam-utils](https://crates.io/crates/crossbeam-utils). 
  This allows us to use scoped threads feature provided by it.

### Changed
* A custom panic hook has been injected to display panic messages more clearly when a panic happens (#77)
* minus now displays the name of the binary in the prompt rather than the just the text `minus`

### Docs
* Improved the crate level documentation including fix for not displaying the optional
  `[n]` prefix for movement keys

## v5.0.3 [2022-05-3]
### Fixed
* Fix line numbers not wrapping properly in some cases of wrapping.
* Fix search bug where the highlighted line is off by one

### Docs
* The README now displays the optional number prefix before all movement keys

## v5.0.2 [2022-04-27]
### Added
* The prompt line now shows the number of search matches found and the current search match number which
  is in focus

### Changed
- Line Numbers are displayed only on the first wrapped row of each line.

  This decreases the clutter on the line number column especially on text which span multiple lines.
  
- Line Numbers are now padded by about 5 spaces. This makes the line numbers not get tightly packed with the left edge of the terminal.

### Fixed
- Fixed bug when appending complex sets of text, a wrong value of `unterminated` got calculated which 
  caused junk text to appended to the `PagerState::formatted_lines` and also to be displayed on the terminal.

- Fixed mouse scroll wheel not scrolling through the screen.

   This occurred because a of a previous patch which removed the line that enabled the mouse events to be captured.
 
* Fix panic when the search term gets changed

  This occurred due to the `search_idx` not being repopulated when a new search is activated.

## v5.0.1 [2022-03-20]
* Fixed extremely high CPU usage while running caused due to calling `Receiver::try_recv()` rather than
  `Receiver::recv()`(#60)
* Fixed another performance bug which was due to calling `event::poll` every 10ms. The poll duration
  was increased to 100ms without any loss of responsiveness (9f7dace34)
* Changed initialization of some fields in `PagerState` tp preallocate memory for them. This reduces
  the number of allocations that need to be made when the pager just starts.
* Bring back and improve the `Justfile`
* Fix bugs related to duplicate line numbering and wrong search lines to be matched (#62)

## v5.0.0 [2022-03-15]
* Added `dynamic_paging` function to enable asynchronous paging.

  This is the unification of the previous `tokio_lib` and `async_std_lib` features.
  minus no longer depends on `tokio` or `async_std` directly and requires end-application to
  bring in these libs as dependency. **This makes minus completely runtime agnostic**
  
* minus can now be called from a OS thread using 
  [`threads`](https://doc.rust-lang.org/std/thread/index.html). 
  See example in [README](./README.md#threads)
  
* Applications should call `dynamic_paging` on s separate non-blocking thread like 
  [`tokio::task::spawn_blocking()`](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html)
  or [`threads`](https://doc.rust-lang.org/std/thread/index.html).

* Use channels for communication
  
  * This allows minus to exactly know when data is changed and do various optimizations on it's
  * Added [`crossbeam_channels`](https://crates.io/crates/crossbeam_channels) as dependency.

* Store the current run mode as static value

  * The `RUNMODE` static item tells minus whether it is running in static mode or asynchronous mode
  * Added `once_cell` as a dependency to store the above value in static scope.
  
* Added feature to scroll through more than one line
  * Prefixing any of the movement keys with a number will move the screen up or down to that many lines. 
    For example `10j` will take the view 10 lines down.
  * Similarly jump to specific line by prefixing `G` with a number. For example `15G` will take you to the 
    15th line of the data.
    
* Searching through text with lots of ansi sequence inside it will no longer break the search
  Previously this case would cause the search matcher to not match it and move to the next one
  (#57)
    
* Added a `PagerState` struct to store and share internal data. It is made public, along with some of its
fields so that it can be used to implement `InputClassifier` trait for applications that want to modify the
default keybindings

* Renamed `AlternatePagingError` to `MinusError`.
  This was to shorten the number of letters one needs to type

* `Pager::set_run_no_overflow(bool)` function is now only available in `static_output` mode.
  In dynamic mode, this was more of a wasted feature as it didn't solve the issue that it should have done
  and was a burden to maintain it.

* Changed function signature of `InputClassifier::handle_input` to `handle_input(&self, ev: Event, ps: PagerState) -> Some(InputEvent)`.
  The `handle_input()` function cared about a lot of things and passing everything as a parameter
  was really tedious. This also caused a breaking change whenever a new parameter was added

* Changed function signature of `Pager::new` to `new() -> Pager`. It previously used to return a        
  `Result<Pager, TermError>`.

* Use threads even in static paging mode. Although mutating the `Pager`s data won't reflect any changes in  
  static mode.
  
* Replaced `tokio-no-overflow` example with `static-no-overflow` function. This is because the 
  `Pager::run_no_overflow` function is only available in `static_output`feature.
  
* All implemented functions on `Pager` except `Pager::new` will return a `Result<(), MinusError>`
  because the communication with the pager may fail if the pager has quit early on.
  
* Applications should spawn `dynamic_paging` by themselves. For example on tokio, this would be
  ```rust
      use tokio::{task::spawn_blocking, join}
      use minus::{Pager, dynamic_paging};

      let pager = Pager::new();

      join!(
          spawn_blocking(move || dynamic_paging(pager)
          // ....
      );
   ```

* Renamed `Pager::set_input_handler` to `Pager::set_input_classifier`.
* Removed `tokio_updating`  and `async_std_updating` in favour of the unified `dynamic_paging` function.
* Removed `PagerMutex` type.
* Removed `tokio`, `async-std` and `async-mutex` from dependencies.
* Removed `Pager::finish` function.
* Removed `Pager::end_data_stream` function.
  
  This was only required for running in dynamic mode with run no overflow on. With deprecation of this 
  feature we no longer need this function
  
* Removed `static_long` example.
* Removed `PageAllError` from `static_pager` and `errors` modules.

## v4.0.5 [2022-1-8]
### Fixed
* Fix all clippy warnings

## v4.0.4 [2022-1-8]
### Fixed
* Fixed a bug where `q` key didn't quit the pager in `PagerQuit` exit strategy (#56)

## v4.0.3 [2021-11-28]
### Fixed
* Introduce a bunch of performance improvements

## v4.0.2 [2021-10-10]
### Added
* Added the `minus` logo in README

### Fixed
* Fixed panic when more than 65,535 lines were searched at once

## v4.0.1 [2021-08-18]
### Fixed
* Fixed bug where selecting with mouse didn't select anything on the output

## v4.0.0 [2021-07-16]
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
* The Pager API has changed from a builder pattern to a more programmatic pattern like

  ```rust
  use minus::Pager;

  let mut pager = Pager::new().unwrap();
  pager.set_prompt("Example")
  ```

* Line Numbers displayed are now bold and have a little more left padding
* Next and Previous now simply move the display to the match line number rather than moving the cursor
* The `utils` module file has have been transformed into a directory with it's own separate

### Fixed
* Prevent panic if invalid regex is given during search
* Fix run\_no\_overflow for static pager (#43)
  
   Previously, this setting had no effect if paging static output, due to an if condition in
   `static_pager.rs` which did not consider the setting. This commit makes
   this setting behave as expected. (@tomstoneham) 

* The cursor is hidden as soon as the search query entry is complete.
* Fix where color outputs get distorted after search matches

## v3.4.0 [2021-05-26]
### Added
* u and d keys for half page scrolling

### Fixed
* The reverse direction of j and k keys

## v3.3.3 [2021-04-29]
### Fixed
* Fixed warnings issued by clippy

## v3.3.2 [2021-04-28]
### Added
* Add docs for InputHandler and DefaultInputHandler

## v3.3.1 [2021-03-13]
### Fixed
* Documentation build failure

## v3.3.0 [2021-03-5]
### Added
* A trait to coustomize the default keybindings

### Fixed
* Fixed bug where cursor movement stops at last search instance

## v3.2.0 [2021-02-22]
### Added
* A new function to signal the end of data to the pager

### Changed
* Page cleanup when the pager needs to quit

## v3.1.2 [2021-02-16]
### Bug fixes
* Fixed bug where text with large amount of lines where not displayed
* Fixed bug last line of text was not displayed when the string is pushed with multiple lines

## v3.1.1 [2021-02-5]
### Documentation Fixes
* Added information about the backward search feature in README

### Bug fixes
* Removed the unstable `spin\_loop` function

## v3.1.0 [2021-01-14]
### Added
* Backward searching

### Bug fixes
* Fix bug where cursor is placed in wrong position if upper\_mark is not 1

## v3.0.2 [2021-01-12]
### Bug Fixes
* If two consecutive searches are done in a single session, the previous search
highlghts are not removed
* If the same search query is called twice, the matches are repeated

### Documentation Fixes
* Add info about `search` feature
* Add info about `searchable` field and `set_searchable` soft-deprecation

## v3.0.1 [2021-01-10]
### Documentation Fixes
* Fix README examples

## v3.0.0 [2021-01-10]
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
