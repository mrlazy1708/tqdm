//! Rust implementation of Python command line progress bar tool [tqdm](https://github.com/tqdm/tqdm/).
//!
//! From original documentation:
//! > tqdm derives from the Arabic word taqaddum (تقدّم) which can mean "progress," and is an abbreviation for "I love you so much" in Spanish (te quiero demasiado).
//! > Instantly make your loops show a smart progress meter - just wrap any iterable with tqdm(iterable), and you're done!
//!
//! This crate provides a wrapper [Iterator]. It controls multiple progress bars when `next` is called.
//! Most traits are bypassed with [auto-dereference](https://doc.rust-lang.org/std/ops/trait.Deref.html), so original methods can be called with no overhead.
//!

extern crate crossterm;

use std::sync::*;
use std::time::*;

use std::io::Write;

pub use config::Style;

/* -------------------------------------------------------------------------- */
/*                                   PUBLIC                                   */
/* -------------------------------------------------------------------------- */

/* -------------------------------- FUNCTION -------------------------------- */

///
/// Wrap [Iterator] like it in Python
///
pub fn tqdm<Item, Iter: Iterator<Item = Item>>(iterable: Iter) -> Tqdm<Item, Iter> {
    let info = Arc::new(Mutex::new(Info {
        begin: SystemTime::now(),
        config: Config::default(),

        nitem: 0,
        total: iterable.size_hint().1,
    }));

    if let Ok(mut tqdm) = TQDM.lock() {
        tqdm.push(info.clone());
    }

    Tqdm {
        iterable,

        info: Some(info),
        next: UNIX_EPOCH,
        cache: 0usize,
        freqlim: 24.,
    }
}

///
/// Manually refresh all progress bars
///
pub fn refresh() -> std::io::Result<()> {
    let mut stderr = std::io::stderr();

    use crossterm::ExecutableCommand;
    stderr.execute(crossterm::cursor::MoveToColumn(0))?;
    stderr.execute(crossterm::cursor::SavePosition)?;

    if let Ok(tqdm) = TQDM.lock() {
        let lim = crossterm::terminal::size().map_or(80, |(width, _)| width as usize);
        let info: Vec<_> = tqdm.iter().filter_map(|info| info.lock().ok()).collect();
        for info in &info {
            eprint!("{:<1$}", format!("{}", info), lim);
        }

        stderr.execute(crossterm::cursor::RestorePosition)?;
    }

    stderr.flush()
}

/* --------------------------------- STRUCT --------------------------------- */

///
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
///
pub struct Tqdm<Item, Iter: Iterator<Item = Item>> {
    /// Iterable to decorate with a progress bar
    pub iterable: Iter,

    /// Reference to its information mutex
    info: Option<Arc<Mutex<Info>>>,

    /// Timestamp after which need refresh
    next: SystemTime,

    /// Steps to be synchronized
    cache: usize,

    /// Maximum update frequency
    freqlim: f64,
}

///
/// Configure progress bar
///
impl<Item, Iter: Iterator<Item = Item>> Tqdm<Item, Iter> {
    ///
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
    ///
    pub fn desc<S: ToString>(self, desc: Option<S>) -> Self {
        if let Some(info) = &self.info {
            if let Ok(mut info) = info.lock() {
                info.config.desc = desc.map(|desc| desc.to_string());
            }
        }

        self
    }

    ///
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
    ///
    pub fn width(self, width: Option<usize>) -> Self {
        if let Some(info) = &self.info {
            if let Ok(mut info) = info.lock() {
                info.config.width = width;
            }
        }

        self
    }

    ///
    /// Configure progress bar's style
    ///
    /// * `style` - `enum` of the style
    ///
    ///
    /// ## Examples
    /// ```
    /// tqdm(0..100).style(tqdm::Style::Balloon)
    /// ```
    ///
    pub fn style(self, style: Style) -> Self {
        if let Some(info) = &self.info {
            if let Ok(mut info) = info.lock() {
                info.config.style = style;
            }
        }

        self
    }
}

///
/// Progress bar operations
///
impl<Item, Iter: Iterator<Item = Item>> Tqdm<Item, Iter> {
    fn sync(&mut self) -> std::io::Result<()> {
        if let Some(info) = &self.info {
            if let Ok(mut info) = info.lock() {
                info.nitem += self.cache;
                self.cache = 0;
            }
        }

        refresh()
    }

    ///
    /// Manually close the bar and unregister it
    ///
    pub fn close(&mut self) -> std::io::Result<()> {
        self.sync()?;

        if let Ok(mut tqdm) = TQDM.lock() {
            if let Some(info) = self.info.take() {
                if let Ok(info) = info.lock() {
                    eprintln!("{}", info);
                }

                tqdm.retain(|this| !Arc::ptr_eq(this, &info));
            }
        }

        refresh()
    }
}

impl<Item, Iter: Iterator<Item = Item>> Iterator for Tqdm<Item, Iter> {
    type Item = Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next.elapsed().is_ok() {
            drop(self.sync());

            self.next = SystemTime::now();
            self.next += Duration::from_secs_f64(1. / self.freqlim);
        }

        if let Some(next) = self.iterable.next() {
            self.cache += 1;
            Some(next)
        } else {
            drop(self.close());
            None
        }
    }
}

impl<Item, Iter: Iterator<Item = Item>> std::ops::Deref for Tqdm<Item, Iter> {
    type Target = Iter;

    fn deref(&self) -> &Self::Target {
        &self.iterable
    }
}

impl<Item, Iter: Iterator<Item = Item>> std::ops::DerefMut for Tqdm<Item, Iter> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.iterable
    }
}

