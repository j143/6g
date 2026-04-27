# ── Stage 1: builder ──────────────────────────────────────────────────────────
# Uses the official Rust slim image so cargo + rustc are pre-installed.
FROM rust:1.82-slim AS builder

WORKDIR /build

# Cache dependency downloads before copying source.
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY src/ src/
COPY experiments/ experiments/

# Build the sixg-bench binary and all example experiment binaries.
RUN cargo build --release \
    && cargo build --release --examples

# ── Stage 2: runtime ──────────────────────────────────────────────────────────
# Minimal Debian image — no Rust toolchain, only the compiled artefacts.
FROM debian:bookworm-slim AS runtime

LABEL org.opencontainers.image.title="sixg-bench" \
      org.opencontainers.image.description="Standalone 6G simulation bench" \
      org.opencontainers.image.source="https://github.com/j143/6g" \
      org.opencontainers.image.licenses="Apache-2.0"

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /bench

# Copy the bench binary.
COPY --from=builder /build/target/release/sixg-bench /usr/local/bin/sixg-bench

# Copy all pre-built experiment binaries.
COPY --from=builder /build/target/release/examples/ /bench/target/release/examples/

# Copy experiment config files (read by the bench at runtime).
COPY experiments/ /bench/experiments/

# Copy bundled baseline CSV files.
COPY baselines/ /bench/baselines/

ENV SIXG_BASELINES=/bench/baselines

ENTRYPOINT ["sixg-bench"]
CMD ["--help"]
