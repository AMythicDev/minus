//! Provides [DataStore] for storing variable data
//!
//! While building complex applications with minus, it is sometimes necessory to store variable data
//! that should be available while configuring minus. Lets say you want to know when the user presses `G`
//! and maybe use this information to call some other part of your application, you may be tempted use
//! something like this
//! ```rust,compile_fail
//! # use minus::input::{HashedEventRegister, InputEvent};
//!
//! let mut pressed_G = false;
//!
//! let mut event_register = HashedEventRegister::default();
//! event_register.add_key_events(&["G"], |_, ps|{
//!     pressed_G = true;
//!     // ...
//! #   InputEvent::NextMatch
//! });
//! ```
//! But this will cause a compilation error and that's quite obvious. The `pressed_G` function may go out of scope
//! whenever the function goes out of scope, but `add_key_events` requies all variables to be `'static` i.e. they
//! should'nt go out of scope till these callbacks go out of scope. 
//!
//! You can use other workaround for this problem but minus provides the [DataStore] type that can easily solve this
//! problem for you. Lets see how you can solve this easily with the help of [DataStore]
//! ```rust
//! # use minus::input::{HashedEventRegister, InputEvent};
//!
//! let mut event_register = HashedEventRegister::default();
//! event_register.add_key_events(&["G"], |_, ps|{
//!     ps.store.push("pressed_G", Box::new(true));
//!     // ...
//! #   InputEvent::NextMatch
//! });
//! ```
//! Out of everything the most important line is this one.
//! ```text
//!     ps.store.push("pressed_G", Box::new(true));
//! ```
//! Lets talk about it in detail. 
//!
//! Any data that you store in the store must be associated with a name. In this case, the first argument `pressed_G` 
//! is that name. It can be anything that can be converted to a [String]. In other words, it must implement
//! `Into<String>`. The name will be useful when you want to access that data again.
//! The next argument of course is the value
//! that we want to store. The data must be wrapped inside a [Box]. This might seem a bit annoying but it allows the
//! [DataStore] to store all sorts of data inside it. Do note that whatever data you store must implement [Hash],
//! [Send], [Sync] and must have a `'static` lifetime.
//!
//! But how do you access that data in other parts of your application? Well, for this you need to do some

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use parking_lot::Mutex;

pub trait Hashable {}

impl<T> Hashable for T where T: Hash + Send + Sync + 'static {
}


pub trait HashableType: Hashable + Send + Sync + 'static {

}

// pub trait HashableTypeExit: Sized + DynClone {
//     fn into_box(&self) -> Box<dyn HashableType>;
// }
//
// impl HashableTypeExit for &dyn HashableType {
//     fn into_box(&self) -> Box<dyn HashableType> {
//         Box::new(*self.clone())
//     }
// }


pub struct DataStore(Arc<Mutex<HashMap<String, Arc<dyn HashableType>>>>);

impl DataStore {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(HashMap::new())))
    }

    pub fn push(&self, k: impl Into<String>, v: Box<dyn HashableType>) {
        let mut map = self.0.lock();

        let v = *v.as_ref();


        let v = Arc::from(v);
        map.insert(k.into(), v);
    }

    pub fn remove(&self, k: impl Into<String>) {
        let mut map = self.0.lock();
        map.remove(&k.into());
    }

    pub fn get<K>(&self, k: impl Into<String>) -> Option<Box<dyn HashableType>>
    where K: Into<String> {
        let map = self.0.lock();
        map.get(&k.into()).map(|v| {
            v.data.clone().as_ref()
            // let t = v.as_ref().clone();
            // t.into_box()
        })
    }
}

impl Default for DataStore {
    fn default() -> Self {
        let ds = Self::new();
        ds
    }
}
