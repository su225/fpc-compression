use criterion::{Criterion, criterion_group, criterion_main};

fn criterion_compress_ten_million_double_precision_values(c: &mut Criterion) {
    unimplemented!("compress 10M double precision elements")
}

fn criterion_compress_hundred_million_double_precision_values(c: &mut Criterion) {
    unimplemented!("compress 100M double precision elements")
}

criterion_group!(benches,
    criterion_compress_ten_million_double_precision_values,
    criterion_compress_hundred_million_double_precision_values,
);

criterion_main!(benches);