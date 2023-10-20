//! Rust implementation of Python command line progress bar tool [tqdm](https://github.com/tqdm/tqdm/).
//!
//! From original documentation:
//! > tqdm derives from the Arabic word taqaddum (تقدّم) which can mean "progress," and is an abbreviation for "I love you so much" in Spanish (te quiero demasiado).
//! > Instantly make your loops show a smart progress meter - just wrap any iterable with tqdm(iterable), and you're done!
//!
//! This crate provides a wrapper [Iterator]. It controls multiple progress bars when `next` is called.
//! Most traits are bypassed with [auto-dereference](https://doc.rust-lang.org/std/ops/trait.Deref.html), so original methods can be called with no overhead.
//!

use std::*;

use std::io::Write;
use std::ops::{Deref, DerefMut};
use std::time::{Duration, SystemTime};

extern crate anyhow;
use anyhow::Result;

extern crate crossterm;
use crossterm::QueueableCommand;

extern crate once_cell;
use once_cell::sync::Lazy;

#[cfg(test)]
mod test;

pub mod style;
pub use style::Style;

/* -------------------------------------------------------------------------- */
/*                                    TQDM                                    */
/* -------------------------------------------------------------------------- */

/* -------------------------------- FUNCTION -------------------------------- */

/// Wrap [Iterator] like it in Python. This function creates a default progress
/// bar object and registers it to the global collection. The returned iterator
/// [Deref] to the given one and will update its tqdm whenever `next` is called.

pub fn tqdm<Item, Iter: Iterator<Item = Item>>(iterable: Iter) -> Tqdm<Item, Iter> {
    let id = ID.fetch_add(1, sync::atomic::Ordering::SeqCst);

    if let Ok(mut tqdm) = BAR.lock() {
        tqdm.insert(
            id,
            Info {
                config: Config::default(),

                it: 0,
                its: None,
                total: iterable.size_hint().1,

                t0: time::SystemTime::now(),
                prev: time::UNIX_EPOCH,
            },
        );
    }

    if let Err(err) = refresh() {
        eprintln!("{}", err)
    }

    Tqdm {
        iterable,
        id,

        next: time::UNIX_EPOCH,
        step: 0,

        mininterval: Duration::from_secs_f64(1. / 24.),
        miniters: 0,
    }
}

/// Manually refresh all tqdms

pub fn refresh() -> Result<()> {
    let mut out = io::stderr();

    if let Ok(tqdm) = BAR.lock() {
        let (ncols, nrows) = size();

        if tqdm.is_empty() {
            return Ok(());
        }

        out.queue(crossterm::cursor::Hide)?;
        out.queue(crossterm::cursor::MoveToColumn(0))?;

        let time = SystemTime::now();

        for info in tqdm.values().take(nrows - 1) {
            let bar = format!("{:<1$}", info.format(time)?, ncols);
            out.queue(crossterm::style::Print(bar))?;
        }

        let nbars = tqdm.len();
        if nbars > nrows {
            out.queue(crossterm::terminal::Clear(
                crossterm::terminal::ClearType::FromCursorDown,
            ))?;
            out.queue(crossterm::style::Print(" ... (more hidden) ..."))?;
            out.queue(crossterm::cursor::MoveToColumn(0))?;
        }

        if let Some(rows) = num::NonZeroUsize::new(nbars - 1) {
            out.queue(crossterm::cursor::MoveUp(rows.get() as u16))?;
        }

        out.queue(crossterm::cursor::Show)?;
    }

    Ok(out.flush()?)
}

/* --------------------------------- STRUCT --------------------------------- */

/// Iterator wrapper that updates progress bar on `next`
///
///
/// ## Examples
///
/// - Basic Usage
/// ```
/// for _ in tqdm(0..100) {
///     thread::sleep(Duration::from_millis(10));
/// }
/// ```
///
/// - Composition
/// ```
/// for _ in tqdm(tqdm(0..100).take(50)) {
///     thread::sleep(Duration::from_millis(10));
/// }
/// ```
///
/// - Multi-threading
/// ```
/// let threads: Vec<_> = [200, 400, 100].iter().map(|its| {
///         std::thread::spawn(move || {
///             for _ in tqdm(0..*its) {
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

pub struct Tqdm<Item, Iter: Iterator<Item = Item>> {
    /// Iterable wrapped
    pub iterable: Iter,

    /// Hash
    id: usize,

    /// Next refresh time
    next: time::SystemTime,

    /// Cached
    step: usize,

    /// Refresh limit
    mininterval: Duration,
    miniters: usize,
}

