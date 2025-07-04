use crate::*;

use future::Future;
use sync::{Arc, Mutex};

pub fn tqdm_async<Item: Future, Iter>(iterable: Iter) -> impl Iterator<Item = impl Future>
where
    Iter: IntoIterator<Item = Item>,
{
    let iter = iterable.into_iter();
    let pbar = Arc::new(Mutex::new(pbar(iter.size_hint().1)));

    iter.map(move |item| {
        let pbar = pbar.clone();
        async move {
            let output = item.await;
            if let Ok(mut pbar) = pbar.lock() {
                if let Err(err) = pbar.update(1) {
                    eprintln!("{err}");
                }
            }
            output
        }
    })
}
