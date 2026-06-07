# NAM References

This directory is for local Neural Amp Modeler reference renders and metadata.

Do not commit downloaded NAM models, downloaded tone packs, or rendered WAVs
unless their redistribution license is explicit and compatible with the project.
Commit only source-safe metadata and experiment notes.

Preferred reference policy:

1. Use an **Amp Head** NAM capture when possible.
2. Render it with the same dry DI used for Greybound.
3. Pair it with the same Greybound cabinet IR when comparing full amp+cab output.
4. Compare that render against a Greybound render with IR enabled.

Fallback policy:

1. Use a **Full Rig / Combo** NAM capture only when no suitable amp-head capture
   is available.
2. Render Greybound with IR enabled for a broad full-chain comparison.
3. Treat all cab/mic differences as part of the reference mismatch.

Suggested first search target:

- Provider: TONE3000
- Category: VOX AC30
- Gear filter: Amp Head
- Platform: NAM
- Tone family: clean or edge-of-breakup AC30/Top Boost

If an exact AC30 amp-head capture is not available, the next closest candidate is
an AC15/Top Boost amp-head capture. That is less ideal, but still useful for
testing the NAM comparison workflow.
