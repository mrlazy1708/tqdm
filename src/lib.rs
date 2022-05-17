//! Rust implementation of Python command line progress bar tool [tqdm](https://github.com/tqdm/tqdm/).
//!
//! From original documentation:
//! > tqdm derives from the Arabic word taqaddum (تقدّم) which can mean "progress," and is an abbreviation for "I love you so much" in Spanish (te quiero demasiado).
//! > Instantly make your loops show a smart progress meter - just wrap any iterable with tqdm(iterable), and you're done!
//!
//! This crate provides a wrapper [Iterator]. It controls multiple progress bars when `next` is called.
//! Most traits are bypassed with [auto-dereference](https://doc.rust-lang.org/std/ops/trait.Deref.html), so original methods can be called with no overhead.
//!

use std::sync::*;
use std::time::*;

/* -------------------------------------------------------------------------- */
/*                                   PUBLIC                                   */
/* -------------------------------------------------------------------------- */

/// Wrap [Iterator] like it in Python. Returns [Tqdm](crate::Tqdm).
///
/// ## Default
/// - [style](crate::Tqdm::style): `Style::Block`
/// - [width](crate::Tqdm::width): `None`
///
pub fn tqdm<Item, Iter: Iterator<Item = Item>>(iter: Iter) -> Tqdm<Item, Iter> {
    let data = Data {
        start: SystemTime::now(),

        style: Style::Block,
        width: None,

        step: 0,
        total: iter.size_hint().1,
    };

    let sync = Arc::new(Mutex::new(data.clone()));
    if let Ok(mut tqdms) = TQDM.lock() {
        tqdms.push(sync.clone());
    }

    refresh_all();

    Tqdm {
        iter,
        prev: UNIX_EPOCH,

        data,
        sync,
    }
}

/// Public trait that allow `.tqdm()` method chaining. Equivalent to `tqdm::tqdm(iter)`.
///
/// ## Examples
/// ```
/// use tqdm::Iter;
/// (0..).take(1000).tqdm()
/// ```
///
pub trait Iter<Item>: Iterator<Item = Item> {
    fn tqdm(self) -> Tqdm<Item, Self>
    where
        Self: Sized,
    {
        tqdm(self)
    }
}
impl<Item, Iter: Iterator<Item = Item>> crate::Iter<Item> for Iter {}

/// Iterator wrapper. Updates progress bar when `next` is called on it.
///
/// ## Examples
///
/// - Basic Usage
/// ```
/// for _i in tqdm(0..100) {
///     thread::sleep(Duration::from_millis(10));
/// }
/// ```
///
/// - Composition
/// ```
/// for _i in tqdm(tqdm(0..100).take(50)) {
///     thread::sleep(Duration::from_millis(10));
/// }
/// ```
///
/// - Multi-threading
/// ```
/// let threads: Vec<_> = [200, 400, 100].iter().map(|its| {
///         std::thread::spawn(move || {
///             for _i in tqdm(0..*its) {
///                 thread::sleep(Duration::from_millis(10));
///             }
///         })
///     })
///     .collect();
///
/// for handle in threads {
///     handle.join().unwrap();
/// }
/// ```
///
pub struct Tqdm<Item, Iter: Iterator<Item = Item>> {
    iter: Iter,
    prev: SystemTime,

    data: Data,
    sync: Arc<Mutex<Data>>,
}

/// Progress bar style enumeration.
///
/// - `ASCII`: Pure ASCII bar with `"0123456789#"`.
/// - `Block`: Common bar with unicode characters `" ▏▎▍▌▋▊▉█"`.
/// - `Balloon`: Simulate balloon explosion with `".oO@*"`. Inspired by [stackoverflow](https://stackoverflow.com/a/2685509/17570263).
///
/// Other styles are open for [contribution](https://github.com/mrlazy1708/tqdm/issues/1).
///

#[derive(Clone)]
pub enum Style {
    ASCII,
    Block,
    Balloon,
}

impl<Item, Iter: Iterator<Item = Item>> Tqdm<Item, Iter> {
    /// Configure progress bar total iterations.
    ///
    /// * `total` - iterater size-hint
    ///     - `Some(usize)`: Fix-sized length.
    ///     - `None`: Acquire original iterator's `size_hint`.
    ///
    /// ## Examples
    /// ```
    /// tqdm(0..100).total(Some(200))
    /// ```
    ///
    pub fn total(mut self, total: Option<usize>) -> Self {
        self.data.total = total;
        self
    }

