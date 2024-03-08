//! Provides the [`HashedEventRegister`] and related items
//!
//! This module holds the [`HashedEventRegister`] which is a [`HashMap`] that stores events and their associated
//! callbacks. When the user does an action on the terminal, the event is scanned and matched against this register.
//! If their is a match related to that event, the associated callback is called

use super::{InputClassifier, InputEvent};
use crate::PagerState;
use crossterm::event::{Event, MouseEvent};
use std::{
    collections::hash_map::RandomState, collections::HashMap, hash::BuildHasher, hash::Hash,
    sync::Arc,
};

/// A convenient type for the return type of [`HashedEventRegister::get`]
type EventReturnType = Arc<dyn Fn(Event, &PagerState) -> InputEvent + Send + Sync>;

// //////////////////////////////
// EVENTWRAPPER TYPE
// //////////////////////////////

#[derive(Clone, Eq)]
enum EventWrapper {
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
pub struct HashedEventRegister<S>(HashMap<EventWrapper, EventReturnType, S>);

impl HashedEventRegister<RandomState> {
    /// Create a new [HashedEventRegister] with the default hasher
    #[must_use]
    pub fn with_default_hasher() -> Self {
        Self::new(RandomState::new())
    }
}

impl Default for HashedEventRegister<RandomState> {
    /// Create a new [HashedEventRegister] with the default hasher and insert the default bindings
    fn default() -> Self {
        let mut event_register = Self::new(RandomState::new());
        super::generate_default_bindings(&mut event_register);
        event_register
    }
}

impl<S> InputClassifier for HashedEventRegister<S>
where
    S: BuildHasher,
{
    fn classify_input(&self, ev: Event, ps: &crate::PagerState) -> Option<InputEvent> {
        self.get(&ev).map(|c| c(ev, ps))
    }
}

// ####################
// GENERAL FUNCTIONS
// ####################
impl<S> HashedEventRegister<S>
where
    S: BuildHasher,
{
    /// Create a new HashedEventRegister with the Hasher `s`
    pub fn new(s: S) -> Self {
        Self(HashMap::with_hasher(s))
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
    pub fn insert_wild_event_matcher(
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
    ///
    /// # Example
    /// These are from the original sources
    /// ```
    /// use minus::input::{InputEvent, HashedEventRegister, crossterm_event::Event};
    ///
    /// let mut input_register = HashedEventRegister::default();
    ///
    /// input_register.add_resize_event(|ev, _| {
    ///     let (cols, rows) = if let Event::Resize(cols, rows) = ev {
    ///         (cols, rows)
    ///     } else {
    ///         unreachable!();
    ///     };
    ///     InputEvent::UpdateTermArea(cols as usize, rows as usize)
    /// });
    /// ```
    pub fn add_resize_event(
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
    pub fn remove_resize_event(&mut self) {
        self.0
            .remove(&EventWrapper::ExactMatchEvent(Event::Resize(0, 0)));
    }
}

// ###############################
// KEYBOARD SPECIFIC FUNCTIONS
// ###############################
impl<S> HashedEventRegister<S>
where
    S: BuildHasher,
{
    /// Add all elemnts of `desc` as key bindings that minus should respond to with the callback `cb`
    ///
    /// You should prefer using the [add_key_events_checked](HashedEventRegister::add_key_events_checked)
    /// over this one.
    ///
    /// # Example
    /// ```
    /// use minus::input::{InputEvent, HashedEventRegister, crossterm_event};
    ///
    /// let mut input_register = HashedEventRegister::default();
    ///
    /// input_register.add_key_events(&["down"], |_, ps| {
    ///     InputEvent::UpdateUpperMark(ps.upper_mark.saturating_sub(1))
    /// });
    /// ```
    pub fn add_key_events(
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

    /// Add all elemnts of `desc` as key bindings that minus should respond to with the callback `cb`
    ///
    /// This will panic if you the keybinding has been previously defined, unless the `remap`
    /// is set to true. This helps preventing accidental overrides of your keybindings.
    ///
    /// Prefer using this over [add_key_events](HashedEventRegister::add_key_events).
    ///
    /// # Example
    /// ```should_panic
    /// use minus::input::{InputEvent, HashedEventRegister, crossterm_event};
    ///
    /// let mut input_register = HashedEventRegister::default();
    ///
    /// input_register.add_key_events_checked(&["down"], |_, ps| {
    ///     InputEvent::UpdateUpperMark(ps.upper_mark.saturating_sub(1))
    /// }, false);
    /// ```
    pub fn add_key_events_checked(
        &mut self,
        desc: &[&str],
        cb: impl Fn(Event, &PagerState) -> InputEvent + Send + Sync + 'static,
        remap: bool,
    ) {
        let v = Arc::new(cb);
        for k in desc {
            let def: EventWrapper =
                Event::Key(super::definitions::keydefs::parse_key_event(k)).into();
            assert!(self.0.contains_key(&def) && remap, "");
            self.0.insert(def, v.clone());
        }
    }

    /// Removes the callback associated with the all the elements of `desc`.
    ///
    /// ```
    /// use minus::input::{InputEvent, HashedEventRegister, crossterm_event};
    ///
    /// let mut input_register = HashedEventRegister::default();
    ///
    /// input_register.remove_key_events(&["down"])
    /// ```
    pub fn remove_key_events(&mut self, desc: &[&str]) {
        for k in desc {
            self.0
                .remove(&Event::Key(super::definitions::keydefs::parse_key_event(k)).into());
        }
    }
}

// ###############################
// MOUSE SPECIFIC FUNCTIONS
// ###############################
impl<S> HashedEventRegister<S>
where
    S: BuildHasher,
{
    /// Add all elemnts of `desc` as mouse bindings that minus should respond to with the callback `cb`
    ///
    /// You should prefer using the [add_mouse_events_checked](HashedEventRegister::add_mouse_events_checked)
    /// over this one.
    ///
    /// # Example
    /// ```
    /// use minus::input::{InputEvent, HashedEventRegister};
    ///
    /// let mut input_register = HashedEventRegister::default();
    ///
    /// input_register.add_mouse_events(&["scroll:down"], |_, ps| {
    ///     InputEvent::UpdateUpperMark(ps.upper_mark.saturating_sub(5))
    /// });
    /// ```
    pub fn add_mouse_events(
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

    /// Add all elemnts of `desc` as mouse bindings that minus should respond to with the callback `cb`
    ///
    /// This will panic if you the keybinding has been previously defined, unless the `remap`
    /// is set to true. This helps preventing accidental overrides of your keybindings.
    ///
    /// Prefer using this over [add_mouse_events](HashedEventRegister::add_mouse_events).
    /// # Example
    /// ```should_panic
    /// use minus::input::{InputEvent, HashedEventRegister};
    ///
    /// let mut input_register = HashedEventRegister::default();
    ///
    /// input_register.add_mouse_events_checked(&["scroll:down"], |_, ps| {
    ///     InputEvent::UpdateUpperMark(ps.upper_mark.saturating_sub(5))
    /// }, false);
    /// ```
    pub fn add_mouse_events_checked(
        &mut self,
        desc: &[&str],
        cb: impl Fn(Event, &PagerState) -> InputEvent + Send + Sync + 'static,
        remap: bool,
    ) {
        let v = Arc::new(cb);
        for k in desc {
            let def: EventWrapper =
                Event::Mouse(super::definitions::mousedefs::parse_mouse_event(k)).into();
            assert!(self.0.contains_key(&def) && remap, "");
            self.0.insert(def, v.clone());
        }
    }

    /// Removes the callback associated with the all the elements of `desc`.
    ///
    /// ```
    /// use minus::input::{InputEvent, HashedEventRegister, crossterm_event};
    ///
    /// let mut input_register = HashedEventRegister::default();
    ///
    /// input_register.remove_mouse_events(&["scroll:down"])
    /// ```
    pub fn remove_mouse_events(&mut self, mouse: &[&str]) {
        for k in mouse {
            self.0
                .remove(&Event::Mouse(super::definitions::mousedefs::parse_mouse_event(k)).into());
        }
    }
}
