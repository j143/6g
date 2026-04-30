//! Capacity benchmarks for the 6G Core Network.
//!
//! Benchmarks the hot-path computations for:
//! - `core_register_burst`: UE registration burst throughput
//!
//! Run with:
//!   cargo bench -p sixg-core --bench core_capacity

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sixg_common::types::UeId;
use sixg_core::{nssf::SliceType, smf::PduSessionType, CoreNetwork};

/// Benchmark UE registration burst throughput.
///
/// Metric: registrations per second.
fn bench_core_register_burst(c: &mut Criterion) {
    let mut group = c.benchmark_group("core_register_burst");

    for &n_ues in &[10usize, 50, 100, 250, 500, 1000] {
        group.throughput(Throughput::Elements(n_ues as u64));
        group.bench_with_input(BenchmarkId::new("ues", n_ues), &n_ues, |b, &n| {
            b.iter(|| {
                let mut core = CoreNetwork::new();
                let mut registered = 0usize;
                for i in 0..n {
                    if core.register_ue(black_box(UeId(i as u64)), 1) {
                        registered += 1;
                    }
                }
                black_box(registered)
            })
        });
    }
    group.finish();
}

/// Benchmark full session establishment (registration + PDU session setup).
fn bench_session_establishment(c: &mut Criterion) {
    let mut group = c.benchmark_group("session_establishment");

    for &n_ues in &[10usize, 50, 100] {
        group.throughput(Throughput::Elements(n_ues as u64));
        group.bench_with_input(BenchmarkId::new("ues", n_ues), &n_ues, |b, &n| {
            b.iter(|| {
                let mut core = CoreNetwork::new();
                let mut sessions = 0usize;
                for i in 0..n {
                    let ue = UeId(i as u64);
                    if core.register_ue(ue, 1) {
                        if core
                            .establish_session(ue, SliceType::EmBb, PduSessionType::Ip)
                            .is_some()
                        {
                            sessions += 1;
                        }
                    }
                }
                black_box(sessions)
            })
        });
    }
    group.finish();
}

/// Benchmark UPF unknown-flow buffering (user-plane-first path).
fn bench_upf_unknown_flow(c: &mut Criterion) {
    let mut group = c.benchmark_group("upf_unknown_flow");
    use sixg_core::upf::Upf;

    for &n_ues in &[100usize, 500, 1000] {
        group.throughput(Throughput::Elements(n_ues as u64));
        group.bench_with_input(BenchmarkId::new("ues", n_ues), &n_ues, |b, &n| {
            b.iter(|| {
                let mut upf = Upf::new();
                let payload = b"first packet payload for lazy establishment test";
                let mut actions = 0usize;
                for i in 0..n {
                    use sixg_core::upf::FlowAction;
                    let action = upf.forward_unknown_flow(UeId(i as u64), black_box(payload));
                    if action == FlowAction::TriggerEstablishment(UeId(i as u64)) {
                        actions += 1;
                    }
                }
                black_box(actions)
            })
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_core_register_burst,
    bench_session_establishment,
    bench_upf_unknown_flow
);
criterion_main!(benches);
