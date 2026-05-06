//! Provides the [`Command`] enum and all its related implementations
//!
//! This module only declares the [Command] type. To know how they are handled internally see
//! the [`ev_handler`](super::ev_handler).

use std::fmt::Debug;

use crate::{
    ExitStrategy, LineNumbers,
    input::{InputClassifier, InputEvent},
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
    SetInputClassifier(Box<dyn InputClassifier + Send + Sync + 'static>),
    AddExitCallback(Box<dyn FnMut() + Send + Sync + 'static>),
    #[cfg(feature = "static_output")]
    SetRunNoOverflow(bool),
    #[cfg(feature = "search")]
    IncrementalSearchCondition(Box<dyn Fn(&SearchOpts) -> bool + Send + Sync + 'static>),

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
            (Self::SetInputClassifier(_), Self::SetInputClassifier(_))
            | (Self::AddExitCallback(_), Self::AddExitCallback(_)) => true,
            #[cfg(feature = "search")]
            (Self::IncrementalSearchCondition(_), Self::IncrementalSearchCondition(_)) => true,
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
            Self::SetInputClassifier(_) => write!(f, "SetInputClassifier"),
            Self::ShowPrompt(show) => write!(f, "ShowPrompt({show:?})"),
            #[cfg(feature = "search")]
            Self::IncrementalSearchCondition(_) => write!(f, "IncrementalSearchCondition"),
            Self::AddExitCallback(_) => write!(f, "AddExitCallback"),
            #[cfg(feature = "static_output")]
            Self::SetRunNoOverflow(val) => write!(f, "SetRunNoOverflow({val:?})"),
            Self::UserInput(input) => write!(f, "UserInput({input:?})"),
            Self::FollowOutput(follow_output) => write!(f, "FollowOutput({follow_output:?})"),
            Self::Io(c) => write!(f, "Internal({c:?})"),
        }
    }
}
