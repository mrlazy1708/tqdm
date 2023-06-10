use crate::*;

#[test]

fn example() {
    tqdm(0..100).for_each(|_| thread::sleep(time::Duration::from_secs_f64(0.01)));
}

#[test]
#[ignore]

fn very_slow() {
    tqdm(0..100).for_each(|_| thread::sleep(time::Duration::from_secs_f64(10.0)));
}

#[test]

fn range() {
    for i in tqdm(0..100).desc(Some("range")).width(Some(82)) {
        thread::sleep(time::Duration::from_secs_f64(0.1));
        if i % 10 == 0 {
            println!("break #{}", i);
        }
    }
}

#[test]
#[ignore]

fn infinite() {
    for i in tqdm(0..).desc(Some("infinite")) {
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

fn performance() {
    let ntest = 100000000;

    let start = time::SystemTime::now();
    for _i in 0..ntest {}
    println!(
        "Baseline: {:.02}it/s",
        ntest as f64
            / time::SystemTime::now()
                .duration_since(start)
                .unwrap()
                .as_millis() as f64
            * 1000.0
    );

    let start = time::SystemTime::now();
    for _i in tqdm(0..ntest) {}
    println!(
        "With tqdm: {:.02}it/s",
        ntest as f64
            / time::SystemTime::now()
                .duration_since(start)
                .unwrap()
                .as_millis() as f64
            * 1000.0
    );
}
