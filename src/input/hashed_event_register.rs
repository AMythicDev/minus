//! Provides the [`HashedEventRegister`] and related items
//!
//! This module holds the [`HashedEventRegister`] which is a [`HashMap`] that stores events and
//! their associated callbacks. When the user does an action on the terminal, the event is scanned
//! and matched against this register. If their is a match related to that event, the associated
//! callback is called

use super::InputEvent;
use crate::PagerState;
use crossterm::event::{Event, MouseEvent};
use std::{collections::HashMap, collections::hash_map::RandomState, hash::Hash, sync::Arc};

/// A convenient type for the return type of [`HashedEventRegister::get`]
type EventReturnType = Arc<dyn Fn(Event, &PagerState) -> InputEvent + Send + Sync>;

// //////////////////////////////
// EVENTWRAPPER TYPE
// //////////////////////////////

#[derive(Clone, Debug, Eq)]
pub enum EventWrapper {
    ExactMatchEvent(Event),
    WildEvent,
}

impl From<Event> for EventWrapper {
    fn from(e: Event) -> Self {
        Self::ExactMatchEvent(e)
    }
}

impl From<&Event> for EventWrapper {
    fn from(e: &Event) -> Self {
        Self::ExactMatchEvent(e.clone())
    }
}

impl PartialEq for EventWrapper {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::ExactMatchEvent(Event::Mouse(MouseEvent {
                    kind, modifiers, ..
                })),
                Self::ExactMatchEvent(Event::Mouse(MouseEvent {
                    kind: o_kind,
                    modifiers: o_modifiers,
                    ..
                })),
            ) => kind == o_kind && modifiers == o_modifiers,
            (
                Self::ExactMatchEvent(Event::Resize(..)),
                Self::ExactMatchEvent(Event::Resize(..)),
            )
            | (Self::WildEvent, Self::WildEvent) => true,
            (Self::ExactMatchEvent(ev), Self::ExactMatchEvent(o_ev)) => ev == o_ev,
            _ => false,
        }
    }
}

impl Hash for EventWrapper {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let tag = std::mem::discriminant(self);
        tag.hash(state);
        match self {
            Self::ExactMatchEvent(Event::Mouse(MouseEvent {
                kind, modifiers, ..
            })) => {
                kind.hash(state);
                modifiers.hash(state);
            }
            Self::WildEvent | Self::ExactMatchEvent(Event::Resize(..)) => {}
            Self::ExactMatchEvent(v) => {
                v.hash(state);
            }
        }
    }
}

// /////////////////////////////////////////////////
// HASHED EVENT REGISTER TYPE AND ITS APIs
// ////////////////////////////////////////////////

/// A hash store for events and it's related callback
///
/// Each item is a key value pair, where the key is a event and it's value is a callback. When a
/// event occurs, it is matched inside and when the related match is found, it's related callback
/// is called.
pub struct HashedEventRegister(HashMap<EventWrapper, EventReturnType, RandomState>);

impl Default for HashedEventRegister {
    /// Create a new [HashedEventRegister] with the default hasher and insert the default bindings
    fn default() -> Self {
        let mut event_register = Self::new();
        super::generate_default_bindings(&mut event_register);
        event_register
    }
}

// ####################
// GENERAL FUNCTIONS
// ####################
impl HashedEventRegister {
    /// Create a new HashedEventRegister with the Hasher `s`
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Adds a callback to handle all events that failed to match
    ///
    /// Sometimes there are bunch of keys having equal importance that should have the same
    /// callback, for instance all the numbers on the keyboard. To handle these types of scenerios
    /// this is extremely useful. This callback is called when no event matches the incoming event,
    /// then we just match whether the event is a keyboard number and perform the required action.
    ///
    /// This is also helpful when you need to do some action, like sending a message when the user
    /// presses wrong keyboard/mouse buttons.
    pub fn map_wild_event(
        &mut self,
        cb: impl Fn(Event, &PagerState) -> InputEvent + Send + Sync + 'static,
    ) {
        self.0.insert(EventWrapper::WildEvent, Arc::new(cb));
    }

