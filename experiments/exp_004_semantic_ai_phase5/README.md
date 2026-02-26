# Experiment 004 — Phase 5: Semantic & AI Layers

## Hypothesis

A semantic encoder can transmit the task-relevant content of a text message
at < 10% of the raw bandwidth while maintaining > 90% task success rate.
An AI-based (MLP) channel estimator achieves lower NMSE than both LS and
MMSE at SNR ≥ 0 dB.

## Method

**Channel estimation (Part 1)**
- Three estimators compared: LS (1/SNR), MMSE (1/(1+SNR)), MLP (learned residual)
- SNR swept from −5 dB to 20 dB
- Reference: Simeone, IEEE TCCN 2018; Dong et al., IEEE OJCOMS 2020

**Semantic communications (Part 2)**
- Three modes: raw, JPEG-style, semantic (TextSemanticCodec)
- Bandwidth reduction swept 1× – 30×
- Task success rate modelled for each mode
- Reference: Xie et al., IEEE Trans. Signal Process. 2021

## Result

- MLP estimator achieves lower NMSE than MMSE at all SNR ≥ 0 dB
- Semantic codec achieves > 90% task success at 10× bandwidth reduction
- JPEG achieves only ~26% task success at 10× compression
