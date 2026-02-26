## Non-Negotiable Rules

1. **NEVER create a new numeric type for frequency, power, distance, or time.**
   Always use the newtypes in `crates/6g-common/src/types.rs`.

   ```rust
   // Wrong:
   fn path_loss(dist_m: f64, freq_hz: f64) -> f64
   // Right:
   fn path_loss(dist: Distance, freq: Frequency) -> PowerDb
   ```

2. **NEVER use raw `f64` for physical quantities at public function boundaries.**
   Raw `f64`, `usize`, or `u64` are only acceptable for dimensionless counts and
   internal computations — not at `pub fn` API surfaces.

3. **NEVER add a crate dependency without checking if it already exists**
   in the workspace `Cargo.toml` under `[workspace.dependencies]`.

4. **`SystemConfig` in `crates/6g-common/src/config.rs` is the single feature-flag
   registry.** Gate new subsystems through it. Do not invent new config structs
   in individual crates.

5. **Every `pub fn` MUST have:**
   - A doc comment with physical units in the signature description.
   - At least one `#[test]` that verifies a known numerical result against a
     formula from the referenced paper.

6. **Before implementing anything in a crate, read `docs/<crate>.md`.**
   If the doc is inconsistent with what you're building, **UPDATE THE DOC FIRST**,
   then implement.

7. **Every physics module MUST implement the `Validate` trait** from
   `crates/6g-common/src/validation.rs`. The `validate()` function runs known-good
   numerical checks and returns a `ValidationResult`. CI calls these automatically.

## Crate Responsibilities (Hard Boundaries)

| Crate          | Owns                                    | Does NOT Own                    |
|----------------|-----------------------------------------|---------------------------------|
| `6g-common`    | Types, config, error types, validation  | Any physics or protocols        |
| `6g-phy`       | Channel model, waveform, RIS, MIMO      | Scheduling, core NFs            |
| `6g-isac`      | DFRC tradeoff, sensing, detection       | PHY channel model (import it)   |
| `6g-mac`       | Scheduler, HARQ, access control         | PHY waveform details            |
| `6g-ai`        | Model trait, inference dispatch         | Domain-specific logic           |
| `6g-ntn`       | Satellite/HAPS/UAV link models          | MAC scheduling                  |
| `6g-pdcp`      | Ciphering, header compression           | RLC or MAC concerns             |
| `6g-rlc`       | Segmentation, ARQ, reordering           | PDCP or MAC concerns            |
| `6g-rrc`       | Radio resource control, mobility        | RAN-layer details               |
| `6g-core`      | Session management, policy              | Core network functions          |
| `6g-semantic`  | Semantic encoding/decoding              | AI model internals              |

## Dependency Graph (No Upward Dependencies Allowed)

```
6g-common      ← no deps (foundation)
    ↑
6g-ai          ← uses 6g-common only
6g-ntn         ← uses 6g-common only
6g-pdcp        ← uses 6g-common only
6g-rlc         ← uses 6g-common only
    ↑
6g-phy         ← uses 6g-common + 6g-ai
6g-semantic    ← uses 6g-ai + 6g-common
    ↑
6g-isac        ← uses 6g-phy + 6g-common + 6g-ai
6g-mac         ← uses 6g-phy + 6g-common + 6g-ai
    ↑
6g-rrc         ← uses 6g-mac + 6g-pdcp + 6g-rlc + 6g-common
    ↑
6g-core        ← uses 6g-rrc + 6g-common + 6g-ai + 6g-ntn + 6g-semantic
```

A crate must **never** depend on a crate above it in this graph.
The CI script `scripts/check_dep_graph.py` enforces this automatically.

## Validation Contract

Every experiment/physics module MUST export a `Validate` implementation:

```rust
use sixg_common::validation::{Validate, ValidationResult, ValidationCheck};

impl Validate for MyModule {
    fn validate() -> ValidationResult {
        ValidationResult {
            module: "my_module",
            checks: vec![
                ValidationCheck::new("known_value_test", actual, expected, 0.01),
            ],
        }
    }
}
```

CI runs `cargo test --workspace` which exercises these validators.

## Pre-Finalisation Checklist (MANDATORY — run in this order before every commit)

Every agent **MUST** run the following four checks and resolve all failures
before pushing or finalising any code change.  A change is not done until
all four pass.

### 1. Formatting
```bash
cargo fmt
cargo fmt --check          # must exit 0 — zero diffs allowed
```

### 2. Dependency Graph
```bash
cargo tree --workspace --edges normal | python3 scripts/check_dep_graph.py
```
Must print `Dependency graph check passed`.  Any `VIOLATION:` line is a
blocker.  Cross-reference against the allowed graph in the
**Dependency Graph** section above before adding any `[dependencies]` entry.

### 3. Type Safety (AGENTS.md rules 1 & 2)
Grep the diff for bare `f64`, `u64`, or `usize` at `pub fn` API surfaces:
```bash
cargo clippy --workspace -- -D warnings   # must exit 0
```
Additionally, manually verify:
- No raw numeric type (`f64`, `u64`, `usize`) is used for a physical quantity
  (frequency, distance, power, time, bandwidth, velocity) at any `pub fn`
  boundary.  Use the newtypes in `crates/6g-common/src/types.rs` instead.
- No new numeric newtype was created when one already exists in `types.rs`.

### 4. Doc-Code and Test Coverage
```bash
cargo test --workspace     # must exit 0 — zero test failures
```
Additionally verify manually:
- Every new `pub fn` has a `///` doc comment stating the physical units of
  all arguments and the return value.
- Every new physics/protocol module has at least one `#[test]` that checks
  a known numerical result (formula + reference) and one `Validate` impl.

> **Rule**: If any check fails, fix the issue first — do not proceed to the
> next change.  Document which checks were run in the PR description.

## Context Compression: How to Navigate This Codebase Quickly

1. Read `AGENTS.md` (this file) — rules and boundaries
2. Read `docs/6g-common.md` — all shared types
3. Read `docs/<crate>.md` for the crate you're working on
4. Look at existing tests in that crate for numerical contracts
5. Check `crates/6g-common/src/validation.rs` for the `Validate` trait

**Do NOT** read all 11 crates before starting. Use the dependency graph above.

## Experiment Structure

New experiments go in `experiments/exp_NNN_<short_name>/`, **not** inside crates:

```
experiments/
  exp_001_ris_snr_gain/
    config.json          ← reproducible parameters
    run.rs               ← single experiment binary
    expected_output.json ← golden output for regression
    README.md            ← 5-line hypothesis + method + result
```

To add an experiment, add an `[[example]]` entry to the workspace `Cargo.toml`.
The experiment calls library functions — it does not add library code itself.

## Git Commit Convention

```
[crate][type]: short description

Types: feat | fix | refactor | exp | doc | test
Crate: phy | mac | isac | common | ai | ntn | pdcp | rlc | rrc | core | semantic

Examples:
  [isac][fix]: CRB formula was missing 8π² factor (Kay eq 3.31)
  [phy][exp]: exp_002 OTFS BER 2.3 dB better than OFDM at v=250 km/h
  [common][refactor]: Distance newtype replaces bare f64 for range params
```
