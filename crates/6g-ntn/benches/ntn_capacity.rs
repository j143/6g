//! Capacity benchmarks for the 6G NTN (Non-Terrestrial Networks) layer.
//!
//! Benchmarks the hot-path computations for:
//! - `ntn_handover_latency`: handover evaluation across altitude tiers
//! - `propagation_delay_sweep`: delay computation across the full altitude range
//!
//! Run with:
//!   cargo bench -p sixg-ntn --bench ntn_capacity

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sixg_common::types::Position3D;
use sixg_common::types::{Distance, PowerDb, UeId};
use sixg_ntn::{
    handover::{leo_propagation_delay_ms, HandoverDecision, HandoverTrigger, NtnHandoverManager},
    NtnLayer, NtnNode,
};

/// Benchmark handover evaluation across altitude tiers (LEO / HAPS / GEO).
///
/// Metric: handover decisions per second.
fn bench_ntn_handover_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("ntn_handover_latency");
    let mgr = NtnHandoverManager::new();

    // Altitude tiers: LEO 550 km, HAPS 20 km, MEO 8000 km, GEO 35786 km
    let altitudes: &[(&str, f64)] = &[
        ("leo_550km", 550_000.0),
        ("haps_20km", 20_000.0),
        ("meo_8000km", 8_000_000.0),
        ("geo_35786km", 35_786_000.0),
    ];

    for &(label, alt_m) in altitudes {
        let delay_ms = leo_propagation_delay_ms(Distance::from_m(alt_m));
        let triggers = vec![HandoverTrigger::PropagationDelayExceeded { delay_ms }];

        group.bench_with_input(
            BenchmarkId::new("altitude", label),
            &triggers,
            |b, trigs| {
                b.iter(|| {
                    let decision = mgr.evaluate(black_box(UeId(1)), black_box(trigs));
                    black_box(decision)
                })
            },
        );
    }
    group.finish();
}

/// Benchmark propagation delay computation across the full altitude range.
fn bench_propagation_delay_sweep(c: &mut Criterion) {
    let mut group = c.benchmark_group("propagation_delay_sweep");

    // Sweep from 500 km to 40 000 km in 1000 steps
    let altitudes: Vec<Distance> = (0..1000)
        .map(|i| Distance::from_m(500_000.0 + i as f64 * 39_500.0))
        .collect();

    group.throughput(Throughput::Elements(altitudes.len() as u64));
    group.bench_function("delay_sweep_1000_altitudes", |b| {
        b.iter(|| {
            let mut acc = 0.0f64;
            for &alt in &altitudes {
                acc += leo_propagation_delay_ms(black_box(alt));
            }
            black_box(acc)
        })
    });

    group.finish();
}

/// Benchmark NTN layer node management at scale.
fn bench_ntn_node_fleet(c: &mut Criterion) {
    let mut group = c.benchmark_group("ntn_node_fleet");

    for &n_nodes in &[10usize, 100, 500, 1000] {
        group.throughput(Throughput::Elements(n_nodes as u64));
        group.bench_with_input(BenchmarkId::new("nodes", n_nodes), &n_nodes, |b, &n| {
            b.iter(|| {
                let mut layer = NtnLayer::new();
                for i in 0..n {
                    let alt = 550_000.0 + (i as f64 * 10.0);
                    layer.add_node(NtnNode::leo_satellite(
                        i as u64,
                        Position3D::new(0.0, i as f64 * 100.0, alt),
                    ));
                }
                black_box(layer.node_count())
            })
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_ntn_handover_latency,
    bench_propagation_delay_sweep,
    bench_ntn_node_fleet
);
criterion_main!(benches);
