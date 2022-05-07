use super::{InputClassifier, InputEvent, PagerState};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use std::{collections::hash_map::RandomState, collections::HashMap, hash::BuildHasher};

pub type HashedEventRegister<S> =
    HashMap<Event, Box<dyn Fn(Event, &PagerState) -> InputEvent + Send + Sync>, S>;

// pub enum Binding {
//     KeyBind()
// }

mod keyevent;

pub fn gen_default_event_matcher() -> HashedEventRegister<RandomState> {
    let mut map = HashedEventRegister::new();

    map.insert(
        Event::Key(KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::NONE,
        }),
        Box::new(|_, _| InputEvent::Exit),
    );
    map
}

impl<S> InputClassifier for HashedEventRegister<S>
where
    S: BuildHasher,
{
    fn classify_input(&self, ev: Event, ps: &crate::PagerState) -> Option<InputEvent> {
        self.get(&ev).map(|c| c(ev, ps))
    }
}
