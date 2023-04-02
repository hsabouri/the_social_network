use futures::stream::StreamExt;
use futures::Stream;
use std::cmp::Ordering;
use std::task::Poll;

pub mod messages;
pub mod repository;
pub mod users;

#[derive(Clone, Copy, Debug)]
enum StreamState<T> {
    Finished,
    Waiting,
    Yielded(T),
}

impl<T> StreamState<T> {
    fn yielded_or_finished(&self) -> bool {
        match self {
            StreamState::Finished => true,
            StreamState::Waiting => false,
            StreamState::Yielded(_) => true,
        }
    }

    fn unwrap(self) -> T {
        match self {
            StreamState::Finished => panic!("Called unwrap on a Finished StreamState"),
            StreamState::Waiting => panic!("Called unwrap on a Waiting StreamState"),
            StreamState::Yielded(t) => t,
        }
    }

    fn as_ref<'a>(&'a self) -> StreamState<&'a T> {
        match self {
            StreamState::Finished => StreamState::Finished,
            StreamState::Waiting => StreamState::Waiting,
            StreamState::Yielded(t) => StreamState::Yielded(&t),
        }
    }
}

/// Merges multiple sorted streams into one sorted streams.
/// * Input streams have to be sorted, otherwise the resulting stream order is not guaranteed.
/// * Stream's `Item` must implement `Ord`
pub struct MergeSortedStreams<T, I>
where
    T: Stream<Item = I>,
    I: Ord,
{
    streams: Vec<(T, StreamState<I>)>,
}

impl<T, I> std::ops::Deref for MergeSortedStreams<T, I>
where
    T: Stream<Item = I>,
    I: Ord,
{
    type Target = Self;

    fn deref(&self) -> &Self::Target {
        self
    }
}

impl<T, I> Unpin for MergeSortedStreams<T, I>
where
    T: Stream<Item = I>,
    I: Ord,
{
}

impl<T, I> Stream for MergeSortedStreams<T, I>
where
    T: Stream<Item = I> + Unpin,
    I: Ord,
{
    type Item = I;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        // Managing our state
        self.streams
            .iter_mut()
            .for_each(|(stream, value)| match value {
                StreamState::Waiting => match stream.poll_next_unpin(cx) {
                    Poll::Ready(Some(new_value)) => *value = StreamState::Yielded(new_value),
                    Poll::Ready(None) => *value = StreamState::Finished,
                    Poll::Pending => (),
                },
                StreamState::Finished => (),
                StreamState::Yielded(_) => (),
            });

        // A value from all streams must be available.
        if self.streams.iter().all(|(_, v)| v.yielded_or_finished()) {
            // Finding biggest value
            let value = self
                .streams
                .iter_mut()
                .filter(|(_, v)| match v {
                    StreamState::Finished => false,
                    StreamState::Waiting => false,
                    StreamState::Yielded(_) => true,
                })
                .max_by(|(_, v1), (_, v2)| {
                    v1.as_ref()
                        .unwrap()
                        .cmp(v2.as_ref().unwrap())
                });

            match value {
                Some((_, v)) => {
                    let mut ret = StreamState::Waiting;

                    std::mem::swap(&mut ret, v);

                    Poll::Ready(Some(ret.unwrap()))
                }
                None => Poll::Ready(None), // Stream finished
            }
        } else {
            return Poll::Pending;
        }
    }
}

/// Merges multiple sorted streams of `Result<O, E>` into one sorted streams of `Result<O, E>`.
/// * Input streams have to be sorted, otherwise the resulting stream order is not guaranteed.
/// * `O` must implement `Ord`
/// * limitation: Bause of the way it is implemented, a stream that returned an error will not be continued.
///     * If all streams returned an error, only the last one will be continued.
pub struct MergeSortedTryStreams<T, E, O>
where
    T: Stream<Item = Result<E, O>>,
    E: Ord,
{
    streams: Vec<(T, StreamState<Result<E, O>>)>,
}

impl<T, E, O> std::ops::Deref for MergeSortedTryStreams<T, E, O>
where
    T: Stream<Item = Result<E, O>>,
    E: Ord,
{
    type Target = Self;

    fn deref(&self) -> &Self::Target {
        self
    }
}

impl<T, E, O> Unpin for MergeSortedTryStreams<T, E, O>
where
    T: Stream<Item = Result<E, O>>,
    E: Ord,
{
}

impl<T, E, O> Stream for MergeSortedTryStreams<T, E, O>
where
    T: Stream<Item = Result<E, O>> + Unpin,
    E: Ord,
{
    type Item = Result<E, O>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        // Managing our state
        self.streams
            .iter_mut()
            .for_each(|(stream, value)| match value {
                StreamState::Waiting => match stream.poll_next_unpin(cx) {
                    Poll::Ready(Some(new_value)) => *value = StreamState::Yielded(new_value),
                    Poll::Ready(None) => *value = StreamState::Finished,
                    Poll::Pending => (),
                },
                StreamState::Finished => (),
                StreamState::Yielded(_) => (),
            });

        // A value from all streams must be available.
        if self.streams.iter().all(|(_, v)| v.yielded_or_finished()) {
            // Finding biggest value
            let value = self
                .streams
                .iter_mut()
                .filter(|(_, v)| match v {
                    StreamState::Finished => false,
                    StreamState::Waiting => false,
                    StreamState::Yielded(_) => true,
                })
                .max_by(|(_, v1), (_, v2)| {
                    match (v1.as_ref().unwrap(), v2.as_ref().unwrap()) {
                        (Ok(v1), Ok(v2)) => v1.cmp(v2),
                        (Ok(_), Err(_)) => Ordering::Greater,
                        (Err(_), Ok(_)) => Ordering::Less,
                        (Err(_), Err(_)) => Ordering::Equal,
                    }
                });

            match value {
                Some((_, v)) => {
                    let mut ret = StreamState::Waiting;

                    std::mem::swap(&mut ret, v);

                    Poll::Ready(Some(ret.unwrap()))
                }
                None => Poll::Ready(None), // Stream finished
            }
        } else {
            return Poll::Pending;
        }
    }
}
