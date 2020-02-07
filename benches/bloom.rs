use bit_vec::BitVec;
use bloom::BloomFilter;
use criterion::{criterion_group, criterion_main, Criterion};
use rand::distributions::Uniform;
use rand::Rng;

fn insert_item(b: &mut Criterion) {
    let mut f = BloomFilter::with_fp_size(0.05, 50000);
    let mut gen = rand::thread_rng();
    let item = gen.sample(Uniform::new(0, 6_000_000));

    b.bench_function("add item", |b| b.iter(|| f.add(&item)));
}

fn insert_5k_items(b: &mut Criterion) {
    let mut f = BloomFilter::with_fp_size(0.05, 50000);
    let mut gen = rand::thread_rng();

    b.bench_function("5k items", |b| {
        b.iter(|| {
            for _ in 0..5000 {
                let item = gen.sample(Uniform::new(0, 6_000_000));
                f.add(&item)
            }
        })
    });
}

fn calc_hashes(b: &mut Criterion) {
    let f = BloomFilter::with_fp_size(0.05, 50000);
    let mut gen = rand::thread_rng();
    let item = gen.sample(Uniform::new(0, 6_000_000));
    b.bench_function("compute_hashes", |b| b.iter(|| f.compute_hashes(&item)));
}

fn insert_into_bitvec(b: &mut Criterion) {
    let mut v = BitVec::from_elem(16, false);
    b.bench_function("bitvec set raw", |b| {
        b.iter(|| {
            for i in 0..6 {
                v.set(i, true)
            }
        })
    });
}

criterion_group!(
    benches,
    insert_item,
    insert_5k_items,
    calc_hashes,
    insert_into_bitvec
);
criterion_main!(benches);
