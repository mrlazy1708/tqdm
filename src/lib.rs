use std::sync::*;
use std::time::*;

/* -------------------------------------------------------------------------- */
/*                                   PUBLIC                                   */
/* -------------------------------------------------------------------------- */

pub fn tqdm<Item: 'static, Iter: Iterator<Item = Item> + 'static>(iter: Iter) -> Tqdm<Item, Iter> {
    let data = Arc::new(Mutex::new(TqdmData {
        start: SystemTime::now(),

        step: 0,
        size: None,

        style: "block".to_string(),
        width: None,
    }));
    TQDM.lock().unwrap().push(data.clone());

    Tqdm {
        iter,
        data,
        prev: UNIX_EPOCH,

        step: 0,
        size: None,
    }
}

pub struct Tqdm<Item, Iter: Iterator<Item = Item>> {
    iter: Iter,
    data: Arc<Mutex<TqdmData>>,
    prev: SystemTime,

    step: usize,
    size: Option<usize>,
}

impl<Item, Iter: Iterator<Item = Item>> Tqdm<Item, Iter> {
    pub fn style(self, style: &str) -> Self {
        self.data.lock().unwrap().style = style.to_string();
        self
    }

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

        let duration = SystemTime::now().duration_since(self.prev).unwrap();
        if duration > Duration::from_millis(40) {
            {
                let mut data = self.data.lock().unwrap();
                data.step = self.step;
                data.size = self.size.or_else(|| self.iter.size_hint().1);
            }

            {
                let tqdms = TQDM.lock().unwrap();

                use termion::{cursor, terminal_size};
                let height = terminal_size().unwrap().1;
                let height = height - (tqdms.len() as u16).min(height - 1);
                print!("{}", cursor::Goto(1, height));

                tqdms.iter().for_each(|data| data.lock().unwrap().print());
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

        use termion::{cursor, terminal_size};
        let height = terminal_size().unwrap().1;
        let height = height - (tqdms.len() as u16).min(height - 1);
        print!("{}", cursor::Goto(1, height));

        self.data.lock().unwrap().print();

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

struct TqdmData {
    start: SystemTime,
    step: usize,
    size: Option<usize>,

    style: String,
    width: Option<usize>,
}

impl TqdmData {
    fn time(&self, mut duration: usize) -> String {
        duration = duration / 1000;
        match duration / 3600 {
            0 => format!("{:02}:{:02}", duration / 60 % 60, duration % 60),
            x => format!("{:02}:{:02}:{:02}", x, duration / 60 % 60, duration % 60),
        }
    }

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

    fn print(&self) {
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

                let time = self.time(elapsed as usize);
                let eta = self.time(elapsed as usize * remain / step);
                let rate = step as f64 / elapsed as f64 * 1000.0;
                let tail = format!("| {}/{} [{}<{}, {:.02}it/s]", step, total, time, eta, rate);

                use termion::terminal_size;
                let width = terminal_size().ok().unwrap().0;
                let width = self.width.unwrap_or(width as usize);
                let length = width.checked_sub(head.len() + tail.len()).unwrap_or(0);
                let mut body = self.full((length as f64 * percent) as usize);
                body.push(self.edge((length as f64 * percent).fract()));

                let body: String = body.chars().take(length).collect();
                println!("{}{:length$}{}", head, body, tail, length = length);
            }

            None => {
                let step = self.step;
                let time = self.time(elapsed as usize);
                let rate = step as f64 / elapsed as f64 * 1000.0;
                println!("{}it [{}, {:.02}it/s]", step, time, rate);
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
}