impl<Item, Iter: Iterator<Item = Item>> Drop for Tqdm<Item, Iter> {
    fn drop(&mut self) {
        drop(self.close());
    }
}

/* ---------------------------------- TRAIT --------------------------------- */

/// Public trait that allow `.tqdm()` method chaining, equivalent to `tqdm::tqdm(iter)`
///
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

/* -------------------------------------------------------------------------- */
/*                                   PRIVATE                                  */
/* -------------------------------------------------------------------------- */

/* --------------------------------- STATIC --------------------------------- */

static TQDM: Mutex<Vec<Arc<Mutex<Info>>>> = Mutex::new(Vec::new());

fn terminal<W: From<u16>, H: From<u16>>() -> (W, H) {
    let (width, height) = crossterm::terminal::size().unwrap_or((80, 64));
    (W::from(width), H::from(height))
}

/* --------------------------------- CONFIG --------------------------------- */

use config::*;
mod config {

    #[derive(Default)]
    pub struct Config {
        pub desc: Option<String>,
        pub width: Option<usize>,
        pub style: Style,
    }

    ///
    /// Progress bar style enumeration
    ///
    /// - `ASCII`: Pure ASCII bar with `"0123456789#"`
    /// - `Block`: Common bar with unicode characters `" ▏▎▍▌▋▊▉█"`
    /// - `Balloon`: Simulate balloon explosion with `".oO@*"`. Inspired by [stackoverflow](https://stackoverflow.com/a/2685509/17570263)
    ///
    /// Other styles are open for [contribution](https://github.com/mrlazy1708/tqdm/issues/1).
    ///
    pub enum Style {
        ASCII,
        Block,
        Balloon,
    }

    impl Default for Style {
        fn default() -> Style {
            Style::Block
        }
    }

    impl ToString for Style {
        fn to_string(&self) -> String {
            String::from(match self {
                Style::ASCII => "0123456789#",
                Style::Block => " ▏▎▍▌▋▊▉█",
                Style::Balloon => ".oO@*",
            })
        }
    }
}

/* ---------------------------------- INFO ---------------------------------- */

struct Info {
    begin: std::time::SystemTime,
    config: Config,

    nitem: usize,
    total: Option<usize>,
}

impl std::fmt::Display for Info {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let elapsed = {
            let time = self.begin.elapsed();
            time.as_ref().map_or(0., std::time::Duration::as_secs_f64)
        };

        let Config { desc, width, style } = &self.config;
        let desc = desc.clone().map_or(String::new(), |desc| desc + ": ");
        let width = width.unwrap_or_else(|| terminal::<usize, u16>().0);

        /// Time format omitting leading 0
        fn ftime(seconds: usize) -> String {
            let m = seconds / 60 % 60;
            let s = seconds % 60;
            match seconds / 3600 {
                0 => format!("{:02}:{:02}", m, s),
                h => format!("{:02}:{:02}:{:02}", h, m, s),
            }
        }

        let it = self.nitem;
        let its = it as f64 / elapsed;
        let time = ftime(elapsed as usize);
        match self.total {
            None => fmt.write_fmt(format_args!("{}{}it [{}, {:.02}it/s]", desc, it, time, its)),

            Some(total) => {
                let pct = (it as f64 / total as f64).clamp(0.0, 1.0);
                let eta = match it {
                    0 => String::from("?"),
                    _ => ftime((elapsed / pct * (1. - pct)) as usize),
                };

                let bra_ = format!("{}{:>3}%|", desc, (100.0 * pct) as usize);
                let _ket = format!("| {}/{} [{}<{}, {:.02}it/s]", it, total, time, eta, its);
                let tqdm = {
                    let limit = width.checked_sub(bra_.len() + _ket.len()).unwrap_or(0);
                    let pattern: Vec<char> = style.to_string().chars().collect();

                    let m = pattern.len();
                    let n = ((limit as f64 * pct) * m as f64) as usize;

                    let bar = pattern.last().unwrap().to_string().repeat(n / m);
                    match n / m {
                        x if x == limit => bar,
                        _ => format!("{:<limit$}", format!("{}{}", bar, pattern[n % m])),
                    }
                };

                fmt.write_fmt(format_args!("{}{}{}", bra_, tqdm, _ket))
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
    fn example() {
        tqdm(0..100).for_each(|_| std::thread::sleep(Duration::from_secs_f64(0.01)));
    }

    #[test]
    #[ignore]
    fn very_slow() {
        tqdm(0..100).for_each(|_| std::thread::sleep(Duration::from_secs_f64(10.0)));
    }

    #[test]
    fn range() {
        for i in tqdm(0..100).desc(Some("range")).width(Some(82)) {
            std::thread::sleep(Duration::from_secs_f64(0.1));
            if i % 10 == 0 {
                println!("break #{}", i);
            }
        }
    }

    #[test]
    #[ignore]
    fn infinite() {
        for i in tqdm(0..).desc(Some("infinite")) {
            std::thread::sleep(Duration::from_secs_f64(0.1));
            if i % 10 == 0 {
                println!("break #{}", i);
            }
        }
    }

    #[test]
    fn parallel() {
        let threads: Vec<_> = [
            (200, Style::ASCII),
            (400, Style::Balloon),
            (100, Style::Block),
        ]
        .into_iter()
        .enumerate()
        .map(|(idx, (its, style))| {
            std::thread::spawn(move || {
                for _i in tqdm(0..its)
                    .style(style)
                    .width(Some(82))
                    .desc(Some(format!("par {}", idx).as_str()))
                {
                    std::thread::sleep(Duration::from_millis(10));
                }
            })
        })
        .collect();

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
