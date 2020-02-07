use bloom::dynamic::DynamicBloom;
use criterion::{criterion_group, criterion_main, Criterion};
use rand::distributions::Uniform;
use rand::Rng;

fn insert_expected(c: &mut Criterion) {
    let mut group = c.benchmark_group("Insert with capacity");
    group.sample_size(15);

    let mut gen = rand::thread_rng();

    for i in [5_000, 10_000, 30_000, 100_000].iter() {
        let mut f = DynamicBloom::new(1_000, 0.05);
        group.bench_with_input(
            format!("insert {}k items with 1k expected capacity", i),
            i,
            |b, i| {
                b.iter(|| {
                    for _ in 0..*i {
                        let item = gen.sample(Uniform::new(0, 6_000_000));
                        f.add(&item)
                    }
                })
            },
        );

        let mut f = DynamicBloom::new(50_000, 0.05);
        group.bench_with_input(
            format!("insert {}k items with 50k expected capacity", i),
            i,
            |b, i| {
                b.iter(|| {
                    for _ in 0..*i {
                        let item = gen.sample(Uniform::new(0, 6_000_000));
                        f.add(&item)
                    }
                })
            },
        );
    }

    group.finish();
}

use std::sync::RwLock;

fn grow_buckets_and_look_up(c: &mut Criterion) {
    let mut group = c.benchmark_group("grow_buckets_and_look_up");

    let gen = rand::thread_rng();
    let mut f = DynamicBloom::new(1_000, 0.05);

    let random_200k = gen
        .sample_iter(Uniform::new(0, 6_000_000))
        .take(200_000)
        .collect::<Vec<u32>>();
    for ref i in random_200k.iter() {
        f.add(i)
    }
    assert!(f.assert_fp());

    let random_200k = RwLock::new(random_200k);
    group.bench_function(
        format!(
            "Get 100 random items from {} buckets with capacity 1000",
            f.len()
        ),
        |b| {
            b.iter(|| {
                for l in random_200k.read().unwrap().iter().take(100) {
                    assert!(f.get(l));
                }
            })
        },
    );
    group.finish();
}

criterion_group!(benches, insert_expected, grow_buckets_and_look_up);
criterion_main!(benches);
