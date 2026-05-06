//! Manages and runs callbacks for events happening in minus.

use std::collections::HashMap;

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
    PostPagerExit,
}

/// Unique ID for a callback
#[derive(Debug, Eq, PartialEq, Hash, Clone, Copy, Default)]
pub struct CallbackId(u64);

/// A callback that can be executed on a hook
pub type HookCallback = Box<dyn FnMut() + Send + Sync + 'static>;

/// Stores callbacks for all hooks
#[derive(Default)]
pub struct Hooks {
    hooks: HashMap<Hook, Vec<(CallbackId, HookCallback)>>,
    next_id: u64,
}

impl Hooks {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_callback(&mut self, hook: Hook, cb: HookCallback) -> CallbackId {
        let id = CallbackId(self.next_id);
        self.next_id += 1;
        self.hooks.entry(hook).or_default().push((id, cb));
        id
    }

    pub fn remove_callback(&mut self, hook: Hook, id: CallbackId) -> bool {
        if let Some(cbs) = self.hooks.get_mut(&hook) {
            if let Some(pos) = cbs.iter().position(|(cb_id, _)| *cb_id == id) {
                cbs.remove(pos);
                return true;
            }
        }
        false
    }

    pub fn run_hooks(&mut self, hook: Hook) {
        if let Some(cbs) = self.hooks.get_mut(&hook) {
            for (_, cb) in cbs {
                cb();
            }
        }
    }
}
