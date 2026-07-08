import type { CSSProperties } from "react";

const DOWNLOAD_URL =
  "https://github.com/laibulle/greybound/releases/download/0.0.1-alpha1/Greybound.Free.dmg";

function GitHubMark({ size }: { size: number }) {
  return (
    <svg
      aria-hidden="true"
      className="githubMark"
      fill="currentColor"
      height={size}
      viewBox="0 0 24 24"
      width={size}
    >
      <path d="M12 .5C5.65.5.5 5.65.5 12c0 5.08 3.29 9.39 7.86 10.91.58.1.79-.25.79-.56v-2.16c-3.2.7-3.87-1.36-3.87-1.36-.52-1.33-1.28-1.68-1.28-1.68-1.04-.71.08-.7.08-.7 1.15.08 1.76 1.18 1.76 1.18 1.03 1.75 2.69 1.24 3.35.95.1-.74.4-1.24.73-1.53-2.55-.29-5.23-1.28-5.23-5.69 0-1.26.45-2.28 1.18-3.09-.12-.29-.51-1.46.11-3.05 0 0 .96-.31 3.16 1.18A10.94 10.94 0 0 1 12 6.02c.98 0 1.97.13 2.89.38 2.19-1.49 3.15-1.18 3.15-1.18.63 1.59.24 2.76.12 3.05.74.81 1.18 1.83 1.18 3.09 0 4.42-2.69 5.39-5.25 5.68.41.36.78 1.06.78 2.14v3.17c0 .31.21.67.8.56A11.51 11.51 0 0 0 23.5 12C23.5 5.65 18.35.5 12 .5Z" />
    </svg>
  );
}

function DownloadIcon({ size }: { size: number }) {
  return (
    <svg
      aria-hidden="true"
      className="downloadIcon"
      fill="none"
      height={size}
      viewBox="0 0 24 24"
      width={size}
    >
      <path
        d="M12 3v11m0 0 4-4m-4 4-4-4M5 17v2.2c0 .5.4.8.8.8h12.4c.4 0 .8-.3.8-.8V17"
        stroke="currentColor"
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth="2"
      />
    </svg>
  );
}

function ModelCircuit({ kind }: { kind: "white" | "black" | "grey" }) {
  if (kind === "white") {
    return (
      <div className="boxGlyph circuitGlyph whiteCircuit" aria-hidden="true">
        <span className="jack inputJack" />
        <span className="wire wireA" />
        <span className="component resistor" />
        <span className="node nodeA" />
        <span className="component capacitor" />
        <span className="node nodeB" />
        <span className="component linearCore">
          <span>Ax=b</span>
        </span>
        <span className="wire wireB" />
        <span className="jack outputJack" />
        <span className="equationLabel labelA">R</span>
        <span className="equationLabel labelB">C</span>
        <span className="equationLabel labelC">linear</span>
      </div>
    );
  }

  if (kind === "black") {
    return (
      <div className="boxGlyph circuitGlyph blackCircuit" aria-hidden="true">
        <span className="jack inputJack" />
        <span className="wire wireA" />
        <span className="sealedBox neuralBox">
          <span className="neuron n1" />
          <span className="neuron n2" />
          <span className="neuron n3" />
          <span className="neuron n4" />
          <span className="neuron n5" />
          <span className="neuron n6" />
          <span className="neuralLink l1" />
          <span className="neuralLink l2" />
          <span className="neuralLink l3" />
          <span className="neuralLabel">neural net</span>
        </span>
        <span className="wire wireB" />
        <span className="jack outputJack" />
        <span className="waveTrace traceIn" />
        <span className="waveTrace traceOut" />
      </div>
    );
  }

  return (
    <div className="boxGlyph circuitGlyph greyCircuit" aria-hidden="true">
      <span className="jack inputJack" />
      <span className="wire wireA" />
      <span className="component resistor" />
      <span className="probe probeA" />
      <span className="component toneStack" />
      <span className="microNet microNetA">µNN</span>
      <span className="microNet microNetB">µNN</span>
      <span className="sharedState">shared voltage state</span>
      <span className="probe probeB" />
      <span className="wire wireB" />
      <span className="jack outputJack" />
      <span className="measureTrace" />
    </div>
  );
}

const stats = [
  { value: "Open source", label: "experiment-first project" },
  { value: "Greybox", label: "circuit ideas plus measured behavior" },
  { value: "Rust", label: "audio model playground" },
  { value: "Notes", label: "transparent research trail" },
];

const chain = [
  "Guitar input",
  "Circuit clue",
  "Greybox model",
  "Reference render",
  "Listening notes",
];