impl<Item, Iter: Iterator<Item = Item>> Tqdm<Item, Iter> {
    /// Configure progress bar's name
    ///
    /// * `desc` - bar description
    ///     - `Some(S)`: Named progress bar
    ///     - `None`: Anonymous
    ///
    ///
    /// ## Examples
    /// ```
    /// tqdm(0..100).desc(Some("Bar1"))
    /// ```

    pub fn desc<S: ToString>(self, desc: Option<S>) -> Self {
        if let Ok(mut tqdm) = BAR.lock() {
            let info = tqdm.get_mut(&self.id);
            if let Some(info) = info {
                info.config.desc = desc.map(|desc| desc.to_string());
            }
        }

        self
    }

    /// Configure progress bar's width
    ///
    /// * `width` - width limitation
    ///     - `Some(usize)`: Fixed width regardless of terminal size
    ///     - `None`: Expand to formatter limit or full terminal width
    ///
    ///
    /// ## Examples
    /// ```
    /// tqdm(0..100).width(Some(100))
    /// ```

    pub fn width(self, width: Option<usize>) -> Self {
        if let Ok(mut tqdm) = BAR.lock() {
            let info = tqdm.get_mut(&self.id);
            if let Some(info) = info {
                info.config.width = width;
            }
        }

        self
    }

    /// Configure progress bar's style
    ///
    /// * `style` - `enum` of the style
    ///
    ///
    /// ## Examples
    /// ```
    /// tqdm(0..100).style(tqdm::Style::Balloon)
    /// ```

    pub fn style(self, style: Style) -> Self {
        if let Ok(mut tqdm) = BAR.lock() {
            let info = tqdm.get_mut(&self.id);
            if let Some(info) = info {
                info.config.style = style;
            }
        }

        self
    }

    /// Exponential smoothing factor
    ///
    /// * `smoothing` - weight for the current update
    ///
    ///
    /// ## Examples
    /// ```
    /// tqdm(0..100).smoothing(0.9999)
    /// ```

    pub fn smoothing(self, smoothing: f64) -> Self {
        if let Ok(mut tqdm) = BAR.lock() {
            let info = tqdm.get_mut(&self.id);
            if let Some(info) = info {
                info.config.smoothing = smoothing;
            }
        }

        self
    }

    /// Behavior of after termination
    ///
    /// * `clear` - true: remove this bar as if never created
    ///           - false: leave completed bar at the very top
    ///
    ///
    /// ## Examples
    /// ```
    /// tqdm(0..100).clear(true)
    /// ```

    pub fn clear(self, clear: bool) -> Self {
        if let Ok(mut tqdm) = BAR.lock() {
            let info = tqdm.get_mut(&self.id);
            if let Some(info) = info {
                info.config.clear = clear;
            }
        }

        self
    }
}

impl<Item, Iter: Iterator<Item = Item>> Tqdm<Item, Iter> {
    /// Manually close the bar and unregister it

    pub fn close(&mut self) -> Result<()> {
        let time = SystemTime::now();
        let mut out = io::stderr();

        if let Ok(mut tqdm) = BAR.lock() {
            if let Some(mut info) = tqdm.remove(&self.id) {
                info.update(time, self.step);

                out.queue(crossterm::cursor::MoveToColumn(0))?;
                if info.config.clear {
                    out.queue(crossterm::cursor::MoveDown(tqdm.len() as u16))?;
                    out.queue(crossterm::terminal::Clear(
                        crossterm::terminal::ClearType::CurrentLine,
                    ))?;
                    out.queue(crossterm::cursor::MoveUp(tqdm.len() as u16))?;
                } else {
                    out.queue(crossterm::style::Print(info.format(time)?))?;
                    out.queue(crossterm::style::Print("\n"))?;
                }
            }
        }

        refresh()
    }
}

impl<Item, Iter: Iterator<Item = Item>> Iterator for Tqdm<Item, Iter> {
    type Item = Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.step >= self.miniters {
            let now = SystemTime::now();
            if now >= self.next {
                if let Ok(mut tqdm) = BAR.lock() {
                    if let Some(info) = tqdm.get_mut(&self.id) {
                        info.update(now, self.step);
                        self.step = 0;
                    }
                }

                if let Err(err) = refresh() {
                    eprintln!("{}", err)
                }

                self.next = now + self.mininterval;
            }
        }

        if let Some(next) = self.iterable.next() {
            self.step += 1;
            Some(next)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iterable.size_hint()
    }
}

impl<Item, Iter: Iterator<Item = Item>> Deref for Tqdm<Item, Iter> {
    type Target = Iter;

    fn deref(&self) -> &Self::Target {
        &self.iterable
    }
}

