//! Manages and runs callbacks for events happening in minus.
//!
//! ## Note on Thread Blacking
//!
//! Callbacks registered for hooks are run on the same thread as the pager.
//! This means that if you add a long-running task in a callback, it will block the pager
//! from rendering, scrolling and responding to events. Hence you should avoid adding
//! long-running tasks in callbacks. If you have a long running task, you should run it on a
//! separate thread.

use std::collections::HashMap;

use crate::PagerState;

/// Events that can have callbacks registered
#[derive(Debug, Eq, PartialEq, Hash, Clone, Copy)]
pub enum Hook {
    /// Fired just before the terminal UI is drawn (before switching to the alternate screen).
    PrePagerStart,
    /// Fired after the terminal UI is drawn with text.
    PostPagerStart,
    /// Fired when the user hits the end of the page
    EofReached,
    /// Fired just before the pager exits due to [`InputEvent::Exit`](crate::input::InputEvent::Exit).
    PrePagerExit,
    /// Fired after the terminal UI is cleared up and main screen is restored.
    ///
    /// For this hook, start your IDs from 2 because 1 is occupied for the
    /// [`ExitStrategy`](crate::ExitStrategy).
    PostPagerExit,
}

/// A callback that can be executed on a hook
pub type HookCallback = Box<dyn FnMut(&PagerState) + Send + Sync + 'static>;

/// Stores callbacks for all hooks
#[derive(Default)]
pub(crate) struct Hooks {
    hooks: HashMap<Hook, Vec<(u64, HookCallback)>>,
    next_id: u64,
}

impl Hooks {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn add_callback(&mut self, hook: Hook, mut id: u64, cb: HookCallback) {
        if id == 0 {
            id = self.next_id;
            self.next_id += 1;
        }

        let callbacks = self.hooks.entry(hook).or_default();
        assert!(
            !callbacks.iter().any(|(cb_id, _)| *cb_id == id),
            "Callback ID {id} already exists for hook {hook:?}"
        );
        callbacks.push((id, cb));
    }

    pub(crate) fn remove_callback(&mut self, hook: Hook, id: u64) -> bool {
        if let Some(cbs) = self.hooks.get_mut(&hook)
            && let Some(pos) = cbs.iter().position(|(cb_id, _)| *cb_id == id)
        {
            _ = cbs.remove(pos);
            return true;
        }
        false
    }

    pub(crate) fn run_hooks(&mut self, hook: Hook, pager_state: &PagerState) {
        if let Some(cbs) = self.hooks.get_mut(&hook) {
            for (_, cb) in cbs {
                cb(pager_state);
            }
        }
    }
}
