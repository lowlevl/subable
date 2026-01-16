use std::{
    collections::HashMap,
    fmt::Debug,
    hash::Hash,
    sync::{Arc, RwLock},
};

use futures::{StreamExt, TryStream, lock::Mutex, stream::Peekable, task::AtomicWaker};

mod sub;
pub use sub::{Sub, Subed};

pub trait Topic: Debug + Clone + Hash + Eq {
    type From;

    fn topic(input: &Self::From) -> Self;
}

struct Inner<S: TryStream, T: Topic> {
    wakers: RwLock<HashMap<T, Arc<AtomicWaker>>>,
    stream: Mutex<Peekable<S>>,
}

pub struct Subscriber<S: TryStream, T: Topic> {
    inner: Arc<Inner<S, T>>,
}

impl<S: TryStream, T: Topic> Subscriber<S, T> {
    pub fn new(stream: S) -> Self {
        Self {
            inner: Inner {
                wakers: Default::default(),
                stream: stream.peekable().into(),
            }
            .into(),
        }
    }

    pub fn subscribe(&self, topic: T) -> Sub<S, T> {
        if self
            .inner
            .wakers
            .write()
            .unwrap()
            .insert(topic.clone(), Default::default())
            .is_some()
        {
            panic!("category already subscribed, bailing");
        }

        tracing::trace!("subscribing {topic:?}");

        Sub::new(self.inner.clone(), topic)
    }

    pub fn unsubscribe_all(&self) {
        for (_, waker) in self.inner.wakers.write().unwrap().drain() {
            // Wake all tasks, that will subsequently return `None`
            waker.wake();
        }
    }
}

impl<S: TryStream, T: Topic> Drop for Subscriber<S, T> {
    fn drop(&mut self) {
        self.unsubscribe_all();
    }
}
