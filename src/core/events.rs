//! Provides the [`Event`] enum and all its related implementations
use std::fmt::Debug;

use crate::{
    input::{InputClassifier, InputEvent},
    ExitStrategy, LineNumbers,
};

/// Different events that can be encountered while the pager is running
pub enum Event {
    AppendData(String),
    SetData(String),
    UserInput(InputEvent),
    SetPrompt(String),
    SendMessage(String),
    SetLineNumbers(LineNumbers),
    SetExitStrategy(ExitStrategy),
    SetInputClassifier(Box<dyn InputClassifier + Send + Sync + 'static>),
    AddExitCallback(Box<dyn FnMut() + Send + Sync + 'static>),
    #[cfg(feature = "static_output")]
    SetRunNoOverflow(bool),
}

impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::SetData(d1), Self::SetData(d2))
            | (Self::AppendData(d1), Self::AppendData(d2))
            | (Self::SetPrompt(d1), Self::SetPrompt(d2))
            | (Self::SendMessage(d1), Self::SendMessage(d2)) => d1 == d2,
            (Self::SetLineNumbers(d1), Self::SetLineNumbers(d2)) => d1 == d2,
            (Self::SetExitStrategy(d1), Self::SetExitStrategy(d2)) => d1 == d2,
            #[cfg(feature = "static_output")]
            (Self::SetRunNoOverflow(d1), Self::SetRunNoOverflow(d2)) => d1 == d2,
            (Self::SetInputClassifier(_), Self::SetInputClassifier(_))
            | (Self::AddExitCallback(_), Self::AddExitCallback(_)) => true,
            _ => false,
        }
    }
}

impl Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SetData(text) => write!(f, "SetData({:?})", text),
            Self::AppendData(text) => write!(f, "AppendData({:?})", text),
            Self::SetPrompt(text) => write!(f, "SetPrompt({:?})", text),
            Self::SendMessage(text) => write!(f, "SendMessage({:?})", text),
            Self::SetLineNumbers(ln) => write!(f, "SetLineNumbers({:?})", ln),
            Self::SetExitStrategy(es) => write!(f, "SetExitStrategy({:?})", es),
            Self::SetInputClassifier(_) => write!(f, "SetInputClassifier"),
            Self::AddExitCallback(_) => write!(f, "AddExitCallback"),
            #[cfg(feature = "static_output")]
            Self::SetRunNoOverflow(val) => write!(f, "SetRunNoOverflow({:?})", val),
            Self::UserInput(input) => write!(f, "UserInput({:?})", input),
        }
    }
}

impl Event {
    pub(crate) const fn is_exit_event(&self) -> bool {
        matches!(self, Self::UserInput(InputEvent::Exit))
    }

    #[cfg(feature = "dynamic_output")]
    pub(crate) const fn required_immidiate_screen_update(&self) -> bool {
        matches!(
            self,
            Self::SetData(_) | Self::SetPrompt(_) | Self::SendMessage(_) | Self::UserInput(_)
        )
    }
}
