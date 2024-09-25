use criterion::{Criterion, criterion_group, criterion_main};

fn criterion_decompress_ten_million_double_precision_values(c: &mut Criterion) {
    todo!()
}

fn criterion_decompress_hundred_million_double_precision_values(c: &mut Criterion) {
    todo!()
}

criterion_group!(benches,
    criterion_decompress_ten_million_double_precision_values,
    criterion_decompress_hundred_million_double_precision_values,
);

criterion_main!(benches);