    fn get(&self, k: &Event) -> Option<&EventReturnType> {
        self.0
            .get(&k.into())
            .map_or_else(|| self.0.get(&EventWrapper::WildEvent), |k| Some(k))
    }

    /// Adds a callback for handling resize events
    pub fn map_resize(
        &mut self,
        cb: impl Fn(Event, &PagerState) -> InputEvent + Send + Sync + 'static,
    ) {
        let v = Arc::new(cb);
        // The 0, 0 are present just to ensure everything compiles and they can be anything.
        // These values are never hashed or stored into the HashedEventRegister
        self.0
            .insert(EventWrapper::ExactMatchEvent(Event::Resize(0, 0)), v);
    }

    /// Removes the currently active resize event callback
    pub fn clear_resize(&mut self) {
        self.0
            .remove(&EventWrapper::ExactMatchEvent(Event::Resize(0, 0)));
    }

    /// Removes the currently active wild event callback
    pub fn clear_wild_event(&mut self) {
        self.0.remove(&EventWrapper::WildEvent);
    }

    pub(crate) fn classify_input(&self, ev: Event, ps: &crate::PagerState) -> Option<InputEvent> {
        self.get(&ev).map(|c| c(ev, ps))
    }
}

// ###############################
// KEYBOARD SPECIFIC FUNCTIONS
// ###############################
impl HashedEventRegister {
    /// Add all elemnts of `desc` as key bindings that minus should respond to with the callback `cb`
    pub fn map_keys(
        &mut self,
        desc: &[&str],
        cb: impl Fn(Event, &PagerState) -> InputEvent + Send + Sync + 'static,
    ) {
        let v = Arc::new(cb);
        for k in desc {
            self.0.insert(
                Event::Key(super::definitions::keydefs::parse_key_event(k)).into(),
                v.clone(),
            );
        }
    }

    pub fn map_keys_ev(
        &mut self,
        desc: Vec<EventWrapper>,
        cb: impl Fn(Event, &PagerState) -> InputEvent + Send + Sync + 'static,
    ) {
        let v = Arc::new(cb);
        for k in desc {
            self.0.insert(k, v.clone());
        }
    }

    /// Removes the callback associated with the all the elements of `desc`.
    pub fn clear_keys(&mut self, desc: &[EventWrapper]) {
        for k in desc {
            self.0.remove(k);
        }
    }

    /// Clear all keyboard bindings
    pub fn clear_all_keys(&mut self) {
        self.0.retain(|k, _| {
            !matches!(
                k,
                EventWrapper::ExactMatchEvent(Event::Key(..)) | EventWrapper::WildEvent
            )
        });
    }
}

// ###############################
// MOUSE SPECIFIC FUNCTIONS
// ###############################
impl HashedEventRegister {
    /// Add all elemnts of `desc` as mouse bindings that minus should respond to with the callback `cb`
    pub fn map_mouse(
        &mut self,
        desc: &[&str],
        cb: impl Fn(Event, &PagerState) -> InputEvent + Send + Sync + 'static,
    ) {
        let v = Arc::new(cb);
        for k in desc {
            self.0.insert(
                Event::Mouse(super::definitions::mousedefs::parse_mouse_event(k)).into(),
                v.clone(),
            );
        }
    }

    pub fn map_mouse_ev(
        &mut self,
        desc: Vec<EventWrapper>,
        cb: impl Fn(Event, &PagerState) -> InputEvent + Send + Sync + 'static,
    ) {
        let v = Arc::new(cb);
        for k in desc {
            self.0.insert(k, v.clone());
        }
    }

    /// Removes the callback associated with the all the elements of `desc`.
    pub fn clear_mouse(&mut self, mouse: &[EventWrapper]) {
        for k in mouse {
            self.0.remove(k);
        }
    }

    /// Clear all mouse bindings
    pub fn clear_all_mouse(&mut self) {
        self.0
            .retain(|k, _| !matches!(k, EventWrapper::ExactMatchEvent(Event::Mouse(..))));
    }
}
