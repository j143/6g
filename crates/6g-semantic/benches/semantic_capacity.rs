//! Capacity benchmarks for the 6G semantic layer.
//!
//! Benchmarks the hot-path computations for:
//! - `semantic_encode_throughput`: encoding throughput at various payload sizes
//!
//! Run with:
//!   cargo bench -p sixg-semantic --bench semantic_capacity

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sixg_semantic::codec::TextSemanticCodec;
use sixg_semantic::SemanticCodec;

/// Benchmark the semantic encoder at various payload sizes.
///
/// Metric: MB/s encoding throughput.
fn bench_semantic_encode_throughput(c: &mut Criterion) {
    let codec = TextSemanticCodec;
    let mut group = c.benchmark_group("semantic_encode_throughput");

    // Payload sizes: 64 B, 512 B, 4 KB, 16 KB, 64 KB
    for &size in &[64usize, 512, 4_096, 16_384, 65_536] {
        let payload: Vec<u8> = (0..size).map(|i| b'a' + (i % 26) as u8).collect();

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::new("payload_bytes", size), &payload, |b, p| {
            b.iter(|| {
                let encoded = codec.encode(black_box(p));
                black_box(encoded)
            })
        });
    }
    group.finish();
}

/// Benchmark the semantic decoder at various payload sizes.
fn bench_semantic_decode_throughput(c: &mut Criterion) {
    let codec = TextSemanticCodec;
    let mut group = c.benchmark_group("semantic_decode_throughput");

    // Encoded output is always 64 bytes for TextSemanticCodec
    let encoded = codec.encode(b"the quick brown fox jumps over the lazy dog");

    group.throughput(Throughput::Bytes(encoded.len() as u64));
    group.bench_function("decode_64b_signature", |b| {
        b.iter(|| {
            let decoded = codec.decode(black_box(&encoded));
            black_box(decoded)
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_semantic_encode_throughput,
    bench_semantic_decode_throughput
);
criterion_main!(benches);
