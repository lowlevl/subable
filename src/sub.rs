use std::sync::Arc;

use futures::{FutureExt, Stream, TryStream, task};

use super::{Inner, Topic};

pub enum Subed<I> {
    Yes(I),
    No(I),
}

pub struct Sub<S: TryStream, T: Topic> {
    inner: Arc<Inner<S, T>>,
    topic: T,
}

impl<S: TryStream, T: Topic> Sub<S, T> {
    pub(super) fn new(inner: Arc<Inner<S, T>>, topic: T) -> Self {
        Self { inner, topic }
    }
}

impl<S: TryStream, T: Topic> Drop for Sub<S, T> {
    fn drop(&mut self) {
        tracing::trace!("unsubscribing {:?}", self.topic);

        self.inner.wakers.write().unwrap().remove(&self.topic);
    }
}

impl<S: TryStream + Stream<Item = Result<S::Ok, S::Error>> + Unpin, T: Topic<From = S::Ok>> Stream
    for Sub<S, T>
{
    type Item = Result<Subed<S::Ok>, S::Error>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        match self.inner.wakers.read().unwrap().get(&self.topic) {
            // Register the task for wake-up
            Some(waker) => waker.register(cx.waker()),

            // If the waker isn't registered, that means the stream is closed
            None => return task::Poll::Ready(None),
        };

        let mut stream = futures::ready!(self.inner.stream.lock().poll_unpin(cx));
        let mut stream = std::pin::Pin::new(&mut *stream);

        match futures::ready!(stream.as_mut().poll_peek(cx)) {
            Some(Ok(item)) => {
                let topic = T::topic(item);
                let wakers = self.inner.wakers.read().unwrap();

                if let Some(waker) = wakers.get(&topic)
                    && topic != self.topic
                {
                    // The item is destined to another task, wake it and stay pending

                    waker.wake();
                    task::Poll::Pending
                } else if topic == self.topic {
                    // The item is for us, pop it as `Match`
                    stream.as_mut().poll_next(cx).map_ok(Subed::Yes)
                } else {
                    // The item is unhandled, pop it as `Default`
                    stream.as_mut().poll_next(cx).map_ok(Subed::No)
                }
            }

            // The stream errored, pop it from the stream
            Some(_) => stream.as_mut().poll_next(cx).map_ok(Subed::No),

            // The stream ended, return `None`
            None => task::Poll::Ready(None),
        }
    }
}
