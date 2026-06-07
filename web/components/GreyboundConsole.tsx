"use client";

import { useEffect, useMemo, useState } from "react";
import type { CSSProperties } from "react";
import { ampControls, defaultRuntimeConfig, rigPresets, type AmpControlId, type Pedal, type RuntimeConfig } from "../lib/rigs";
import { commandPreview, formatDbfs, simulateMonitor, type MonitorStats } from "../lib/simulation";

export function GreyboundConsole() {
  const [rigId, setRigId] = useState("nox30-driven");
  const [runtime, setRuntime] = useState<RuntimeConfig>(defaultRuntimeConfig);
  const [tick, setTick] = useState(0);
  const rig = useMemo(() => rigPresets.find((preset) => preset.id === rigId) ?? rigPresets[0], [rigId]);
  const [ampValues, setAmpValues] = useState(rig.amp);
  const liveRig = useMemo(() => ({ ...rig, amp: ampValues }), [rig, ampValues]);
  const stats = useMemo(() => simulateMonitor(liveRig, runtime, tick), [liveRig, runtime, tick]);

  useEffect(() => {
    setAmpValues(rig.amp);
  }, [rig]);

  useEffect(() => {
    const interval = window.setInterval(() => setTick((value) => value + 1), 250);
    return () => window.clearInterval(interval);
  }, []);

  const command = commandPreview(liveRig, runtime);

  return (
    <main className="shell">
      <header className="topbar">
        <div>
          <p className="eyebrow">Greybound standalone</p>
          <h1>Monitor web</h1>
        </div>
        <div className="engineState">
          <span className="stateDot" />
          <span>WASM pending</span>
        </div>
      </header>

      <section className="workspace">
        <aside className="sidebar" aria-label="Rig presets">
          <label className="fieldLabel" htmlFor="rig-select">Rig</label>
          <select id="rig-select" value={rigId} onChange={(event) => setRigId(event.target.value)}>
            {rigPresets.map((preset) => (
              <option key={preset.id} value={preset.id}>{preset.name}</option>
            ))}
          </select>

          <div className="runtimeGrid">
            <NumberField label="Sample rate" value={runtime.sampleRate} min={1} step={1000} onChange={(sampleRate) => setRuntime({ ...runtime, sampleRate })} />
            <NumberField label="Period" value={runtime.periodSize} min={1} step={16} onChange={(periodSize) => setRuntime({ ...runtime, periodSize })} />
            <NumberField label="Input ch" value={runtime.inputChannel} min={1} step={1} onChange={(inputChannel) => setRuntime({ ...runtime, inputChannel })} />
            <TextField label="Outputs" value={runtime.outputChannels} onChange={(outputChannels) => setRuntime({ ...runtime, outputChannels })} />
            <NumberField label="Input dB" value={runtime.inputDb} min={-60} max={24} step={1} onChange={(inputDb) => setRuntime({ ...runtime, inputDb })} />
            <NumberField label="Output dB" value={runtime.outputDb} min={-60} max={6} step={1} onChange={(outputDb) => setRuntime({ ...runtime, outputDb })} />
          </div>

          <div className="switches">
            <Switch label="Monitor" checked={runtime.monitor} onChange={(monitor) => setRuntime({ ...runtime, monitor })} />
            <Switch label="Speaker IR" checked={runtime.speakerIr} onChange={(speakerIr) => setRuntime({ ...runtime, speakerIr })} />
            <Switch label="Null output" checked={runtime.nullOutput} onChange={(nullOutput) => setRuntime({ ...runtime, nullOutput })} />
          </div>

          <TextField label="Device" value={runtime.device} onChange={(device) => setRuntime({ ...runtime, device })} />
          <TextField label="Input WAV" value={runtime.inputWav} onChange={(inputWav) => setRuntime({ ...runtime, inputWav })} />
          <TextField label="Output WAV" value={runtime.outputWav} onChange={(outputWav) => setRuntime({ ...runtime, outputWav })} />
        </aside>

        <section className="mainPanel">
          <MonitorHeader rigName={liveRig.name} file={liveRig.file} log={runtime.monitorLog} />
          <Pedalboard pedals={liveRig.pedals} ampBypassed={liveRig.ampBypassed} cabEnabled={runtime.speakerIr || liveRig.cabEnabled} />
          <Meters stats={stats} />
          <ComponentTelemetry stats={stats} />
          <div className="lowerGrid">
            <AmpControls values={ampValues} onChange={(id, value) => setAmpValues({ ...ampValues, [id]: value })} />
            <CommandPreview command={command} />
          </div>
        </section>
      </section>
    </main>
  );
}

