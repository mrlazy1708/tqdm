# tqdm

Rust implementation of Python command line progress bar tool [tqdm](https://github.com/tqdm/tqdm/).

From original documentation:

> tqdm derives from the Arabic word taqaddum (تقدّم) which can mean "progress," and is an abbreviation for "I love you so much" in Spanish (te quiero demasiado).
>
> Instantly make your loops show a smart progress meter - just wrap any iterable with tqdm(iterable), and you're done!
>

![demo](demo-multithread.gif)



This crate provides a wrapper [Iterator](https://doc.rust-lang.org/core/iter/trait.Iterator.html). It controls multiple progress bars when `next` is called.

Most traits are bypassed with [auto-dereference](https://doc.rust-lang.org/std/ops/trait.Deref.html), so original methods can be called with no overhead.



## Usage

Just wrap anything that implements the [Iterator](https://doc.rust-lang.org/core/iter/trait.Iterator.html) trait with `tqdm`

```rust
use tqdm::tqdm;
for i in tqdm(0..10000) {
  ...
```

```
 76%|███████████████▉     | 7618/10000 [00:09<00:03, 782.14it/s]
```



Expose trait to allow method chaining

```rust
use tqdm::Iter;
for i in (0..).take(10000).tqdm().style(tqdm::Style::Balloon) {
  ...
```

```
 47%|**********.          | 4792/10000 [00:06<00:06, 783.39it/s]
```



Multi-threading is also supported!

```rust
use tqdm::tqdm;
use std::thread;
for t in (0..3) {
  thread::spawn(move || {
    for i in tqdm(0..).style(...) {
      ...
```

```
 38%|##########0               | 77/200 [00:00<00:01, 83.24it/s]
 77%|████████████████████      | 77/100 [00:00<00:00, 83.24it/s]
 19%|*****.                    | 77/400 [00:00<00:03, 83.24it/s]
```



For more usage, please refer to [doc](https://docs.rs/tqdm/latest/tqdm)

