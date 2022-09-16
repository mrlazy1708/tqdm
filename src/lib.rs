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

/* -------------------------------------------------------------------------- */
/*                                   PUBLIC                                   */
/* -------------------------------------------------------------------------- */

/* -------------------------------- FUNCTION -------------------------------- */

/// Wrap [Iterator] like it in Python. Returns [Tqdm](crate::Tqdm).
///
/// ## Default
/// - [style](crate::Tqdm::style): `Style::Block`
/// - [width](crate::Tqdm::width): `None`
///
pub fn tqdm<Item, Iter: Iterator<Item = Item>>(iter: Iter) -> Tqdm<Item, Iter> {
    let data = Arc::new(Mutex::new(Data {
        begin: SystemTime::now(),
        config: config::Config::default(),

        nitem: 0,
        total: iter.size_hint().1,
    }));

    if let Ok(mut tqdm) = TQDM.lock() {
        tqdm.push(data.clone());
    }

    refresh();
    Tqdm {
        iter,
        data,

        next: UNIX_EPOCH,
        cache: 0,
        freqlim: 24.,
    }
}

/// Force refresh all tqdms
///
pub fn refresh() {
    if let Ok(tqdms) = TQDM.lock() {
        let tqdms: Vec<_> = tqdms.iter().filter_map(|data| data.lock().ok()).collect();
        let (width, height) = crossterm::terminal::size()
            .map(|(width, height)| (width as usize, height))
            .unwrap_or((80, 24));

        use crossterm::ExecutableCommand;
        let mut stdout = std::io::stdout();
        for (data, pos) in tqdms.iter().zip((0..height).rev()).rev() {
            stdout.execute(crossterm::cursor::MoveTo(0, pos)).unwrap();
            stdout
                .write_fmt(format_args!("{:width$}", format!("{}", data)))
                .unwrap();
        }

        use std::io::Write;
        stdout.flush().unwrap();
    }
}

/* --------------------------------- STRUCT --------------------------------- */

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
    data: Arc<Mutex<Data>>,

    next: SystemTime,
    cache: isize,
    freqlim: f64,
}

impl<Item, Iter: Iterator<Item = Item>> Tqdm<Item, Iter> {
    /// Configure progress bar's name.
    ///
    /// * `desc` - bar description
    ///     - `Some(ToString)`: Named progress bar.
    ///     - `None`: Anonymous.
    ///
    /// ## Examples
    /// ```
    /// tqdm(0..100).desc(Some("Bar1"))
    /// ```
    ///
    pub fn desc<S: ToString>(self, desc: Option<S>) -> Self {
        if let Ok(mut data) = self.data.lock() {
            data.config.desc = desc.map(|desc| desc.to_string());
        }

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
    pub fn width(self, width: Option<usize>) -> Self {
        if let Ok(mut data) = self.data.lock() {
            data.config.width = width;
        }

        self
    }

    /// Configure progress bar style.
    ///
    /// * `style` - `enum` of the style
    ///
    /// ## Examples
    /// ```
    /// tqdm(0..100).style(Style::Balloon)
    /// ```
    ///
    pub fn style(self, style: config::Style) -> Self {
        if let Ok(mut data) = self.data.lock() {
            data.config.style = style;
        }

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
    pub fn update(&mut self, nitem: isize) {
        self.cache += nitem;

        if self.next.elapsed().is_ok() {
            if let Ok(mut data) = self.data.lock() {
                let nitem = (data.nitem as isize) + self.cache;
                data.nitem = nitem.try_into().unwrap_or(0);

                self.cache = 0;
            }

            self.next = SystemTime::now() + Duration::from_secs_f64(1. / self.freqlim);
            refresh();
        }
    }
}

impl<Item, Iter: Iterator<Item = Item>> Iterator for Tqdm<Item, Iter> {
    type Item = Item;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.iter.next();
        self.update(next.is_some() as isize);

        next
    }
}

impl<Item, Iter: Iterator<Item = Item>> std::fmt::Display for Tqdm<Item, Iter> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.data.lock() {
            Ok(data) => data.fmt(fmt),
            Err(_) => Err(std::fmt::Error),
        }
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
            tqdms.retain(|this| !Arc::ptr_eq(this, &self.data));
            let (width, height) = crossterm::terminal::size()
                .map(|(width, height)| (width as usize, height))
                .unwrap_or((80, 24));

