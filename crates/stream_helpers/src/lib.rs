use futures::stream::StreamExt;
use futures::Stream;
use std::cmp::Ordering;
use std::task::Poll;

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
    T: Stream<Item = I> + Send,
    I: Ord,
{
    streams: Vec<(T, StreamState<I>)>,
}

impl<T, I> MergeSortedStreams<T, I>
where
    T: Stream<Item = I> + Send,
    I: Ord,
{
    pub fn new(streams: Vec<T>) -> Self {
        Self {
            streams: streams
                .into_iter()
                .map(|stream| (stream, StreamState::Waiting))
                .collect(),
        }
    }
}

impl<T, I> Unpin for MergeSortedStreams<T, I>
where
    T: Stream<Item = I> + Send,
    I: Ord,
{
}

impl<T, I> Stream for MergeSortedStreams<T, I>
where
    T: Stream<Item = I> + Unpin + Send,
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
                .min_by(|(_, v1), (_, v2)| v1.as_ref().unwrap().cmp(v2.as_ref().unwrap()));

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
    T: Stream<Item = Result<E, O>> + Send,
    E: Ord,
{
    streams: Vec<(T, StreamState<Result<E, O>>)>,
}

impl<T, E, O> MergeSortedTryStreams<T, E, O>
where
    T: Stream<Item = Result<E, O>> + Send,
    E: Ord,
{
    pub fn new(streams: Vec<T>) -> Self {
        Self {
            streams: streams
                .into_iter()
                .map(|stream| (stream, StreamState::Waiting))
                .collect(),
        }
    }
}

impl<T, E, O> Unpin for MergeSortedTryStreams<T, E, O>
where
    T: Stream<Item = Result<E, O>> + Send,
    E: Ord,
{
}

impl<T, E, O> Stream for MergeSortedTryStreams<T, E, O>
where
    T: Stream<Item = Result<E, O>> + Unpin + Send,
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
                .min_by(
                    |(_, v1), (_, v2)| match (v1.as_ref().unwrap(), v2.as_ref().unwrap()) {
                        (Ok(v1), Ok(v2)) => v1.cmp(v2),
                        (Ok(_), Err(_)) => Ordering::Less,
                        (Err(_), Ok(_)) => Ordering::Greater,
                        (Err(_), Err(_)) => Ordering::Equal,
                    },
                );

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

#[cfg(test)]
#[tokio::test]
async fn test_merge_sorted_streams() -> Result<(), Box<dyn std::error::Error>> {
    use futures::stream;

    let a = stream::iter(&[7, 8, 14, 16]);
    let b = stream::iter(&[9]);
    let c = stream::iter(&[7, 8]);
    let d = stream::iter(&[1, 12]);
    let expected = vec![1, 7, 7, 8, 8, 9, 12, 14, 16];

    let stream = MergeSortedStreams::new(vec![a, b, c, d]);

    let result: Vec<i32> = stream.collect().await;

    assert_eq!(result, expected);

    Ok(())
}

#[cfg(test)]
#[tokio::test]
async fn test_merge_sorted_trystreams_simple() -> Result<(), Box<dyn std::error::Error>> {
    use futures::stream;

    let a = stream::iter(vec![Ok(7), Ok(8), Ok(14), Ok(16)]);
    let b = stream::iter(vec![Ok(9)]);
    let c = stream::iter(vec![Ok(7), Ok(8)]);
    let d = stream::iter(vec![Ok(1), Ok(12)]);
    let expected = vec![
        Ok(1),
        Ok(7),
        Ok(7),
        Ok(8),
        Ok(8),
        Ok(9),
        Ok(12),
        Ok(14),
        Ok(16),
    ];

    let stream = MergeSortedTryStreams::new(vec![a, b, c, d]);

    let result: Vec<Result<i32, ()>> = stream.collect().await;

    assert_eq!(result, expected);

    Ok(())
}

#[cfg(test)]
#[tokio::test]
async fn test_merge_sorted_trystreams_intermediate() -> Result<(), Box<dyn std::error::Error>> {
    use futures::stream;

    let a = stream::iter(vec![Ok(7), Err(()), Ok(14), Ok(16)]);
    let b = stream::iter(vec![Ok(9)]);
    let c = stream::iter(vec![Ok(7), Ok(8)]);
    let d = stream::iter(vec![Ok(1), Ok(12)]);
    let expected = vec![
        Ok(1),
        Ok(7),
        Ok(7),
        Ok(8),
        Ok(9),
        Ok(12),
        Err(()),
        Ok(14),
        Ok(16),
    ];

    let stream = MergeSortedTryStreams::new(vec![a, b, c, d]);

    let result: Vec<Result<i32, ()>> = stream.collect().await;

    assert_eq!(result, expected);

    Ok(())
}

/// Demonstrate an edge case where output stream is not sorted if there is an error in one of the streams.
#[cfg(test)]
#[tokio::test]
async fn test_merge_sorted_trystreams_edge() -> Result<(), Box<dyn std::error::Error>> {
    use futures::stream;

    let a = stream::iter(vec![Ok(7), Err(()), Ok(9), Ok(16)]);
    let b = stream::iter(vec![Ok(9)]);
    let c = stream::iter(vec![Ok(7), Ok(8)]);
    let d = stream::iter(vec![Ok(1), Ok(12)]);
    let expected = vec![
        Ok(1),
        Ok(7),
        Ok(7),
        Ok(8),
        Ok(9),
        Ok(12),
        Err(()),
        Ok(9),
        Ok(16),
    ];

    let stream = MergeSortedTryStreams::new(vec![a, b, c, d]);

    let result: Vec<Result<i32, ()>> = stream.collect().await;

    assert_eq!(result, expected);

    Ok(())
}
