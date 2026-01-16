#![doc = include_str!("../README.md")]
//!

#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs, clippy::unimplemented)]

use std::{
    collections::HashMap,
    fmt::Debug,
    hash::Hash,
    sync::{Arc, RwLock},
};

use futures::{TryStream, lock::Mutex, stream::Peekable, task::AtomicWaker};

mod subed;
pub use subed::{Item, Subed};

mod subable;
pub use subable::Subable;

struct Inner<S: TryStream, T: Topic> {
    wakers: RwLock<HashMap<T, Arc<AtomicWaker>>>,
    stream: Mutex<Peekable<S>>,
}

/// The _topic_ that will be used to route items to a specific subscriber.
pub trait Topic: Debug + Clone + Hash + Eq {
    /// The type of the items in the [`Subable`] stream.
    type Item;

    /// Identify the topic from the item type.
    fn topic(item: &Self::Item) -> Self;
}
