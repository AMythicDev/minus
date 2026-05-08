//! Provides the [`Command`] enum and all its related implementations
//!
//! This module only declares the [Command] type. To know how they are handled internally see
//! the [`ev_handler`](super::ev_handler).

use std::fmt::Debug;

use crate::{
    ExitStrategy, LineNumbers,
    hooks::{Hook, HookCallback},
    input::{InputEvent, InputEventBoxed},
    minus_core::utils::display::AppendStyle,
};

#[cfg(feature = "search")]
use crate::search::SearchOpts;

#[derive(Debug, PartialEq, Eq)]
pub enum IoCommand {
    RedrawPrompt,
    RedrawDisplay,
    /// Append text to the screen
    ///
    /// First item corresponds to the value of unterminated lines before the text is formatted while
    /// the second value corresponds to the total number of rows before formatting.
    DrawAppendedText(usize, usize, AppendStyle),
    SetUpperMark(usize),
    #[cfg(feature = "search")]
    FetchSearchQuery,
}

/// Different events that can be encountered while the pager is running
#[non_exhaustive]
#[allow(private_interfaces)]
pub enum Command {
    // User input
    UserInput(InputEvent),

    // Data related
    AppendData(String),
    SetData(String),

    // Prompt related
    SendMessage(String),
    ShowPrompt(bool),
    SetPrompt(String),

    // Screen output configurations
    LineWrapping(bool),
    SetLineNumbers(LineNumbers),
    FollowOutput(bool),

    // Configuration options
    SetExitStrategy(ExitStrategy),
    AddExitCallback(Box<dyn FnMut() + Send + Sync + 'static>),
    AddHook(Hook, u64, HookCallback),
    RemoveHook(Hook, u64),
    #[cfg(feature = "static_output")]
    SetRunNoOverflow(bool),
    #[cfg(feature = "search")]
    IncrementalSearchCondition(Box<dyn Fn(&SearchOpts) -> bool + Send + Sync + 'static>),

    // Input
    AddKeyBinding(Vec<String>, InputEventBoxed, bool),
    RemoveKeyBinding(Vec<String>),
    AddMouseBinding(Vec<String>, InputEventBoxed, bool),
    RemoveMouseBinding(Vec<String>),

    Io(IoCommand),
}

impl PartialEq for Command {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::SetData(d1), Self::SetData(d2))
            | (Self::AppendData(d1), Self::AppendData(d2))
            | (Self::SetPrompt(d1), Self::SetPrompt(d2))
            | (Self::SendMessage(d1), Self::SendMessage(d2)) => d1 == d2,
            (Self::LineWrapping(d1), Self::LineWrapping(d2)) => d1 == d2,
            (Self::SetLineNumbers(d1), Self::SetLineNumbers(d2)) => d1 == d2,
            (Self::ShowPrompt(d1), Self::ShowPrompt(d2)) => d1 == d2,
            (Self::SetExitStrategy(d1), Self::SetExitStrategy(d2)) => d1 == d2,
            #[cfg(feature = "static_output")]
            (Self::SetRunNoOverflow(d1), Self::SetRunNoOverflow(d2)) => d1 == d2,
            (Self::AddExitCallback(_), Self::AddExitCallback(_))
            | (Self::AddHook(..), Self::AddHook(..)) => true,
            (Self::RemoveHook(h1, id1), Self::RemoveHook(h2, id2)) => h1 == h2 && id1 == id2,
            #[cfg(feature = "search")]
            (Self::IncrementalSearchCondition(_), Self::IncrementalSearchCondition(_)) => true,
            (Self::AddKeyBinding(a_desc, _, a_remap), Self::AddKeyBinding(b_desc, _, b_remap))
            | (
                Self::AddMouseBinding(a_desc, _, a_remap),
                Self::AddMouseBinding(b_desc, _, b_remap),
            ) => a_desc == b_desc && a_remap == b_remap,
            (Self::RemoveKeyBinding(a), Self::RemoveKeyBinding(b))
            | (Self::RemoveMouseBinding(a), Self::RemoveMouseBinding(b)) => a == b,
            (Self::Io(a), Self::Io(b)) => a == b,
            _ => false,
        }
    }
}

impl Debug for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SetData(text) => write!(f, "SetData({text:?})"),
            Self::AppendData(text) => write!(f, "AppendData({text:?})"),
            Self::SetPrompt(text) => write!(f, "SetPrompt({text:?})"),
            Self::SendMessage(text) => write!(f, "SendMessage({text:?})"),
            Self::SetLineNumbers(ln) => write!(f, "SetLineNumbers({ln:?})"),
            Self::LineWrapping(lw) => write!(f, "LineWrapping({lw:?})"),
            Self::SetExitStrategy(es) => write!(f, "SetExitStrategy({es:?})"),
            Self::ShowPrompt(show) => write!(f, "ShowPrompt({show:?})"),
            #[cfg(feature = "search")]
            Self::IncrementalSearchCondition(_) => write!(f, "IncrementalSearchCondition"),
            Self::AddExitCallback(_) => write!(f, "AddExitCallback"),
            Self::AddHook(h, id, _) => write!(f, "AddHook({h:?}, {id})"),
            Self::RemoveHook(h, id) => write!(f, "RemoveHook({h:?}, {id})"),
            #[cfg(feature = "static_output")]
            Self::SetRunNoOverflow(val) => write!(f, "SetRunNoOverflow({val:?})"),
            Self::UserInput(input) => write!(f, "UserInput({input:?})"),
            Self::FollowOutput(follow_output) => write!(f, "FollowOutput({follow_output:?})"),
            Self::AddKeyBinding(desc, _, remap) => write!(f, "AddKeyBinding({desc:?}, {remap})"),
            Self::AddMouseBinding(desc, _, remap) => {
                write!(f, "AddMouseBinding({desc:?}, {remap})")
            }
            Self::RemoveKeyBinding(desc) => write!(f, "RemoveKeyBinding({desc:?})"),
            Self::RemoveMouseBinding(desc) => write!(f, "RemoveMouseBinding({desc:?})"),
            Self::Io(c) => write!(f, "Internal({c:?})"),
        }
    }
}
