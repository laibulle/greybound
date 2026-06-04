# AC30 Top Boost, Dumble Overdrive Special, and JCM800 schematic references

The folder contains three documented circuit references:

- `circuit-map.toml`: extracted topology map for the JMI AC30/6 OS/065 with OS/010 Top Boost.
- `dumble-overdrive-special.toml`: extracted topology map for the Dumble Overdrive Special.
- `jcm800-2203-2204.toml`: extracted topology map for the Marshall JCM800 2203/2204 Master Volume Lead.

The AC30 model targets a JMI-era AC30/6 fitted with the optional Top Boost unit.
The Dumble Overdrive Special model targets the boutique Dumble ODS head.
The JCM800 model targets the 2203/2204 Master Volume Lead family.

There was no single canonical AC30 circuit across all production years, so the
reference is explicitly split into these original drawings:

- `jmi-os065-ac30-6-normal.jpg`: JMI OS/065 AC30/6 chassis, including the
  long-tail-pair phase inverter, Cut control, cathode-biased EL84 quartet,
  output transformer, and GZ34 supply.
- `jmi-os010-top-boost.jpg`: JMI OS/010 add-on Top Boost circuit, including the
  bright-capped 500k volume control, two ECC83 stages, and interactive treble
  and bass network.
- `vox-ac30-reference.pdf` and `vox-top-boost-reference.pdf`: clearer service
  reference copies used to cross-check the original drawings.

The extracted component/topology map used by the DSP is in
`circuit-map.toml`. This remains a real-time graybox model, not a SPICE or
component-exact wave-digital simulation.

The OS/010 tone network itself is solved from its circuit topology with
trapezoidal Modified Nodal Analysis. The model includes the split 1M Treble
and Bass potentiometers, 50pF and two 22nF capacitors, the 100k and 10k
ground paths, low cathode-follower source impedance, and the downstream load.

## Sources

- https://www.voxac30.org.uk/vox_ac30_circuit_diagrams.html
- https://www.voxac30.org.uk/vox_ac30_top_boost_circuit.html
- https://el34world.com/charts/Schematics/Files/Vox/Vox_Schematics.htm
- https://www.drtube.com/marshall-jcm800/
- https://stompboxelectronics.com/2023/12/16/the-marshall-tone-stacks/
- https://www.marshall.com/id/en/product/jcm800-2203-vintage-reissue-head
