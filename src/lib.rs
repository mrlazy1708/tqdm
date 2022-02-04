use std::time::*;

pub fn tqdm<Item, Iter: Iterator<Item = Item>>(iter: Iter) -> Tqdm<Item, Iter> {
    Tqdm {
        iter,
        start: SystemTime::now(),
        it: 0,

        ascii: false,
        shape: None,
    }
}

pub struct Tqdm<Item, Iter: Iterator<Item = Item>> {
    iter: Iter,
    start: SystemTime,
    it: usize,

    pub ascii: bool,
    pub shape: Option<(u16, u16)>,
}
impl<Item, Iter: Iterator<Item = Item>> Iterator for Tqdm<Item, Iter> {
    type Item = Item;
    fn next(&mut self) -> Option<Self::Item> {
        self.it += 1;
        let result = self.iter.next();

        let elapsed = SystemTime::now()
            .duration_since(self.start)
            .as_ref()
            .map_or(0, Duration::as_millis);
        print!(
            "\r{}",
            match self.iter.size_hint().1.as_ref() {
                Some(remain) => {
                    let percent = (self.it as f64 / (self.it + remain) as f64).clamp(0.0, 1.0);

                    let progress = (100.0 * percent) as usize;
                    let head = format!("{:>2}%|", progress);

                    let it = self.it;
                    let total = it + remain;
                    let time = format_time(elapsed as usize);
                    let eta = format_time(elapsed as usize * remain / it);
                    let rate = it as f64 / elapsed as f64 * 1000.0;
                    let tail = format!("| {}/{} [{}<{}, {:.02}it/s]", it, total, time, eta, rate);

                    use termion::terminal_size;
                    let (width, _height) = self.shape.or(terminal_size().ok()).unwrap_or((0, 20));
                    let length = (width as usize)
                        .checked_sub(head.len() + tail.len())
                        .unwrap_or(0);
                    let mut body = self.full((length as f64 * percent) as usize);
                    body.push(self.extra((length as f64 * percent).fract()));

                    let body: String = body.chars().take(length).collect();
                    format!("{}{:length$}{}", head, body, tail, length = length)
                }
                None => {
                    let it = self.it;
                    let time = format_time(elapsed as usize);
                    let rate = self.it as f64 / elapsed as f64 * 1000.0;
                    format!("{}it [{}, {:.02}it/s]", it, time, rate)
                }
            }
        );

        use std::io::Write;
        std::io::stdout()
            .flush()
            .unwrap_or_else(|_err| println!(""));

        result
    }
}

impl<Item, Iter: Iterator<Item = Item>> Tqdm<Item, Iter> {
    fn full(&self, length: usize) -> String {
        match self.ascii {
            true => "#",
            false => "█",
        }
        .repeat(length)
    }
    fn extra(&self, frac: f64) -> char {
        match self.ascii {
            true => "0123456789".chars().nth((10.0 * frac) as usize),
            false => " ▏▎▍▌▋▊▉".chars().nth((8.0 * frac) as usize),
        }
        .unwrap()
    }
}

impl<Item, Iter: Iterator<Item = Item>> Tqdm<Item, Iter> {
    pub fn ascii(mut self, ascii: bool) -> Self {
        self.ascii = ascii;
        self
    }
    pub fn shape(mut self, shape: Option<(u16, u16)>) -> Self {
        self.shape = shape;
        self
    }
}

fn format_time(mut duration: usize) -> String {
    duration = duration / 1000;
    match duration / 3600 {
        0 => format!("{:02}:{:02}", duration / 60 % 60, duration % 60),
        x => format!("{:02}:{:02}:{:02}", x, duration / 60 % 60, duration % 60),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_finite() {
        for i in tqdm(0..100) {
            std::thread::sleep(Duration::from_millis(10));
            if i % 10 == 0 {
                println!("break #{}", i);
            }
        }
    }

    #[test]
    fn range_infinite() {
        for i in tqdm(0..) {
            std::thread::sleep(Duration::from_millis(10));
            if i % 10 == 0 {
                println!("break #{}", i);
            }
        }
    }
}