function MonitorHeader({ rigName, file, log }: { rigName: string; file: string; log: string }) {
  return (
    <div className="monitorHeader">
      <div>
        <p className="eyebrow">model nox30</p>
        <h2>{rigName}</h2>
      </div>
      <dl>
        <div><dt>rig</dt><dd>{file}</dd></div>
        <div><dt>log</dt><dd>{log}</dd></div>
      </dl>
    </div>
  );
}

function Pedalboard({ pedals, ampBypassed, cabEnabled }: { pedals: Pedal[]; ampBypassed: boolean; cabEnabled: boolean }) {
  const sections = [
    { id: "pre", label: "GTR", out: "AMP", pedals: pedals.filter((pedal) => pedal.section === "pre") },
    { id: "fx", label: "SEND", out: "RETURN", pedals: pedals.filter((pedal) => pedal.section === "fx") },
    { id: "post", label: "AMP OUT", out: "OUT", pedals: pedals.filter((pedal) => pedal.section === "post") },
  ];

  return (
    <div className="pedalboard">
      {sections.map((section) => (
        <div key={section.id} className={section.pedals.length || section.id === "pre" ? "signalRow" : "signalRow empty"}>
          <span className="node">{section.label}</span>
          <span className="cable" />
          {section.pedals.map((pedal) => <PedalBox key={pedal.id} pedal={pedal} />)}
          {section.id === "pre" ? <AmpBox bypassed={ampBypassed} /> : null}
          {section.id === "pre" ? <CabBox enabled={cabEnabled} /> : null}
          <span className="cable" />
          <span className="node">{section.out}</span>
        </div>
      ))}
    </div>
  );
}

function PedalBox({ pedal }: { pedal: Pedal }) {
  return (
    <article className={pedal.bypassed ? "pedal bypassed" : "pedal"} style={{ "--pedal-color": pedal.color } as CSSProperties}>
      <div className="pedalLed" />
      <strong>{pedal.label}</strong>
      <span>{pedal.bypassed ? "bypass" : "active"}</span>
      <button type="button" aria-label={`${pedal.label} footswitch`} />
    </article>
  );
}

function AmpBox({ bypassed }: { bypassed: boolean }) {
  return (
    <article className={bypassed ? "ampBox bypassed" : "ampBox"}>
      <div className="pedalLed" />
      <strong>AMP Nox30</strong>
      <span>{bypassed ? "bypass" : "active"}</span>
      <button type="button" aria-label="Amp footswitch" />
    </article>
  );
}

function CabBox({ enabled }: { enabled: boolean }) {
  return (
    <article className={enabled ? "cabBox" : "cabBox bypassed"}>
      <div className="pedalLed" />
      <strong>CAB IR</strong>
      <span>{enabled ? "active" : "bypass"}</span>
      <button type="button" aria-label="Cab IR footswitch" />
    </article>
  );
}

function Meters({ stats }: { stats: MonitorStats }) {
  return (
    <div className="meters">
      <Meter label="input" rms={stats.inputRms} peak={stats.inputPeak} near={stats.inputNearClips} clips={stats.inputClips} />
      <Meter label="output" rms={stats.outputRms} peak={stats.outputPeak} near={stats.outputNearClips} clips={stats.outputClips} />
      <div className="xrun">
        <span>xrun in/out</span>
        <strong>{stats.inputOverruns}/{stats.outputUnderruns}</strong>
      </div>
    </div>
  );
}

