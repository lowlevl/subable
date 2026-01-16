use std::sync::Arc;

use futures::{StreamExt, TryStream};

use super::{Inner, Sub, Topic};

/// A _stream_ that can be [`Subable::subscribe`]d to.
pub struct Subable<S: TryStream, T: Topic> {
    inner: Arc<Inner<S, T>>,
}

impl<S: TryStream, T: Topic> Subable<S, T> {
    /// Create a new _subable_ from a `stream`.
    pub fn new(stream: S) -> Self {
        Self {
            inner: Inner {
                wakers: Default::default(),
                stream: stream.peekable().into(),
            }
            .into(),
        }
    }

    /// Subscribe to the provided [`Topic`].
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

    /// Unsubscribe all currently subscribed [`Sub`]s
    /// triggering individual streams to return [`None`].
    pub fn unsubscribe_all(&self) {
        for (_, waker) in self.inner.wakers.write().unwrap().drain() {
            // Wake all tasks, that will subsequently return `None`
            waker.wake();
        }
    }
}

impl<S: TryStream, T: Topic> Drop for Subable<S, T> {
    fn drop(&mut self) {
        self.unsubscribe_all();
    }
}
