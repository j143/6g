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

- **Liu, F. et al.** — *Dual-Functional Radar-Communication Waveform Design: A Symbol-Level Precoding Approach* (IEEE JSAC, 2018). Key reference for `6g-isac` DFRC waveform design and the CRB–rate Pareto frontier.

### AI / ML

- **O-RAN Alliance WG2** — *AI/ML Workflow Description and Requirements* (O-RAN.WG2.AIML-v01.00). Defines how AI/ML models are trained, deployed, and updated in the RAN; informs `6g-ai` and `6g-mac` scheduler design.

- **Simeone, O.** — *A Very Brief Introduction to Machine Learning With Applications to Communication Systems* (IEEE Trans. Cogn. Commun. Netw., 2018). Accessible entry point for the AI-native concepts in `6g-ai`.

### Semantic Communications

- **Qin, Z. et al.** — *Semantic Communications: Principles and Challenges* (IEEE JSAC, 2022). Theoretical foundation for `6g-semantic` encoder/decoder design.

- **Xie, H. et al.** — *Deep Learning Enabled Semantic Communication Systems* (IEEE Trans. Signal Process., 2021). DNN-based autoencoder approach implemented as the `SemanticCodec` trait.