function Meter({ label, rms, peak, near, clips }: { label: string; rms: number; peak: number; near: number; clips: number }) {
  return (
    <div className="meter">
      <div className="meterLabel">
        <span>{label}</span>
        <strong>rms {formatDbfs(rms)} dBFS</strong>
        <em>peak {formatDbfs(peak)} dBFS near/clip {near}/{clips}</em>
      </div>
      <div className="bar"><span style={{ width: `${Math.min(100, Math.max(0, ((20 * Math.log10(rms) + 60) / 60) * 100))}%` }} /></div>
    </div>
  );
}

function ComponentTelemetry({ stats }: { stats: MonitorStats }) {
  return (
    <section className="telemetry">
      <div className="telemetryLine">
        <span>rails avg/min</span>
        <strong>pre {stats.rails.preampAvg.toFixed(0)}/{stats.rails.preampMin.toFixed(0)} V</strong>
        <strong>pi {stats.rails.piAvg.toFixed(0)}/{stats.rails.piMin.toFixed(0)} V</strong>
        <strong>pwr {stats.rails.powerAvg.toFixed(0)}/{stats.rails.powerMin.toFixed(0)} V</strong>
        <strong>scr {stats.rails.screenAvg.toFixed(0)}/{stats.rails.screenMin.toFixed(0)} V</strong>
      </div>
      <div className="telemetryLine">
        <span>I avg/max mA</span>
        <strong>first {stats.currents.firstAvg.toFixed(2)}/{stats.currents.firstMax.toFixed(2)}</strong>
        <strong>pi {stats.currents.piAvg.toFixed(2)}/{stats.currents.piMax.toFixed(2)}</strong>
        <strong>pwr {stats.currents.powerAvg.toFixed(1)}/{stats.currents.powerMax.toFixed(1)}</strong>
        <strong>atk {stats.currents.attackAvg.toFixed(1)}/{stats.currents.attackMax.toFixed(1)}</strong>
      </div>
      <div className="probeStrip">
        {stats.probes.map((probe) => (
          <span key={probe.label}>{probe.label} {probe.avg.toFixed(3)}/{probe.max.toFixed(3)}</span>
        ))}
      </div>
    </section>
  );
}

function AmpControls({ values, onChange }: { values: Record<AmpControlId, number>; onChange: (id: AmpControlId, value: number) => void }) {
  return (
    <section className="controlsPanel">
      <div className="panelTitle">
        <h3>amp controls</h3>
        <span>0.0-10.0</span>
      </div>
      <div className="knobGrid">
        {ampControls.map((control) => (
          <label key={control.id} className="knob">
            <span>{control.label}</span>
            <input
              type="range"
              min="0"
              max="1"
              step="0.01"
              value={values[control.id]}
              onChange={(event) => onChange(control.id, Number(event.target.value))}
            />
            <strong>{(values[control.id] * 10).toFixed(1)}</strong>
          </label>
        ))}
      </div>
    </section>
  );
}

function CommandPreview({ command }: { command: string }) {
  return (
    <section className="commandPanel">
      <div className="panelTitle">
        <h3>CLI preview</h3>
        <span>mock runtime</span>
      </div>
      <code>{command}</code>
    </section>
  );
}

function NumberField({ label, value, min, max, step, onChange }: { label: string; value: number; min?: number; max?: number; step?: number; onChange: (value: number) => void }) {
  return (
    <label className="field">
      <span>{label}</span>
      <input type="number" value={value} min={min} max={max} step={step} onChange={(event) => onChange(Number(event.target.value))} />
    </label>
  );
}

function TextField({ label, value, onChange }: { label: string; value: string; onChange: (value: string) => void }) {
  return (
    <label className="field">
      <span>{label}</span>
      <input type="text" value={value} onChange={(event) => onChange(event.target.value)} />
    </label>
  );
}

function Switch({ label, checked, onChange }: { label: string; checked: boolean; onChange: (checked: boolean) => void }) {
  return (
    <label className="switch">
      <input type="checkbox" checked={checked} onChange={(event) => onChange(event.target.checked)} />
      <span>{label}</span>
    </label>
  );
}
