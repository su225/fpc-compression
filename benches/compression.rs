use std::time::{Duration, Instant};
use criterion::{black_box, Criterion, criterion_group, criterion_main};
use rand::prelude::{SliceRandom, StdRng};
use rand::SeedableRng;
use fpc_compression::compress_into;

fn deterministic_shuffle(vec: &mut Vec<f64>, seed: u64) {
    let mut rng = StdRng::seed_from_u64(seed);
    vec.shuffle(&mut rng);
}

fn criterion_compress_ten_million_double_precision_values(c: &mut Criterion) {
    let table_sizes: Vec<u64> = vec![32, 64, 128, 256, 512, 1024, 2048, 4096, 8192,
                                     8192*2, 8192*4, 8192*8, 8192*16, 8192*32];
    let mut vals = (0..10_000_000).map(|x| x as f64).collect::<Vec<f64>>();
    deterministic_shuffle(&mut vals, 124840); // random seed
    let mut bench_group = c.benchmark_group("compress_10M");
    for &table_size in &table_sizes {
        bench_group.bench_with_input(
            criterion::BenchmarkId::new("table_size", table_size),
            &table_size,
            |b, &tsz| { b.iter_custom(|iters| {
                let mut total_elapsed = Duration::new(0, 0);
                for _ in 0..iters {
                    let mut encoding = vec![0_u8; (vals.len() + 1)/2];
                    let mut residual = Vec::with_capacity(size_of::<f64>() * vals.len());
                    {
                        let start = Instant::now();
                        black_box(compress_into(tsz, &vals, &mut encoding, &mut residual));
                        total_elapsed += start.elapsed();
                    }
                }
                total_elapsed
            })});
    }
}

criterion_group!(benches,
    criterion_compress_ten_million_double_precision_values,
);

criterion_main!(benches);