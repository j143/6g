//! Level-2 comparisons against external MAC/RAN reference traces.

use sixg_common::baseline::{BaselineDataset, BaselineSource};
use sixg_common::types::{SnrLinear, UeId};

use crate::harq::{HarqManager, MAX_RETX};
use crate::scheduler::{Scheduler, SchedulingPolicy, UeChannelState, jain_fairness};

fn baseline_path(filename: &str) -> Option<std::path::PathBuf> {
    std::env::var_os("SIXG_BASELINES").map(|root| {
        let root = std::path::PathBuf::from(root);
        if root.is_absolute() {
            root.join(filename)
        } else {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(root)
                .join(filename)
        }
    })
}

#[test]
fn ideal_chase_model_matches_analytical_reference() {
    let Some(path) = baseline_path("srsran_harq_chase_rounds.csv") else {
        return;
    };
    let dataset = BaselineDataset::from_csv(
        path,
        BaselineSource {
            system: "srsRAN-inspired ideal Chase model",
            metric: "harq_chase_rounds",
            citation: "https://www.srsran.com",
        },
    )
    .expect("SIXG_BASELINES srsRAN CSV must be readable");

    let simulated: Vec<(f64, f64)> = dataset
        .points
        .iter()
        .map(|point| {
            let mut manager = HarqManager::new();
            manager.start(0);
            let mut rounds = 0;
            while rounds < MAX_RETX {
                rounds += 1;
                if manager.chase_combine(0, point.input_parameter) {
                    break;
                }
            }
            (point.input_parameter, rounds as f64)
        })
        .collect();

    let result = dataset.compare_values(&simulated, 0.0);
    assert!(result.passed(), "{}", result.summary());
}

#[test]
fn round_robin_fairness_matches_analytical_reference() {
    let Some(path) = baseline_path("ns3_rr_jain_fairness.csv") else {
        return;
    };
    let dataset = BaselineDataset::from_csv(
        path,
        BaselineSource {
            system: "ns-3-inspired equal-allocation model",
            metric: "jain_fairness_rr",
            citation: "https://gitlab.com/cttc-lena/nr",
        },
    )
    .expect("SIXG_BASELINES ns-3 CSV must be readable");

    let simulated: Vec<(f64, f64)> = dataset
        .points
        .iter()
        .map(|point| {
            let ue_count = point.input_parameter as usize;
            let ues: Vec<_> = (0..ue_count).map(|id| UeId(id as u64)).collect();
            let states: Vec<_> = ues
                .iter()
                .copied()
                .map(|ue| UeChannelState::new(ue, SnrLinear::new(1.0)))
                .collect();
            let mut scheduler = Scheduler::with_policy(SchedulingPolicy::RoundRobin);
            let mut throughput = vec![0.0; ue_count];
            for _ in 0..100 {
                for assignment in scheduler.schedule_with_csi(&states, 100) {
                    throughput[assignment.ue.0 as usize] += assignment.rb_count as f64;
                }
            }
            (point.input_parameter, jain_fairness(&throughput))
        })
        .collect();

    let result = dataset.compare_values(&simulated, 5.0);
    assert!(result.passed(), "{}", result.summary());
}

#[test]
fn fiveg_lena_ue_sweep_preserves_load_trend() {
    let mut rows = include_str!("../../../baselines/5glena_v5.1_ue_sweep.csv")
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.trim().starts_with('#'))
        .skip(1)
        .map(|line| {
            let fields: Vec<_> = line.split(',').collect();
            assert_eq!(fields.len(), 3, "expected three fields in {line:?}");
            (
                fields[0]
                    .parse::<usize>()
                    .expect("UE count must be numeric"),
                fields[1]
                    .parse::<f64>()
                    .expect("throughput must be numeric"),
                fields[2].parse::<f64>().expect("delay must be numeric"),
            )
        });

    let first = rows.next().expect("5G-LENA sweep must not be empty");
    let mut previous = first;
    let mut row_count = 1;
    for current in rows {
        assert!(current.0 > previous.0, "UE counts must increase");
        assert!(
            current.1 < previous.1,
            "throughput should decrease as UE load rises: {previous:?} -> {current:?}"
        );
        assert!(
            current.2 > previous.2,
            "delay should increase as UE load rises: {previous:?} -> {current:?}"
        );
        previous = current;
        row_count += 1;
    }
    assert_eq!(row_count, 4, "expected the 5/10/20/50 UE sweep");
}
