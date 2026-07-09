//! Reassemble an as-completed result stream into submission order.

use std::future::poll_fn;

use futures_core::Stream;

/// Drain `stream` and place each `(id, value)` into a `Vec` of length `len` at
/// index `id`, returning the values in submission order.
///
/// Ids at or beyond `len`, and gaps left by ids never seen, are dropped: the
/// caller sizes `len` to the number of submitted jobs, and the engine emits each
/// id exactly once, so a well-formed run fills every slot.
pub(crate) async fn collect_ordered<S, O>(stream: &mut S, len: usize) -> Vec<O>
where
    S: Stream<Item = (usize, O)> + Unpin,
{
    let mut slots: Vec<Option<O>> = Vec::with_capacity(len);
    slots.resize_with(len, || None);
    while let Some((id, value)) = poll_fn(|cx| std::pin::Pin::new(&mut *stream).poll_next(cx)).await
    {
        if let Some(slot) = slots.get_mut(id) {
            *slot = Some(value);
        }
    }
    slots.into_iter().flatten().collect()
}

#[cfg(test)]
mod tests {
    use super::collect_ordered;
    use futures_util::stream;

    #[tokio::test]
    async fn reorders_completion_order_into_submission_order() {
        // Arrives 2, 0, 1 — must come back [a0, a1, a2].
        let mut s = stream::iter(vec![(2usize, "c"), (0usize, "a"), (1usize, "b")]);
        let ordered = collect_ordered(&mut s, 3).await;
        assert_eq!(ordered, vec!["a", "b", "c"]);
    }

    #[tokio::test]
    async fn empty_stream_yields_empty_vec() {
        let mut s = stream::iter(Vec::<(usize, &str)>::new());
        let ordered = collect_ordered(&mut s, 0).await;
        assert!(ordered.is_empty());
    }
}
