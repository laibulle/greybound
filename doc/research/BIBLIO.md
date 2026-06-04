Project Reference Bibliography: Virtual Analog & WDF Modeling
This document provides an overview of the foundational literature and state-of-the-art papers utilized in the development of this real-time DSP modeling engine.

1. The Core Architecture Blueprint
File: comj.2009.33.2.85.pdf

Citation: Pakarinen, J., & Yeh, D. T. (2009). A Review of Digital Techniques for Modeling Guitar Amplifiers. Computer Music Journal, 33(2), 85-98.

What it refers to: This is the definitive foundational review for Gray-Box modeling. It outlines how to break down complex tube amplifier topologies into discrete, cascaded stages—separating linear filtering (like tone stacks) from static non-linearities (like triode saturation). It provides the exact structural logic used to organize our pipeline.

2. The Wave Digital Filter Bible
File: Wave Digital Filters Theory and Practice.pdf

Citation: Fettweis, A. (1986). Wave Digital Filters: Theory and Practice. Proceedings of the IEEE, 74(2), 270-327.

What it refers to: The absolute "Genesis" of Wave Digital Filters (WDF). Fettweis explains how to map classical analog circuit variables (voltages and currents) onto wave variables (incident and reflected waves). This methodology allows the creation of digital structures that naturally mimic physical components while maintaining absolute numerical passivity and stability.

3. Solving Complex Topologies & Feedback
File: DAFx-15_submission_53.pdf

Citation: Werner, K. J., Smith, W. R., & Abel, J. S. (2015). Wave Digital Filter Adaptors for Arbitrary Topologies and Multiport Linear Elements. In Proceedings of the 18th International Conference on Digital Audio Effects (DAFx-15).

What it refers to: Historically, WDFs struggled heavily with circuits that couldn't be neatly arranged into simple series or parallel branches. Werner et al. solved this by deriving custom adaptors using Modified Nodal Analysis (MNA). This paper is what allows modern WDF libraries to compute complex, interconnected sub-circuits (like multi-port transformers or interactive tone stacks) without getting stuck in non-computable delay-free loops.

4. The Deep Learning Black-Box Baseline
File: 1804.07145v1.pdf

Citation: Schmitz, T., & Embrechts, J. J. (2018). Real Time Emulation of Parametric Guitar Tube Amplifier With Long Short Term Memory Neural Network. arXiv preprint arXiv:1804.07145.

What it refers to: This paper represents the pure Black-Box AI modeling methodology. It investigates using Recurrent Neural Networks (specifically LSTMs) to predict the output of a tube amplifier stage directly from raw data, including adapting to knob parameter changes (like gain). It serves as a benchmark for pure data-driven approaches vs. our physical structural modeling.

5. Multiple Nonlinearities in Neural WDFs
File: DAFx24_paper_45.pdf

Citation: Massi, O., Manino, E., & Bernardini, A. (2024). Wave Digital Modeling of Circuits with Multiple One-Port Nonlinearities Based on Lipschitz-Bounded Neural Networks. In Proceedings of the 27th International Conference on Digital Audio Effects (DAFx-24).

What it refers to: This paper models nonlinear one-port scattering relations with
Lipschitz-bounded neural networks inside a Wave Digital circuit containing
multiple nonlinearities. The Lipschitz constraint preserves sufficient
fixed-point convergence conditions for the Scattering Iterative Method (SIM).
Its validation circuit is a four-diode ring modulator, not a tube amplifier.
The neural blocks do not eliminate iteration: the reported optimized SIM still
uses an average of seven iterations per sample, compared with 37 using fixed
port resistances.

Relevance to VoxBox: This becomes directly applicable only if a future
component-level WDF implementation couples several nonlinear one-port devices
through a shared scattering junction. The current VoxBox model instead
cascades independent behavioral tube nonlinearities with linear filters, so it
has no nonlinear delay-free loop for SIM to solve. Tube stages and output
transformers are also generally multi-port and may require vector WDF or other
models beyond the one-port method studied here.