const methodSteps = [
  {
    label: "1",
    title: "Keep the graph",
    text: "The amplifier remains a network of stages, rails, loads, coupling paths, and observable operating points.",
  },
  {
    label: "2",
    title: "Learn locally",
    text: "Tiny cells are allowed only where nonlinear behavior is too subtle for a clean equation.",
  },
  {
    label: "3",
    title: "Return state",
    text: "The learned component reports voltage, current, bias, and headroom back into the shared circuit state.",
  },
];

const pillars = [
  {
    eyebrow: "Greybox core",
    title: "Not a black box, not a full circuit clone.",
    text: "Greybound keeps the circuit graph visible, then lets local nonlinear cells improve the hard parts without swallowing the whole sound into one opaque model.",
  },
  {
    eyebrow: "Research lab",
    title: "Experiments should leave a trail.",
    text: "Model notes, diagrams, reference renders, and evaluation scripts live close to the code so every tone decision can be questioned, repeated, or improved.",
  },
  {
    eyebrow: "Sound design",
    title: "Built for guitar tone as a living system.",
    text: "The project starts with amp and pedal behavior, rig files, and listening tests, then grows only when the model earns its place in the chain.",
  },
];

const modelTypes = [
  {
    name: "White box",
    short: "Equations first",
    description:
      "A model built from explicit equations. In practice it often captures the linear part of components well, while the messy nonlinear feel still needs careful choices.",
    pros: ["Readable assumptions", "Stable linear behavior", "Good for design questions"],
    cons: ["Can miss organic feel", "Can sound too ideal", "Nonlinear parts are hard"],
    className: "whiteBox",
    diagram: "white",
  },
  {
    name: "Black box",
    short: "Examples first",
    description:
      "A neural network trained from input and output examples. It can imitate what it has heard, but its internal reasoning is not a circuit you can inspect.",
    pros: ["Can capture complex tone", "Fast once trained", "Works without schematics"],
    cons: ["Replays existing sounds", "Hard to inspect", "Needs strong datasets"],
    className: "blackBox",
    diagram: "black",
  },
  {
    name: "Grey box",
    short: "Circuit state plus micro-nets",
    description:
      "A circuit-level model where nonlinear components can use tiny neural networks, while voltage drops are reported back into the global circuit as shared state.",
    pros: ["Keeps circuit context", "Learns nonlinear feel locally", "Auditable signal flow"],
    cons: ["Requires careful coupling", "Needs component validation", "More design judgment"],
    className: "greyBox",
    diagram: "grey",
  },
] as const;

const roadmap = [
  "Top Boost-style greybox amp experiment",
  "Pedal model sketches for gain, time, filter, and dynamics blocks",
  "Circuit diagrams and model notes",
  "Offline render and reference comparison tooling",
  "Rust-first codebase for repeatable audio experiments",
];

const subtletyPoints = [
  {
    title: "The circuit stays global.",
    text: "Stages do not secretly mutate each other. They exchange explicit boundary data: voltage, impedance, DC offset, headroom, coupling style, level history, and latency.",
  },
  {
    title: "Learning stays local.",
    text: "Micro neural cells are used for nonlinear components, not for memorizing an entire amplifier. A learned cell has a small job and a visible electrical contract.",
  },
  {
    title: "Voltage drops become shared state.",
    text: "Current draw updates rails, bias, and recovery behavior. Those voltage changes are reported back into the circuit so the next stage feels the same stress.",
  },
  {
    title: "Promotion needs evidence.",
    text: "A cell can shadow the analytic path before replacing it. SPICE fixtures, held-out stimuli, render metrics, and listening notes decide whether it earns its place.",
  },
];

const productScreens = [
  {
    title: "Pedal chain",
    text: "Gain stages, bypass state, and level choices stay visible while the model runs.",
    src: "/product/desktop-minotaur.webp",
    alt: "Greybound desktop Minotaur pedal view",
  },
  {
    title: "FX loop",
    text: "Time and ambience blocks live in the same inspectable signal path.",
    src: "/product/desktop-springfield.webp",
    alt: "Greybound desktop Springfield reverb view",
  },
  {
    title: "Tone shaping",
    text: "Output EQ and filtering are part of the repeatable demo workflow.",
    src: "/product/desktop-eq.webp",
    alt: "Greybound desktop equalizer view",
  },
];

