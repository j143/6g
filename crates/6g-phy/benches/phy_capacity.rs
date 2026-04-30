//! Capacity benchmarks for the 6G PHY layer.
//!
//! Benchmarks the hot-path computations for:
//! - `path_loss_throughput`: free-space + absorption path loss at various batch sizes
//! - `ris_snr_sweep`: RIS-optimised SNR computation at various element counts
//!
//! Run with:
//!   cargo bench -p sixg-phy --bench phy_capacity

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sixg_common::types::{Distance, Frequency, SnrLinear};
use sixg_phy::{
    path_loss_db,
    ris::{RisChannel, RisConfig},
    waveform::{bpsk_ber_awgn, Waveform},
};
use sixg_common::types::SnrDb;

/// Benchmark path loss computation across a range of batch sizes.
///
/// Metric: nanoseconds per call (scalar path-loss evaluation).
fn bench_path_loss_throughput(c: &mut Criterion) {
    let freq = Frequency::from_ghz(28.0);
    let mut group = c.benchmark_group("path_loss_throughput");

    for &batch in &[1usize, 10, 100, 1_000, 10_000] {
        let distances: Vec<Distance> = (0..batch)
            .map(|i| Distance::from_m(10.0 + i as f64 * 10.0))
            .collect();
        group.throughput(Throughput::Elements(batch as u64));
        group.bench_with_input(BenchmarkId::new("batch", batch), &distances, |b, dists| {
            b.iter(|| {
                let mut acc = 0.0f64;
                for &d in dists {
                    acc += path_loss_db(d, black_box(freq)).as_db();
                }
                black_box(acc)
            })
        });
    }
    group.finish();
}

/// Benchmark RIS SNR sweep across a range of element counts.
///
/// Metric: microseconds per SNR optimisation call.
fn bench_ris_snr_sweep(c: &mut Criterion) {
    let mut group = c.benchmark_group("ris_snr_sweep");

    for &n_elements in &[16usize, 64, 256, 1024, 4096] {
        let side = ((n_elements as f64).sqrt() as usize).max(1);
        let ris_cfg = RisConfig {
            num_elements: n_elements,
            rows: side,
            columns: side,
            ..Default::default()
        };
        let channel = RisChannel::new(0.0001, 0.01, 0.01, ris_cfg);
        let snr_no_ris = SnrLinear::new(0.01);

        group.bench_with_input(
            BenchmarkId::new("elements", n_elements),
            &channel,
            |b, ch| {
                b.iter(|| {
                    let snr_opt = ch.snr_opt_ris(black_box(snr_no_ris));
                    black_box(snr_opt)
                })
            },
        );
    }
    group.finish();
}

/// Benchmark BER computation for OTFS vs CP-OFDM.
fn bench_ber_computation(c: &mut Criterion) {
    let mut group = c.benchmark_group("ber_computation");
    let snr_points: Vec<SnrDb> = (-5..=20).map(|db| SnrDb(db as f64)).collect();

    let otfs = Waveform::Otfs {
        delay_bins: 64,
        doppler_bins: 16,
    };
    let ofdm = Waveform::CpOfdm {
        subcarrier_spacing_khz: 120,
        fft_size: 2048,
    };

    group.bench_function("otfs_ber_sweep", |b| {
        b.iter(|| {
            let mut acc = 0.0f64;
            for &snr in &snr_points {
                acc += otfs.ber_awgn(black_box(snr));
            }
            black_box(acc)
        })
    });

    group.bench_function("ofdm_ber_sweep", |b| {
        b.iter(|| {
            let mut acc = 0.0f64;
            for &snr in &snr_points {
                acc += ofdm.ber_awgn(black_box(snr));
            }
            black_box(acc)
        })
    });

    group.bench_function("bpsk_ber_awgn_single", |b| {
        b.iter(|| black_box(bpsk_ber_awgn(black_box(SnrDb(10.0)))))
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_path_loss_throughput,
    bench_ris_snr_sweep,
    bench_ber_computation
);
criterion_main!(benches);
