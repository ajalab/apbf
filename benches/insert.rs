use std::fmt;

use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput};
use rand::prelude::*;

use apbf::APBF;

const ELEMENTS: u64 = 1024;

struct Setting {
    k: usize,
    l: usize,
    m: usize,
}

impl Setting {
    fn new(k: usize, l: usize, m: usize) -> Self {
        Setting { k, l, m }
    }
}

impl fmt::Display for Setting {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "k={}, l={}, m={}", self.k, self.l, self.m)
    }
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert");

    let settings = vec![
        Setting::new(10, 7, 64),
        Setting::new(10, 7, 256),
        Setting::new(10, 7, 1024),
        Setting::new(14, 11, 64),
        Setting::new(14, 11, 256),
        Setting::new(14, 11, 1024),
    ];

    group.throughput(Throughput::Elements(ELEMENTS));

    let mut rng = StdRng::from_seed([0u8; 32]);
    let input = (0..ELEMENTS).map(|_| rng.gen()).collect::<Vec<usize>>();

    for s in settings {
        group.bench_with_input(BenchmarkId::new("size", &s), &s, |b, s| {
            b.iter_batched(
                || APBF::new(s.k, s.l, s.m),
                |mut apbf| {
                    for &n in &input {
                        apbf.insert(n);
                    }
                },
                BatchSize::SmallInput,
            )
        });
    }
}

criterion_group!(benches, bench);
criterion_main!(benches);
