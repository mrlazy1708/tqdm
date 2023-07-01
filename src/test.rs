use crate::*;

#[test]

fn base() {
    tqdm(0..100).for_each(|_| thread::sleep(time::Duration::from_secs_f64(0.01)));
}

#[test]
#[ignore]

fn very_slow() {
    tqdm(0..100).for_each(|_| thread::sleep(time::Duration::from_secs_f64(10.0)));
}

#[test]
#[ignore]

fn infinite() {
    for _ in tqdm(0..).desc(Some("infinite")) {
        thread::sleep(time::Duration::from_secs_f64(0.1));
    }
}

#[test]

fn breaking() {
    for i in tqdm(0..100).desc(Some("breaking")) {
        thread::sleep(time::Duration::from_secs_f64(0.1));
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
        thread::spawn(move || {
            for _i in tqdm(0..its)
                .style(style)
                .width(Some(82))
                .desc(Some(format!("par {}", idx).as_str()))
            {
                thread::sleep(time::Duration::from_millis(10));
            }
        })
    })
    .collect();

    for handle in threads {
        handle.join().unwrap();
    }
}

#[test]

fn overflow() {
    let threads: Vec<_> = (0..30)
        .map(|idx| {
            thread::spawn(move || {
                for _i in tqdm(0..100)
                    .style(Style::ASCII)
                    .width(Some(82))
                    .desc(Some(format!("par {}", idx).as_str()))
                {
                    thread::sleep(time::Duration::from_millis(10));
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
    const N: usize = 100000000;
    fn speed(start: time::SystemTime) -> f64 {
        N as f64
            / time::SystemTime::now()
                .duration_since(start)
                .unwrap()
                .as_millis() as f64
            * 1000.0
    }

    let start = time::SystemTime::now();
    for _i in 0..N {}
    println!("baseline: {:.02}it/s", speed(start));

    let start = time::SystemTime::now();
    for _i in tqdm(0..N) {}
    println!("w/ tqdm: {:.02}it/s", speed(start));
}
