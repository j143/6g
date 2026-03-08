# References

Pinned reference documents for the 6G experiment bed.

## Standards & Framework Documents

- **ITU-R M.2160** — *Framework and overall objectives of the future development of IMT for 2030 and beyond* (IMT-2030 Framework, 2023). Free download from ITU-R. Defines the performance targets and use-case families that drive the project's validation strategy.

- **3GPP TR 38.901** — *Study on channel model for frequencies from 0.5 to 100 GHz*. Directly applicable for the PHY channel models; the 6G extension to THz builds on the CDL/TDL parametrisation defined here.

- **3GPP TR 22.837** — *Study on Integrated Sensing and Communication*. Defines ISAC use cases and sensing requirements, relevant to `6g-isac`.

## Industry White Papers

- **Samsung Research** — *The Next Hyper-Connected Experience for All: A White Paper on 6G* (2020). Comprehensive 6G use-case taxonomy; covers ISAC, XR, digital twins, and network intelligence.

- **Nokia Bell Labs** — *Communications in the 6G Era* (2020). Covers architectural shifts, THz spectrum, intelligent surfaces, and the role of AI/ML.

- **Qualcomm Technologies** — *Qualcomm 6G Foundry Paper Series* (2022–2023). Five papers available at qualcomm.com/research/6g:
  - *The 6G Vision*
  - *Rethinking the Control Plane*
  - *AI-Native 6G Air Interface*
  - *6G Spectrum*
  - *6G System Architecture*

- **Ericsson** — *6G — Connecting a Cyber-Physical World* (2022). Covers network architecture, sustainability targets, and connected intelligence.

## Academic Papers

### PHY / Waveforms

- **Hadani, R. et al.** — *Orthogonal Time Frequency Space Modulation* (IEEE WCNC 2017). Foundational OTFS paper; motivation for `6g-phy/waveform.rs` OTFS implementation.

- **Basar, E. et al.** — *Wireless Communications Through Reconfigurable Intelligent Surfaces* (IEEE Access, 2019). Core reference for `6g-phy/ris.rs` channel model `H_eff = H_d + H_r · Φ · H_i`.

- **Björnson, E. et al.** — *Massive MIMO Networks: Spectral, Energy, and Hardware Efficiency* (Foundations and Trends in Signal Processing, 2017). Baseline for the `6g-phy/mimo.rs` ELAA/massive-MIMO model.

### ISAC

- **Liu, F. et al.** — *Cramér–Rao Bound Optimization for Joint Radar-Communication Beamforming* (IEEE Trans. Signal Process., 2018; DOI: 10.1109/TSP.2018.2864261). Reference for the approximate scalar CRB used in `6g-isac/dfrc.rs`; the code uses a simplified SISO form (Kay, SPSS Vol. I, eq. 3.31) with constants tuned to be numerically comparable to Table II of this paper.

### AI / ML

- **O-RAN Alliance WG2** — *AI/ML Workflow Description and Requirements* (O-RAN.WG2.AIML-v01.00). Defines how AI/ML models are trained, deployed, and updated in the RAN; informs `6g-ai` and `6g-mac` scheduler design.

- **Simeone, O.** — *A Very Brief Introduction to Machine Learning With Applications to Communication Systems* (IEEE Trans. Cogn. Commun. Netw., 2018). Accessible entry point for the AI-native concepts in `6g-ai`.

### Semantic Communications

- **Qin, Z. et al.** — *Semantic Communications: Principles and Challenges* (IEEE JSAC, 2022). Theoretical foundation for `6g-semantic` encoder/decoder design.

- **Xie, H. et al.** — *Deep Learning Enabled Semantic Communication Systems* (IEEE Trans. Signal Process., 2021). DNN-based autoencoder approach implemented as the `SemanticCodec` trait.

## Open-Source Simulators and Public Datasets (Comparison Targets)

These are the primary external systems used to validate this testbed
(see `docs/comparison-strategy.md` for the full methodology).

- **srsRAN Project** — Open-source 5G NR gNB/UE in C++. Produces PDSCH BLER vs SNR JSON logs, MAC throughput traces, and HARQ retransmission stats.  URL: https://www.srsran.com

- **OpenAirInterface5G (OAI)** — Open-source 5G SA stack from EURECOM. Provides SINR traces, MAC scheduler throughput, and HARQ BLER.  URL: https://openairinterface.org

- **ns-3 5G-LENA NR module** — ns-3-based 5G NR system-level simulator from CTTC. Useful for end-to-end latency, Jain fairness index, and coverage metrics.  URL: https://5g-lena.cttc.es

- **Vienna 5G Link Level Simulator** — MATLAB-based PHY link-level simulator from TU Wien. Canonical BER/BLER vs Eb/N0 curves for OFDM and OTFS.  URL: https://www.nt.tuwien.ac.at/research/mobile-communications/vienna-5g-simulators/

- **NIST 5G mmWave Channel Model** — Publicly released path-loss tables at 28 GHz and 73 GHz for UMa/UMi scenarios; used to validate `6g-phy/spectrum.rs`.  URL: https://www.nist.gov/programs-projects/5g-channel-model

- **DeepMIMO** — Raytracing-based MIMO channel dataset. Provides CSI matrices and beamforming gain benchmarks for massive MIMO validation.  URL: https://deepmimo.net