impl<Item, Iter: Iterator<Item = Item>> DerefMut for Tqdm<Item, Iter> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.iterable
    }
}

impl<Item, Iter: Iterator<Item = Item>> Drop for Tqdm<Item, Iter> {
    fn drop(&mut self) {
        if let Err(err) = self.close() {
            eprintln!("{}", err)
        }
    }
}

/* ---------------------------------- TRAIT --------------------------------- */

/// Trait that allows `.tqdm()` method chaining, equivalent to `tqdm::tqdm(iter)`
///
///
/// ## Examples
/// ```
/// use tqdm::Iter;
/// (0..).take(1000).tqdm()
/// ```

pub trait Iter<Item>: Iterator<Item = Item> {
    fn tqdm(self) -> Tqdm<Item, Self>
    where
        Self: Sized,
    {
        tqdm(self)
    }
}

impl<Item, Iter: Iterator<Item = Item>> crate::Iter<Item> for Iter {}

/* -------------------------------------------------------------------------- */
/*                                   PRIVATE                                  */
/* -------------------------------------------------------------------------- */

/* --------------------------------- STATIC --------------------------------- */

static ID: Lazy<sync::atomic::AtomicUsize> = Lazy::new(|| sync::atomic::AtomicUsize::new(0));
static BAR: Lazy<sync::Mutex<collections::BTreeMap<usize, Info>>> =
    Lazy::new(|| sync::Mutex::new(collections::BTreeMap::new()));

fn size<T: From<u16>>() -> (T, T) {
    let (width, height) = crossterm::terminal::size().unwrap_or((80, 24));
    (T::from(width), T::from(height))
}

/* --------------------------------- CONFIG --------------------------------- */

struct Config {
    desc: Option<String>,
    width: Option<usize>,
    style: style::Style,
    smoothing: f64,
    clear: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            desc: None,
            width: None,
            style: Style::default(),
            smoothing: 0.3,
            clear: false,
        }
    }
}

/* ---------------------------------- INFO ---------------------------------- */

struct Info {
    config: Config,

    it: usize,
    its: Option<f64>,
    total: Option<usize>,

    t0: time::SystemTime,
    prev: time::SystemTime,
}

impl Info {
    fn format(&self, t: SystemTime) -> Result<String> {
        let desc = self
            .config
            .desc
            .clone()
            .map_or(String::new(), |desc| desc + ": ");
        let width = self.config.width.unwrap_or_else(|| size().0);

        let elapsed = ftime(t.duration_since(self.t0)?.as_secs_f64() as usize);

        /// Time format omitting leading 0
        fn ftime(seconds: usize) -> String {
            let m = seconds / 60 % 60;
            let s = seconds % 60;
            match seconds / 3600 {
                0 => format!("{:02}:{:02}", m, s),
                h => format!("{:02}:{:02}:{:02}", h, m, s),
            }
        }

        let it = self.it;
        let its = match self.its {
            None => String::from("?"),
            Some(its) => format!("{:.02}", its),
        };

        Ok(match self.total {
            None => format_args!("{}{}it [{}, {}it/s]", desc, it, elapsed, its).to_string(),

            Some(total) => {
                let pct = (it as f64 / total as f64).clamp(0.0, 1.0);
                let eta = match self.its {
                    None => String::from("?"),
                    Some(its) => ftime(((total - it) as f64 / its) as usize),
                };

                let bra_ = format!("{}{:>3}%|", desc, (100.0 * pct) as usize);
                let _ket = format!("| {}/{} [{}<{}, {}it/s]", it, total, elapsed, eta, its);
                let tqdm = {
                    let limit = width.saturating_sub(bra_.len() + _ket.len());
                    let pattern: Vec<_> = self.config.style.to_string().chars().collect();

                    let m = pattern.len();
                    let n = ((limit as f64 * pct) * m as f64) as usize;

                    let bar = pattern.last().unwrap().to_string().repeat(n / m);
                    match n / m {
                        x if x == limit => bar,
                        _ => format!("{:<limit$}", format!("{}{}", bar, pattern[n % m])),
                    }
                };

                format_args!("{}{}{}", bra_, tqdm, _ket).to_string()
            }
        })
    }

    fn update(&mut self, t: SystemTime, n: usize) {
        if self.prev != time::UNIX_EPOCH {
            let dt = t.duration_since(self.prev).unwrap();
            let its = n as f64 / dt.as_secs_f64();

            self.its = match self.its {
                None => Some(its),
                Some(ema) => {
                    let beta = self.config.smoothing;
                    Some(its * beta + ema * (1. - beta))
                }
            };
        }

        self.prev = t;
        self.it += n;
    }
}
