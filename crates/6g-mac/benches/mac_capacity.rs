//! Capacity benchmarks for the 6G MAC layer.
//!
//! Benchmarks the hot-path computations for:
//! - `scheduler_scale`: scheduling throughput at various UE counts and TTI counts
//! - `harq_rounds_distribution`: HARQ Chase Combining across SNR values
//!
//! Run with:
//!   cargo bench -p sixg-mac --bench mac_capacity

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sixg_common::types::{SnrLinear, UeId};
use sixg_mac::{
    harq::ChaseCombineBuffer,
    scheduler::{jain_fairness, Scheduler, SchedulingPolicy, UeChannelState},
};

/// Benchmark the scheduler across a range of UE counts and TTI depths.
///
/// Metric: TTIs per second (throughput of the scheduling loop).
fn bench_scheduler_scale(c: &mut Criterion) {
    let mut group = c.benchmark_group("scheduler_scale");

    for &n_ues in &[8usize, 64, 128, 256, 512] {
        // Build channel states with varied SNR (linear 1..n_ues).
        let states: Vec<UeChannelState> = (0..n_ues)
            .map(|i| UeChannelState::new(UeId(i as u64), SnrLinear::new(1.0 + i as f64 * 0.5)))
            .collect();

        const TOTAL_RBS: usize = 273;
        const N_TTI: usize = 100;

        group.throughput(Throughput::Elements(N_TTI as u64));

        // AI-native (Q-bandit) policy
        group.bench_with_input(
            BenchmarkId::new("ai_native_ues", n_ues),
            &states,
            |b, sts| {
                b.iter(|| {
                    let mut sched = Scheduler::with_policy(SchedulingPolicy::AiNative);
                    let mut total = 0usize;
                    for _tti in 0..N_TTI {
                        let assignments = sched.schedule_with_csi(black_box(sts), TOTAL_RBS);
                        total += assignments.len();
                        for (idx, a) in assignments.iter().enumerate() {
                            sched.observe_reward(
                                idx,
                                black_box(sts[idx % sts.len()].snr),
                                a.rb_count as f64 * 1e6,
                            );
                        }
                    }
                    black_box(total)
                })
            },
        );

        // Round-Robin policy (baseline comparison)
        group.bench_with_input(
            BenchmarkId::new("round_robin_ues", n_ues),
            &states,
            |b, sts| {
                b.iter(|| {
                    let mut sched = Scheduler::with_policy(SchedulingPolicy::RoundRobin);
                    let mut total = 0usize;
                    for _tti in 0..N_TTI {
                        let a = sched.schedule_with_csi(black_box(sts), TOTAL_RBS);
                        total += a.len();
                    }
                    black_box(total)
                })
            },
        );
    }
    group.finish();
}

/// Benchmark HARQ Chase Combining across SNR values.
///
/// Metric: HARQ combining rounds needed to reach the decode threshold
/// (measures the distribution shape, not just the scalar output).
fn bench_harq_rounds_distribution(c: &mut Criterion) {
    let mut group = c.benchmark_group("harq_rounds_distribution");

    // SNR linear values from −5 dB to 20 dB
    let snr_values: Vec<f64> = (-5..=20).map(|db| 10f64.powf(db as f64 / 10.0)).collect();

    const N_SAMPLES: usize = 10_000;

    group.throughput(Throughput::Elements(N_SAMPLES as u64));
    group.bench_function("chase_combining_10k_samples", |b| {
        b.iter(|| {
            let mut rounds_total = 0u64;
            for (i, &snr) in snr_values
                .iter()
                .enumerate()
                .take(N_SAMPLES % snr_values.len() + 1)
            {
                let mut buf = ChaseCombineBuffer::default();
                let mut rounds = 0u8;
                while !buf.can_decode() && rounds < 8 {
                    buf.combine(black_box(snr * (1.0 + 0.01 * i as f64)));
                    rounds += 1;
                }
                rounds_total += rounds as u64;
            }
            black_box(rounds_total)
        })
    });

    group.finish();
}

/// Benchmark Jain fairness computation at various UE counts.
fn bench_jain_fairness(c: &mut Criterion) {
    let mut group = c.benchmark_group("jain_fairness");

    for &n_ues in &[8usize, 64, 256, 512, 1024] {
        let throughputs: Vec<f64> = (1..=n_ues).map(|i| i as f64 * 1e6).collect();
        group.bench_with_input(BenchmarkId::new("ues", n_ues), &throughputs, |b, tp| {
            b.iter(|| black_box(jain_fairness(black_box(tp))))
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_scheduler_scale,
    bench_harq_rounds_distribution,
    bench_jain_fairness
);
criterion_main!(benches);