    /// Configure progress bar style with its name.
    ///
    /// * `style` - enum of the style
    ///
    /// ## Examples
    /// ```
    /// tqdm(0..100).style(Style::Balloon)
    /// ```
    ///
    pub fn style(mut self, style: Style) -> Self {
        self.data.style = style;
        self
    }

    /// Configure progress bar width.
    ///
    /// * `width` - width limitation
    ///     - `Some(usize)`: Fixed width regardless of terminal size.
    ///     - `None`: Expand to formatter limit or full terminal width.
    ///
    /// ## Examples
    /// ```
    /// tqdm(0..100).width(Some(100))
    /// ```
    ///
    pub fn width(mut self, width: Option<usize>) -> Self {
        self.data.width = width;
        self
    }
}

impl<Item, Iter: Iterator<Item = Item>> Tqdm<Item, Iter> {
    /// Update progress bar steps.
    ///
    /// * `delta` - number of newly processed items
    ///
    /// ## Examples
    /// ```
    /// tqdm(0..100).step(100).step(-1)
    /// ```
    ///
    pub fn step(&mut self, delta: isize) {
        self.data.step = (self.data.step as isize + delta) as usize;

        if (self.prev + Duration::from_millis(16)).elapsed().is_ok() {
            self.prev = SystemTime::now();

            if let Ok(mut data) = self.sync.lock() {
                *data = self.data.clone();
            }

            refresh_all();
        }
    }
}

impl<Item, Iter: Iterator<Item = Item>> Iterator for Tqdm<Item, Iter> {
    type Item = Item;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(item) = self.iter.next() {
            self.step(1);
            Some(item)
        } else {
            None
        }
    }
}

impl<Item, Iter: Iterator<Item = Item>> std::fmt::Display for Tqdm<Item, Iter> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.data.fmt(fmt)
    }
}

impl<Item, Iter: Iterator<Item = Item>> std::ops::Deref for Tqdm<Item, Iter> {
    type Target = Iter;
    fn deref(&self) -> &Self::Target {
        &self.iter
    }
}

impl<Item, Iter: Iterator<Item = Item>> std::ops::DerefMut for Tqdm<Item, Iter> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.iter
    }
}

impl<Item, Iter: Iterator<Item = Item>> Drop for Tqdm<Item, Iter> {
    fn drop(&mut self) {
        if let Ok(mut tqdms) = TQDM.lock() {
            tqdms.retain(|this| !Arc::ptr_eq(this, &self.sync));

            let (ncol, nrow) = display_size();
            let top = nrow.checked_sub(tqdms.len()).unwrap_or(0);

            use termion::cursor;
            print!("{}", cursor::Goto(1, top as u16));
            print!("{:ncol$}", format!("{}", self.data));
        }

        refresh_all();
    }
}

/* -------------------------------------------------------------------------- */
/*                                   PRIVATE                                  */
/* -------------------------------------------------------------------------- */

#[macro_use]
extern crate lazy_static;
lazy_static! {
    static ref TQDM: Mutex<Vec<Arc<Mutex<Data>>>> = Mutex::new(Vec::new());
}

fn display_size() -> (usize, usize) {
    use termion::terminal_size;
    if let Ok((ncol, nrow)) = terminal_size() {
        (ncol as usize, nrow as usize)
    } else {
        (80, 1)
    }
}

fn refresh_all() {
    if let Ok(tqdms) = TQDM.lock() {
        let tqdms: Vec<_> = tqdms.iter().filter_map(|data| data.lock().ok()).collect();

        let (ncol, nrow) = display_size();
        let top = nrow.checked_sub(tqdms.len()).unwrap_or(0);

        use termion::cursor;
        print!("{}", cursor::Goto(1, top as u16 + 1));

        for data in tqdms.iter().take(nrow as usize) {
            print!("{:ncol$}", format!("{}", data));
        }

        use std::io::Write;
        std::io::stdout().flush().unwrap();
    }
}

