use std::time::{Duration, Instant};

use criterion::{black_box, Criterion, criterion_group, criterion_main};
use rand::prelude::{SliceRandom, StdRng};
use rand::SeedableRng;

use fpc_compression::decompress_into;

fn deterministic_shuffle(vec: &mut Vec<f64>, seed: u64) {
    let mut rng = StdRng::seed_from_u64(seed);
    vec.shuffle(&mut rng);
}

fn criterion_decompress_ten_million_double_precision_values(c: &mut Criterion) {
    let table_sizes: Vec<u64> = vec![32, 64, 128, 256, 512, 1024, 2048, 4096, 8192,
                                     8192*2, 8192*4, 8192*8, 8192*16, 8192*32];
    let mut vals = (0..10_000_000).map(|x| x as f64).collect::<Vec<f64>>();
    deterministic_shuffle(&mut vals, 124840); // random seed
    let mut bench_group = c.benchmark_group("decompress_10M");
    for &table_size in &table_sizes {
        let compressed = fpc_compression::compress(table_size, &vals);
        bench_group.bench_with_input(
            criterion::BenchmarkId::new("table_size", table_size),
            &table_size,
            |b, &tsz| { b.iter_custom(|iters| {
                let mut total_elapsed = Duration::new(0, 0);
                for _ in 0..iters {
                    let mut result = Vec::with_capacity(vals.len());
                    {
                        let start = Instant::now();
                        black_box(decompress_into(tsz, &compressed, &mut result));
                        total_elapsed += start.elapsed();
                    }
                }
                total_elapsed
            })},
        );
    }
}

criterion_group!(benches,
    criterion_decompress_ten_million_double_precision_values,
);

criterion_main!(benches);