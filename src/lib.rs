//! Rust implementation of Python command line progress bar tool [tqdm](https://github.com/tqdm/tqdm/).
//!
//! From original documentation:
//! > tqdm derives from the Arabic word taqaddum (تقدّم) which can mean "progress," and is an abbreviation for "I love you so much" in Spanish (te quiero demasiado).
//! > Instantly make your loops show a smart progress meter - just wrap any iterable with tqdm(iterable), and you're done!
//!
//! This crate provides a wrapper [Iterator]. It controls multiple progress bars when `next` is called.
//! Most traits are bypassed with [auto-dereference](https://doc.rust-lang.org/std/ops/trait.Deref.html), so original methods can be called with no overhead.

use std::sync::*;
use std::time::*;

/* -------------------------------------------------------------------------- */
/*                                   PUBLIC                                   */
/* -------------------------------------------------------------------------- */

/// Wrap [Iterator] like it in Python. Returns [Tqdm](crate::Tqdm).
///
/// ## Default
/// - [style](crate::Tqdm::style): `"block"`
/// - [width](crate::Tqdm::width): `None`
///
pub fn tqdm<Item, Iter: Iterator<Item = Item>>(iter: Iter) -> Tqdm<Item, Iter> {
    let data = Arc::new(Mutex::new(TqdmData {
        start: SystemTime::now(),

        step: 0,
        size: None,

        style: "block".to_string(),
        width: None,
    }));

    use termion::cursor;
    print!("{}", cursor::Goto(1, terminal_size().1));
    TQDM.lock().unwrap().push(data.clone());

    Tqdm {
        iter,
        data,
        prev: UNIX_EPOCH,

        step: 0,
        size: None,
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
    data: Arc<Mutex<TqdmData>>,
    prev: SystemTime,

    step: usize,
    size: Option<usize>,
}

impl<Item, Iter: Iterator<Item = Item>> Tqdm<Item, Iter> {
    /// Configure progress bar style with its name.
    ///
    /// * `style` - name of the style
    ///     - `"ascii"`: Pure ascii bar with `"0123456789#"`.
    ///     - `"block"`: Common bar with unicode characters `" ▏▎▍▌▋▊▉█"`.
    ///     - `"bubble"`: Simulate bubble explosion with `".oO@*"`. Inspired by [stackoverflow](https://stackoverflow.com/a/2685509/17570263).
    ///
    ///     Other styles are open for [contribution](https://github.com/mrlazy1708/tqdm/issues/1).
    ///
    /// ## Examples
    /// ```
    /// tqdm(0..100).style("bubble")
    /// ```
    ///
    pub fn style(self, style: &str) -> Self {
        self.data.lock().unwrap().style = style.to_string();
        self
    }

    /// Configure progress bar width.
    ///
    /// * `width` - width limitation
    ///     - `Some(usize)`: Fixed width regardless of terminal size.
    ///     - `None`: Expand to whole terminal width.
    ///
    /// ## Examples
    /// ```
    /// tqdm(0..100).width(Some(100))
    /// ```
    ///
    pub fn width(self, width: Option<usize>) -> Self {
        self.data.lock().unwrap().width = width;
        self
    }
}

impl<Item, Iter: Iterator<Item = Item>> Iterator for Tqdm<Item, Iter> {
    type Item = Item;
    fn next(&mut self) -> Option<Self::Item> {
        let next = self.iter.next();
        if next.is_some() {
            self.step += 1
        }

        let now = SystemTime::now();
        if now.duration_since(self.prev).unwrap() > Duration::from_millis(40) {
            self.prev = now;

            {
                let mut data = self.data.lock().unwrap();
                data.step = self.step;
                data.size = self.size.or_else(|| self.iter.size_hint().1);
            }

            {
                let tqdms = TQDM.lock().unwrap();

                let (ncol, nrow) = terminal_size();
                let top = nrow.checked_sub(tqdms.len() as u16).unwrap_or(0);

                use termion::cursor;
                print!("{}", cursor::Goto(1, top + 1));
                tqdms.iter().take(nrow as usize - 1).for_each(|data| {
                    print!(
                        "{:ncol$}",
                        data.lock().unwrap().print(),
                        ncol = ncol as usize
                    )
                });
            }

            use std::io::Write;
            std::io::stdout().flush().unwrap();
        }

        next
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
        let mut tqdms = TQDM.lock().unwrap();

        let nrows = terminal_size().1;
        let top = nrows.checked_sub(tqdms.len() as u16).unwrap_or(0);

        use termion::cursor;
        print!("{}", cursor::Goto(1, top + 1));
        println!("{}", self.data.lock().unwrap().print());

        tqdms.retain(|this| !Arc::ptr_eq(this, &self.data));
    }
}

/* -------------------------------------------------------------------------- */
/*                                   PRIVATE                                  */
/* -------------------------------------------------------------------------- */

#[macro_use]
extern crate lazy_static;
lazy_static! {
    static ref TQDM: Mutex<Vec<Arc<Mutex<TqdmData>>>> = Mutex::new(Vec::new());
}

fn terminal_size() -> (u16, u16) {
    use termion::terminal_size;
    terminal_size().unwrap()
}

fn format_time(mut duration: usize) -> String {
    duration = duration / 1000;
    match duration / 3600 {
        0 => format!("{:02}:{:02}", duration / 60 % 60, duration % 60),
        x => format!("{:02}:{:02}:{:02}", x, duration / 60 % 60, duration % 60),
    }
}

struct TqdmData {
    start: SystemTime,
    step: usize,
    size: Option<usize>,

    style: String,
    width: Option<usize>,
}

impl TqdmData {
    fn full(&self, length: usize) -> String {
        match self.style.as_str() {
            "ascii" => "#",
            "block" => "█",
            "balloon" => "*",
            unknown => panic!("unknown style '{}'", unknown),
        }
        .repeat(length)
    }

    fn edge(&self, frac: f64) -> char {
        match self.style.as_str() {
            "ascii" => "0123456789".chars().nth((10.0 * frac) as usize),
            "block" => " ▏▎▍▌▋▊▉".chars().nth((8.0 * frac) as usize),
            "balloon" => ".oO@".chars().nth((4.0 * frac) as usize),
            unknown => panic!("unknown style '{}'", unknown),
        }
        .unwrap()
    }

    fn print(&self) -> String {
        let elapsed = SystemTime::now()
            .duration_since(self.start)
            .as_ref()
            .map_or(0, Duration::as_millis);

        match self.size {
            Some(remain) => {
                let step = self.step;
                let total = step + remain;
                let percent = (step as f64 / total as f64).clamp(0.0, 1.0);

                let progress = (100.0 * percent) as usize;
                let head = format!("{:>3}%|", progress);

                let time = format_time(elapsed as usize);
                let eta = format_time(elapsed as usize * remain / step);
                let rate = step as f64 / elapsed as f64 * 1000.0;
                let tail = format!("| {}/{} [{}<{}, {:.02}it/s]", step, total, time, eta, rate);

                let width = self.width.unwrap_or_else(|| terminal_size().0 as usize);
                let length = width.checked_sub(head.len() + tail.len()).unwrap_or(0);
                let mut body = self.full((length as f64 * percent) as usize);
                body.push(self.edge((length as f64 * percent).fract()));

                let body: String = body.chars().take(length).collect();
                format!("{}{:length$}{}", head, body, tail, length = length)
            }

            None => {
                let step = self.step;
                let time = format_time(elapsed as usize);
                let rate = step as f64 / elapsed as f64 * 1000.0;
                format!("{}it [{}, {:.02}it/s]", step, time, rate)
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
    fn chain() {
        for _i in (0..).take(1000).tqdm().take(500).tqdm() {
            std::thread::sleep(Duration::from_millis(10));
        }
    }

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
        for _i in tqdm(tqdm(0..100).take(50)) {
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    #[test]
    fn parallel() {
        let threads: Vec<_> = [(200, "ascii"), (400, "balloon"), (100, "block")]
            .iter()
            .map(|(its, style)| {
                std::thread::spawn(move || {
                    for _i in tqdm(0..*its).style(style).width(Some(82)) {
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
            "Tqdm: {:.02}it/s",
            ntest as f64 / SystemTime::now().duration_since(start).unwrap().as_millis() as f64
                * 1000.0
        );
    }
}