            use crossterm::ExecutableCommand;
            let mut stdout = std::io::stdout();

            let pos = (height - 1).checked_sub(tqdms.len() as u16).unwrap_or(0);
            stdout.execute(crossterm::cursor::MoveTo(0, pos)).unwrap();
            stdout
                .write_fmt(format_args!(
                    "{:width$}",
                    format!("{}", self.data.lock().unwrap())
                ))
                .unwrap();

            use std::io::Write;
            stdout.flush().unwrap();

            // let (ncol, nrow) = display_size();
            // let top = nrow.checked_sub(tqdms.len()).unwrap_or(0);

            // use termion::cursor;
            // print!("{}", cursor::Goto(1, top as u16));
            // print!("{:ncol$}", format!("{}", self.data));
        }

        refresh();
    }
}

/* ---------------------------------- TRAIT --------------------------------- */

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

/* ----------------------------------- MOD ---------------------------------- */

pub mod config {

    #[derive(Default)]
    pub struct Config {
        pub desc: Option<String>,
        pub width: Option<usize>,
        pub style: Style,
    }

    /// Progress bar style enumeration.
    ///
    /// - `ASCII`: Pure ASCII bar with `"0123456789#"`.
    /// - `Block`: Common bar with unicode characters `" ▏▎▍▌▋▊▉█"`.
    /// - `Balloon`: Simulate balloon explosion with `".oO@*"`. Inspired by [stackoverflow](https://stackoverflow.com/a/2685509/17570263).
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

/* -------------------------------------------------------------------------- */
/*                                   PRIVATE                                  */
/* -------------------------------------------------------------------------- */

/* --------------------------------- STATIC --------------------------------- */

static TQDM: Mutex<Vec<Arc<Mutex<Data>>>> = Mutex::new(Vec::new());

/* --------------------------------- STRUCT --------------------------------- */

pub struct Data {
    begin: std::time::SystemTime,
    config: config::Config,

    nitem: usize,
    total: Option<usize>,
}

impl std::fmt::Display for Data {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let elapsed = {
            let time = self.begin.elapsed();
            time.as_ref().map_or(0., std::time::Duration::as_secs_f64)
        };

        let config::Config { desc, width, style } = &self.config;
        let desc = desc
            .as_ref()
            .map(|id| format!("{}: ", id))
            .unwrap_or(String::new());
        let width = width.unwrap_or_else(|| {
            crossterm::terminal::size()
                .map(|(width, _)| width as usize)
                .unwrap_or(80)
        });

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
    fn range() {
        for i in tqdm(0..100).desc(Some("range")).width(Some(82)) {
            std::thread::sleep(Duration::from_secs_f64(0.1));
            if i % 10 == 0 {
                println!("break #{}", i);
            }
        }
    }

    #[test]
    fn infinite() {
        for i in tqdm(0..).desc(Some("infinite")) {
            std::thread::sleep(Duration::from_secs_f64(0.1));
            if i % 10 == 0 {
                println!("break #{}", i);
            }
        }
    }

    #[test]
    fn concurrent() {
        for i in tqdm(tqdm(0..100).desc(Some("in")).take(50)).desc(Some("out")) {
            std::thread::sleep(Duration::from_millis(100));
            if i % 10 == 0 {
                println!("break #{}", i);
            }
        }
    }

    #[test]
    fn parallel() {
        let threads: Vec<_> = [
            (200, config::Style::ASCII),
            (400, config::Style::Balloon),
            (100, config::Style::Block),
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