export default function Home() {
  return (
    <main>
      <section className="hero" id="top" aria-labelledby="hero-title">
        <div className="heroScene" aria-hidden="true">
          <div className="ampFace">
            <div className="ampHandle" />
            <div className="ampGrille">
              {Array.from({ length: 12 }).map((_, index) => (
                <span key={index} />
              ))}
            </div>
            <div className="ampControls">
              {Array.from({ length: 7 }).map((_, index) => (
                <span key={index} />
              ))}
            </div>
          </div>
          <div className="waveform">
            {Array.from({ length: 28 }).map((_, index) => (
              <span key={index} style={{ "--i": index } as CSSProperties} />
            ))}
          </div>
        </div>

        <header className="siteHeader">
          <a className="brand" href="#top" aria-label="Greybound home">
            <img src="/greybound-robine-mark.svg" alt="" />
            <span>Greybound</span>
          </a>
          <nav aria-label="Primary navigation">
            <a href="#desktop-alpha">App</a>
            <a href="#engine">Engine</a>
            <a href="#method">Method</a>
            <a href="#rigs">Rigs</a>
            <a href="#roadmap">Roadmap</a>
            <a
              className="navDownload"
              href={DOWNLOAD_URL}
              rel="noopener noreferrer"
              target="_blank"
            >
              <DownloadIcon size={16} />
              <span>Download</span>
            </a>
            <a
              className="externalLink"
              href="https://github.com/laibulle/greybound"
              rel="noopener noreferrer"
              target="_blank"
            >
              <GitHubMark size={16} />
              <span>Repository</span>
            </a>
          </nav>
        </header>

        <div className="heroContent">
          <p className="kicker">Open-source greybox guitar tone experiment.</p>
          <h1 id="hero-title">Greybound</h1>
          <p className="heroText">
            A public research base for guitar amp and pedal modeling, built
            around greybox thinking: analog circuits as a map, measurements as
            a check, and local learned laws that still report back into the
            circuit.
          </p>
          <div className="heroActions" aria-label="Primary actions">
            <a
              className="button primary downloadButton"
              href={DOWNLOAD_URL}
              rel="noopener noreferrer"
              target="_blank"
            >
              <DownloadIcon size={20} />
              <span>
                Download for macOS
                <small>0.0.1 alpha DMG</small>
              </span>
            </a>
            <a className="button secondary" href="#engine">
              Explore the experiment
            </a>
            <a
              className="button secondary"
              href="https://github.com/laibulle/greybound"
              rel="noopener noreferrer"
              target="_blank"
            >
              <GitHubMark size={18} />
              Open the repository
            </a>
          </div>
          <p className="heroRelease">
            Alpha build distributed outside the App Store. macOS may ask for a
            manual open confirmation.
          </p>
        </div>
      </section>

      <section className="statsBand" aria-label="Project highlights">
        {stats.map((item) => (
          <div key={item.label}>
            <strong>{item.value}</strong>
            <span>{item.label}</span>
          </div>
        ))}
      </section>

      <section className="desktopShowcase" id="desktop-alpha" aria-labelledby="desktop-title">
        <div className="desktopCopy">
          <p className="sectionKicker">Desktop alpha</p>
          <h2 id="desktop-title">A real app for testing the greybox chain.</h2>
          <p>
            Load the macOS alpha, route live guitar or a WAV file through the
            current rig, and record repeatable demo passes while changing amp,
            pedal, cab, doubler, and EQ controls.
          </p>
          <div className="desktopActions">
            <a
              className="button primary downloadButton"
              href={DOWNLOAD_URL}
              rel="noopener noreferrer"
              target="_blank"
            >
              <DownloadIcon size={20} />
              <span>
                Download for macOS
                <small>Apple Silicon alpha</small>
              </span>
            </a>
            <a
              className="button secondary"
              href="https://github.com/laibulle/greybound/releases/tag/0.0.1-alpha1"
              rel="noopener noreferrer"
              target="_blank"
            >
              Release notes
            </a>
          </div>
        </div>
        <div className="desktopVisuals" aria-label="Greybound desktop screenshots">
          <figure className="desktopMainShot">
            <img
              src="/product/desktop-amp.webp"
              alt="Greybound desktop amp view showing the Nox30 amp controls"
            />
            <figcaption>Nox30 amp view</figcaption>
          </figure>
          <div className="desktopShotGrid">
            {productScreens.map((screen) => (
              <figure key={screen.title} className="desktopShot">
                <img src={screen.src} alt={screen.alt} />
                <figcaption>
                  <strong>{screen.title}</strong>
                  <span>{screen.text}</span>
                </figcaption>
              </figure>
            ))}
          </div>
        </div>
      </section>

      <section className="section intro" id="engine">
        <div>
          <p className="sectionKicker">Why it exists</p>
          <h2>Greybound is a shared workbench for greybox tone.</h2>
        </div>
        <p>
          The project starts from a simple idea: guitar tone models should be
          understandable enough to change, measured enough to trust, and open
          enough for other builders to test their own instincts against the
          code.
        </p>
      </section>

      <section className="thesisStrip" id="method" aria-labelledby="method-title">
        <div className="thesisIntro">
          <p className="sectionKicker">Method</p>
          <h2 id="method-title">Not a captured amp. Not a frozen schematic.</h2>
          <p>
            Greybound treats tone as a living circuit state. The graph stays
            readable, the nonlinearity is learned locally, and every local
            choice must return useful electrical state to the rest of the model.
          </p>
        </div>
        <div className="methodFlow" aria-label="Greybound modeling method">
          {methodSteps.map((step) => (
            <article key={step.label}>
              <strong>{step.label}</strong>
              <h3>{step.title}</h3>
              <p>{step.text}</p>
            </article>
          ))}
        </div>
      </section>

      <section className="boxModels" aria-labelledby="box-models-title">
        <div className="boxModelsHeader">
          <p className="sectionKicker">Modeling map</p>
          <h2 id="box-models-title">White box, black box, grey box.</h2>
          <p>
            Greybound lives in the middle: more inspectable than pure machine
            learning, more organic than a purely linearized circuit solve, and
            more practical than a component-perfect simulator.
          </p>
        </div>
        <div className="boxModelGrid">
          {modelTypes.map((model) => (
            <article className={`boxModel ${model.className}`} key={model.name}>
              <ModelCircuit kind={model.diagram} />
              <p>{model.short}</p>
              <h3>{model.name}</h3>
              <span className="boxDescription">{model.description}</span>
              <div className="prosCons">
                <div>
                  <strong>Pros</strong>
                  <ul>
                    {model.pros.map((item) => (
                      <li key={item}>{item}</li>
                    ))}
                  </ul>
                </div>
                <div>
                  <strong>Cons</strong>
                  <ul>
                    {model.cons.map((item) => (
                      <li key={item}>{item}</li>
                    ))}
                  </ul>
                </div>
              </div>
            </article>
          ))}
        </div>
      </section>

      <section className="modelDepth" aria-labelledby="model-depth-title">
        <div className="modelDepthVisual" aria-hidden="true">
          <span className="depthRail" />
          <span className="depthNode nodeInput">in</span>
          <span className="depthNode nodePre">V1</span>
          <span className="depthNode nodeTone">Z</span>
          <span className="depthNode nodePower">B+</span>
          <span className="depthCell cellA">µNN</span>
          <span className="depthCell cellB">µNN</span>
          <span className="depthState">voltage / impedance / headroom</span>
          <span className="depthFeedback feedbackA" />
          <span className="depthFeedback feedbackB" />
        </div>
        <div className="modelDepthCopy">
          <p className="sectionKicker">The subtle part</p>
          <h2 id="model-depth-title">The model is not a capture. It is a circuit with learned local laws.</h2>
          <p>
            Many amp models choose one extreme: solve simplified equations and
            lose the messy feel, or train one large network that can only echo
            the sounds in its dataset. Greybound keeps the circuit as the
            organizing structure, then learns the small nonlinear behaviors
            that deserve it.
          </p>
          <div className="subtletyGrid">
            {subtletyPoints.map((point) => (
              <article key={point.title}>
                <h3>{point.title}</h3>
                <p>{point.text}</p>
              </article>
            ))}
          </div>
        </div>
      </section>

      <section className="section signal" aria-labelledby="signal-title">
        <div className="signalHeader">
          <p className="sectionKicker">Signal chain</p>
          <h2 id="signal-title">From string to speaker, every stage has a job.</h2>
        </div>
        <ol>
          {chain.map((item) => (
            <li key={item}>
              <span />
              {item}
            </li>
          ))}
        </ol>
      </section>

      <section className="section pillars" id="rigs" aria-label="Greybound pillars">
        {pillars.map((pillar) => (
          <article key={pillar.title}>
            <p>{pillar.eyebrow}</p>
            <h3>{pillar.title}</h3>
            <span>{pillar.text}</span>
          </article>
        ))}
      </section>

      <section className="section lab" aria-labelledby="lab-title">
        <div className="labVisual" aria-hidden="true">
          <div className="meter meterA" />
          <div className="meter meterB" />
          <div className="meter meterC" />
          <div className="scopeTrace" />
        </div>
        <div>
          <p className="sectionKicker">Research spine</p>
          <h2 id="lab-title">A tone project with receipts.</h2>
          <p>
            Greybound keeps topology notes, model diagrams, calibration runs,
            render scripts, and comparison traces close to the code. The result
            is an experiment that can be tuned by ear and challenged by
            measurement.
          </p>
        </div>
      </section>

      <section className="section roadmap" id="roadmap">
        <div>
          <p className="sectionKicker">What is inside</p>
          <h2>Built as an experiment others can open up.</h2>
        </div>
        <ul>
          {roadmap.map((item) => (
            <li key={item}>{item}</li>
          ))}
        </ul>
      </section>

      <footer>
        <img src="/greybound-robine-mark.svg" alt="" />
        <div>
          <strong>Greybound</strong>
          <span>Made for loud questions, quiet measurement, and responsive guitar tone.</span>
        </div>
      </footer>
    </main>
  );
}