#[derive(Clone)]
struct Data {
    start: SystemTime,

    width: Option<usize>,
    style: Style,

    total: Option<usize>,
    step: usize,
}

impl Data {
    fn bar(&self) -> (&str, &[char]) {
        match self.style {
            Style::ASCII => ("#", &['0', '1', '2', '3', '4', '5', '6', '7', '8', '9']),
            Style::Block => ("█", &[' ', '▏', '▎', '▍', '▌', '▋', '▊', '▉']),
            Style::Balloon => ("*", &['.', 'o', 'O', '@']),
        }
    }
}

impl std::fmt::Display for Data {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let elapsed_time = SystemTime::now().duration_since(self.start);
        let elapsed = elapsed_time.as_ref().map_or(0, Duration::as_millis) as usize;

        let width = self
            .width
            .or_else(|| fmt.width())
            .unwrap_or_else(|| display_size().0);

        let format_time = |seconds| match seconds / 3600 {
            0 => format!("{:02}:{:02}", seconds / 60 % 60, seconds % 60),
            x => format!("{:02}:{:02}:{:02}", x, seconds / 60 % 60, seconds % 60),
        };

        let et = format_time(elapsed / 1000);
        let rate = self.step as f64 / elapsed as f64 * 1000.0;
        match self.total {
            None => fmt.write_fmt(format_args!("{}it [{}, {:.02}it/s]", self.step, et, rate)),
            Some(tot) => {
                let pct = (self.step as f64 / tot as f64).clamp(0.0, 1.0);
                let rem = tot.checked_sub(self.step).unwrap_or(0);
                let eta = match self.step {
                    0 => "?".to_string(),
                    x => format_time(elapsed * rem / x / 1000),
                };

                let head = format!("{:>3}%|", (100.0 * pct) as usize);
                let tail = format!("| {}/{} [{}<{}, {:.02}it/s]", self.step, tot, et, eta, rate);

                let length = width.checked_sub(head.len() + tail.len()).unwrap_or(0);
                let body = match self.step {
                    step if step == tot => self.bar().0.repeat(length),
                    _ => {
                        let bar = length as f64 * pct;
                        let full = self.bar().0.repeat(bar.floor() as usize);
                        let edge = self.bar().1[(self.bar().1.len() as f64 * bar.fract()) as usize];
                        format!("{}{}", full, edge)
                    }
                };

                fmt.write_fmt(format_args!("{}{:length$}{}", head, body, tail))
            }
        }
    }
}

/* -------------------------------------------------------------------------- */
/*                                    TEST                                    */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range() {
        for i in tqdm(0..100) {
            std::thread::sleep(Duration::from_millis(100));
            if i % 10 == 0 {
                println!("break #{}", i);
            }
        }
    }

    #[test]
    fn infinite() {
        for i in tqdm(0..) {
            std::thread::sleep(Duration::from_millis(10));
            if i % 10 == 0 {
                println!("break #{}", i);
            }
        }
    }

    #[test]
    fn concurrent() {
        for i in tqdm(tqdm(0..100).take(50)) {
            std::thread::sleep(Duration::from_millis(100));
            if i % 10 == 0 {
                println!("break #{}", i);
            }
        }
    }

    #[test]
    fn parallel() {
        let threads = [
            (200, Style::ASCII),
            (400, Style::Balloon),
            (100, Style::Block),
        ]
        .map(|(its, style)| {
            std::thread::spawn(move || {
                for _i in tqdm(0..its).style(style).width(Some(82)) {
                    std::thread::sleep(Duration::from_millis(10));
                }
            })
        });
        for handle in threads {
            handle.join().unwrap();
        }
    }

    #[test]
    fn performance() {
        let ntest = 100000000;

        let start = SystemTime::now();
        for _i in 0..ntest {}
        println!(
            "Baseline: {:.02}it/s",
            ntest as f64 / SystemTime::now().duration_since(start).unwrap().as_millis() as f64
                * 1000.0
        );

        let start = SystemTime::now();
        for _i in tqdm(0..ntest) {}
        println!(
            "With tqdm: {:.02}it/s",
            ntest as f64 / SystemTime::now().duration_since(start).unwrap().as_millis() as f64
                * 1000.0
        );
    }
}
