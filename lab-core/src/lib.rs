//! Shared, headless experiment layer for Greybound Lab.
//!
//! This crate deliberately knows nothing about MCP or iced. Both AI clients and
//! human-facing applications create the same bounded render request and receive
//! the same versioned artifact record.

use anyhow::{bail, Context, Result};
use rustfft::num_complex::Complex32;
use rustfft::FftPlanner;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

pub const ARTIFACT_SCHEMA: &str = "greybound.lab.run.v1";
pub const HOST_ADAPTER_SCHEMA: &str = "greybound.lab.host-adapter.v1";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RenderRuntime {
    pub sample_rate: u64,
    pub period_size: u64,
    pub render_seconds: f64,
    pub input_wav: String,
    pub input_channel: u64,
    pub ir: String,
    pub output_wav: String,
    pub monitor: bool,
}

#[derive(Clone, Debug)]
pub struct RenderRequest {
    pub run_id: String,
    pub cli: PathBuf,
    pub rig: String,
    pub effective_rig: PathBuf,
    pub runtime: RenderRuntime,
    pub monitor_log: PathBuf,
    pub metadata_path: PathBuf,
    pub git_revision: Option<String>,
    pub context: Value,
}

/// Analysis record for audio rendered by another plugin host or hardware
/// workflow. It shares Lab history without granting arbitrary process execution.
#[derive(Clone, Debug)]
pub struct WavAnalysisRequest {
    pub run_id: String,
    pub candidate_wav: PathBuf,
    pub metadata_path: PathBuf,
    pub label: String,
    pub context: Value,
}

/// A portable, declarative bridge to a third-party plugin host. The Lab does
/// not load AU/VST3/CLAP code inside its own process; instead it invokes a
/// user-installed renderer with explicit arguments and records the result.
/// This keeps crashes and vendor SDK constraints outside the Lab process while
/// still giving every format the same reproducible artifact contract.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HostAdapterManifest {
    pub schema: String,
    pub id: String,
    pub name: String,
    pub plugin_format: String,
    pub executable: String,
    pub arguments: Vec<String>,
    #[serde(default)]
    pub environment: BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
pub struct PluginRenderRequest {
    pub run_id: String,
    pub adapter_path: PathBuf,
    pub plugin_path: PathBuf,
    pub input_wav: PathBuf,
    pub output_wav: PathBuf,
    pub metadata_path: PathBuf,
    pub sample_rate: u64,
    pub block_size: u64,
    pub render_seconds: f64,
    pub context: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunArtifacts {
    pub candidate_wav: String,
    pub monitor_log: String,
    pub metadata: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunArtifact {
    pub schema: String,
    pub run_id: String,
    pub status: String,
    pub created_at_unix_ms: u128,
    pub command: Vec<String>,
    pub exit_status: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub git_revision: Option<String>,
    pub rig: String,
    pub effective_rig: String,
    pub runtime: RenderRuntime,
    pub artifacts: RunArtifacts,
    pub diagnostics: RunDiagnostics,
    pub context: Value,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RunDiagnostics {
    pub status: String,
    pub warnings: Vec<String>,
    pub monitor_entries: usize,
    pub input_rms_dbfs: Option<f64>,
    pub input_peak_dbfs: Option<f64>,
    pub output_rms_dbfs: Option<f64>,
    pub output_peak_dbfs: Option<f64>,
    pub xrun_count: u64,
    pub hard_clip_count: u64,
    pub near_clip_count: u64,
    pub wav_sample_rate: Option<u32>,
    pub wav_channels: Option<u16>,
    pub wav_frames: Option<u64>,
    pub wav_duration_seconds: Option<f64>,
    pub wav_rms: Option<f64>,
    pub wav_peak: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WaveformPreview {
    pub sample_rate: u32,
    pub channels: u16,
    pub duration_seconds: f64,
    pub min: Vec<f32>,
    pub max: Vec<f32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpectrumPreview {
    pub sample_rate: u32,
    pub frequencies_hz: Vec<f32>,
    pub magnitudes_db: Vec<f32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WavComparison {
    pub candidate: String,
    pub reference: String,
    pub status: String,
    pub warnings: Vec<String>,
    pub sample_rate: Option<u32>,
    pub aligned_frames: u64,
    pub latency_samples: i64,
    pub gain_correction_db: Option<f64>,
    pub candidate_rms: Option<f64>,
    pub reference_rms: Option<f64>,
    pub residual_rms: Option<f64>,
    pub null_relative_db: Option<f64>,
}

#[derive(Clone, Debug)]
pub struct ComparisonReportRequest {
    pub candidate: PathBuf,
    pub reference: PathBuf,
    pub metadata: Option<PathBuf>,
    pub segments: Option<PathBuf>,
    pub report: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComparisonReport {
    pub status: String,
    pub report: String,
    pub command: Vec<String>,
    pub stdout: String,
    pub stderr: String,
    pub exit_status: Option<i32>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WorkspaceSnapshot {
    pub runs: Vec<RunArtifact>,
}

#[derive(Clone, Debug)]
pub struct LabCore {
    root: PathBuf,
}

impl LabCore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn run_render(&self, request: RenderRequest) -> Result<RunArtifact> {
        let output_wav = self.resolve(&request.runtime.output_wav);
        let input_wav = self.resolve(&request.runtime.input_wav);
        let ir = self.resolve(&request.runtime.ir);
        for path in [&output_wav, &request.monitor_log, &request.metadata_path] {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
        }

        let mut command = Command::new(&request.cli);
        command
            .current_dir(&self.root)
            .arg("--rig")
            .arg(&request.effective_rig)
            .arg("--input-wav")
            .arg(&input_wav)
            .arg("--input-channel")
            .arg((request.runtime.input_channel + 1).to_string())
            .arg("--output-wav")
            .arg(&output_wav)
            .arg("--render-seconds")
            .arg(request.runtime.render_seconds.to_string())
            .arg("--sample-rate")
            .arg(request.runtime.sample_rate.to_string())
            .arg("--period-size")
            .arg(request.runtime.period_size.to_string())
            .arg("--ir")
            .arg(&ir)
            .arg("--monitor-log")
            .arg(&request.monitor_log);
        if request.runtime.monitor {
            command.arg("--monitor");
        }

        let command_line = command_to_vec(&request.cli, &command);
        let output = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .with_context(|| format!("failed to launch {}", request.cli.display()))?;
        let diagnostics = self.analyse_render(&request.monitor_log, &output_wav)?;
        let status = if output.status.success() {
            diagnostics.status.clone()
        } else {
            "severe".to_string()
        };
        let artifact = RunArtifact {
            schema: ARTIFACT_SCHEMA.to_string(),
            run_id: request.run_id,
            status,
            created_at_unix_ms: now_unix_ms(),
            command: command_line,
            exit_status: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            git_revision: request.git_revision,
            rig: request.rig,
            effective_rig: path_for_artifact(&self.root, &request.effective_rig),
            runtime: request.runtime,
            artifacts: RunArtifacts {
                candidate_wav: path_for_artifact(&self.root, &output_wav),
                monitor_log: path_for_artifact(&self.root, &request.monitor_log),
                metadata: path_for_artifact(&self.root, &request.metadata_path),
            },
            diagnostics,
            context: request.context,
        };
        fs::write(
            &request.metadata_path,
            serde_json::to_vec_pretty(&artifact)?,
        )?;
        Ok(artifact)
    }

    /// Runs an approved external renderer declared by a host-adapter manifest.
    /// Values are passed as process arguments (never through a shell), and the
    /// manifest must explicitly reference both input and output WAV paths.
    pub fn run_plugin_host(&self, request: PluginRenderRequest) -> Result<RunArtifact> {
        let adapter_path = self.approved_adapter_path(&request.adapter_path)?;
        let manifest = self.read_host_adapter(&adapter_path)?;
        let plugin_path = self.resolve(&request.plugin_path);
        let input_wav = self.resolve(&request.input_wav);
        let output_wav = self.resolve(&request.output_wav);
        if !plugin_path.exists() {
            bail!("plugin does not exist: {}", plugin_path.display());
        }
        if !input_wav.exists() {
            bail!("input WAV does not exist: {}", input_wav.display());
        }
        if !(0.01..=600.0).contains(&request.render_seconds) {
            bail!("render_seconds must be between 0.01 and 600");
        }
        for path in [&output_wav, &request.metadata_path] {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
        }
        let host_log = request.metadata_path.with_extension("host.log");
        if let Some(parent) = host_log.parent() {
            fs::create_dir_all(parent)?;
        }

        let executable = resolve_adapter_executable(&adapter_path, &manifest.executable);
        let variables = adapter_variables(
            &plugin_path,
            &input_wav,
            &output_wav,
            request.sample_rate,
            request.block_size,
            request.render_seconds,
        );
        let arguments = manifest
            .arguments
            .iter()
            .map(|argument| expand_adapter_template(argument, &variables))
            .collect::<Result<Vec<_>>>()?;
        if !manifest
            .arguments
            .iter()
            .any(|argument| argument.contains("{plugin}"))
            || !manifest
                .arguments
                .iter()
                .any(|argument| argument.contains("{input_wav}"))
            || !manifest
                .arguments
                .iter()
                .any(|argument| argument.contains("{output_wav}"))
        {
            bail!(
                "host adapter '{}' must include {{plugin}}, {{input_wav}}, and {{output_wav}} in its arguments",
                manifest.id
            );
        }
        let environment = manifest
            .environment
            .iter()
            .map(|(key, value)| Ok((key.clone(), expand_adapter_template(value, &variables)?)))
            .collect::<Result<Vec<_>>>()?;
        let mut command = Command::new(&executable);
        command
            .current_dir(&self.root)
            .args(&arguments)
            .envs(environment)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let command_line = command_to_vec(&executable, &command);
        let output = command
            .output()
            .with_context(|| format!("failed to launch host adapter '{}'", manifest.id))?;
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        fs::write(
            &host_log,
            format!("stdout:\n{stdout}\n\nstderr:\n{stderr}\n"),
        )?;

        let mut diagnostics = if output_wav.exists() {
            self.analyse_wav(&output_wav)?
        } else {
            RunDiagnostics {
                status: "severe".to_string(),
                warnings: vec!["host adapter did not produce its requested output WAV".to_string()],
                ..RunDiagnostics::default()
            }
        };
        if !output.status.success() {
            diagnostics.warnings.push(format!(
                "host adapter exited with status {:?}",
                output.status.code()
            ));
            diagnostics.status = "severe".to_string();
        }
        if let Some(actual_rate) = diagnostics.wav_sample_rate {
            if actual_rate != request.sample_rate as u32 {
                diagnostics.warnings.push(format!(
                    "host output sample rate is {actual_rate} Hz; requested {} Hz",
                    request.sample_rate
                ));
                if diagnostics.status == "clean" {
                    diagnostics.status = "warning".to_string();
                }
            }
        }
        let artifact = RunArtifact {
            schema: ARTIFACT_SCHEMA.to_string(),
            run_id: request.run_id,
            status: diagnostics.status.clone(),
            created_at_unix_ms: now_unix_ms(),
            command: command_line,
            exit_status: output.status.code(),
            stdout,
            stderr,
            git_revision: None,
            rig: format!("{} ({})", manifest.name, manifest.plugin_format),
            effective_rig: path_for_artifact(&self.root, &plugin_path),
            runtime: RenderRuntime {
                sample_rate: request.sample_rate,
                period_size: request.block_size,
                render_seconds: request.render_seconds,
                input_wav: path_for_artifact(&self.root, &input_wav),
                input_channel: 0,
                ir: String::new(),
                output_wav: path_for_artifact(&self.root, &output_wav),
                monitor: false,
            },
            artifacts: RunArtifacts {
                candidate_wav: path_for_artifact(&self.root, &output_wav),
                monitor_log: path_for_artifact(&self.root, &host_log),
                metadata: path_for_artifact(&self.root, &request.metadata_path),
            },
            diagnostics,
            context: request.context,
        };
        fs::write(
            &request.metadata_path,
            serde_json::to_vec_pretty(&artifact)?,
        )?;
        Ok(artifact)
    }

    pub fn read_host_adapter(&self, adapter_path: impl AsRef<Path>) -> Result<HostAdapterManifest> {
        let adapter_path = self.resolve(adapter_path);
        let manifest: HostAdapterManifest = serde_json::from_slice(
            &fs::read(&adapter_path)
                .with_context(|| format!("could not read adapter '{}'", adapter_path.display()))?,
        )
        .with_context(|| format!("could not parse adapter '{}'", adapter_path.display()))?;
        if manifest.schema != HOST_ADAPTER_SCHEMA {
            bail!(
                "unsupported host adapter schema '{}' (expected '{HOST_ADAPTER_SCHEMA}')",
                manifest.schema
            );
        }
        if manifest.id.trim().is_empty() || manifest.executable.trim().is_empty() {
            bail!("host adapter requires a non-empty id and executable");
        }
        Ok(manifest)
    }

    pub fn list_host_adapters(&self) -> Result<Vec<(String, HostAdapterManifest)>> {
        let directory = self.root.join("lab/adapters");
        if !directory.exists() {
            return Ok(Vec::new());
        }
        let mut paths = Vec::new();
        collect_json(&directory, &mut paths)?;
        paths.sort();
        let adapters = paths
            .into_iter()
            .filter_map(|path| {
                self.read_host_adapter(&path)
                    .ok()
                    .map(|manifest| (path_for_artifact(&self.root, &path), manifest))
            })
            .collect::<Vec<_>>();
        Ok(adapters)
    }

    fn approved_adapter_path(&self, path: impl AsRef<Path>) -> Result<PathBuf> {
        let directory = self.root.join("lab/adapters");
        let approved_directory = directory.canonicalize().with_context(|| {
            format!(
                "could not resolve adapter directory '{}'",
                directory.display()
            )
        })?;
        let adapter_path = self
            .resolve(path)
            .canonicalize()
            .context("could not resolve host adapter path")?;
        if !adapter_path.starts_with(&approved_directory) {
            bail!(
                "host adapter must be installed under '{}'",
                directory.display()
            );
        }
        Ok(adapter_path)
    }

    pub fn analyse_render(
        &self,
        monitor_log: &Path,
        candidate_wav: &Path,
    ) -> Result<RunDiagnostics> {
        let monitor = parse_monitor_log(monitor_log)?;
        let wav = if candidate_wav.exists() {
            Some(wav_summary(candidate_wav)?)
        } else {
            None
        };
        let xrun_count = monitor.input_xruns + monitor.output_xruns;
        let hard_clip_count = monitor.input_hard_clips + monitor.output_hard_clips;
        let near_clip_count = monitor.input_near_clips + monitor.output_near_clips;
        let mut warnings = Vec::new();
        if xrun_count > 0 {
            warnings.push(format!("xrun count is {xrun_count}"));
        }
        if hard_clip_count > 0 {
            warnings.push(format!("hard clip count is {hard_clip_count}"));
        }
        if near_clip_count > 0 {
            warnings.push(format!("near clip count is {near_clip_count}"));
        }
        if monitor.entries == 0 {
            warnings.push("monitor log has no MON entries".to_string());
        }
        if wav
            .as_ref()
            .is_some_and(|summary| summary.peak <= 0.000_001)
        {
            warnings.push("candidate output appears silent".to_string());
        }
        let status = if xrun_count > 0 || hard_clip_count > 0 {
            "severe"
        } else if warnings.is_empty() {
            "clean"
        } else {
            "warning"
        };
        Ok(RunDiagnostics {
            status: status.to_string(),
            warnings,
            monitor_entries: monitor.entries,
            input_rms_dbfs: monitor.input_rms_dbfs,
            input_peak_dbfs: monitor.input_peak_dbfs,
            output_rms_dbfs: monitor.output_rms_dbfs,
            output_peak_dbfs: monitor.output_peak_dbfs,
            xrun_count,
            hard_clip_count,
            near_clip_count,
            wav_sample_rate: wav.as_ref().map(|summary| summary.sample_rate),
            wav_channels: wav.as_ref().map(|summary| summary.channels),
            wav_frames: wav.as_ref().map(|summary| summary.frames),
            wav_duration_seconds: wav.as_ref().map(|summary| summary.duration_seconds),
            wav_rms: wav.as_ref().map(|summary| summary.rms),
            wav_peak: wav.as_ref().map(|summary| summary.peak),
        })
    }

    /// Inspects a WAV when no runtime monitor log exists. The result deliberately
    /// reports only file health; it does not make a claim about tone quality.
    pub fn analyse_wav(&self, candidate_wav: impl AsRef<Path>) -> Result<RunDiagnostics> {
        let summary = wav_summary(&self.resolve(candidate_wav))?;
        let mut warnings = Vec::new();
        if summary.peak <= 0.000_001 {
            warnings.push("candidate output appears silent".to_string());
        }
        Ok(RunDiagnostics {
            status: if warnings.is_empty() {
                "clean".to_string()
            } else {
                "warning".to_string()
            },
            warnings,
            output_rms_dbfs: db(summary.rms),
            output_peak_dbfs: db(summary.peak),
            wav_sample_rate: Some(summary.sample_rate),
            wav_channels: Some(summary.channels),
            wav_frames: Some(summary.frames),
            wav_duration_seconds: Some(summary.duration_seconds),
            wav_rms: Some(summary.rms),
            wav_peak: Some(summary.peak),
            ..RunDiagnostics::default()
        })
    }

    pub fn record_wav_analysis(&self, request: WavAnalysisRequest) -> Result<RunArtifact> {
        let candidate_wav = self.resolve(&request.candidate_wav);
        if !candidate_wav.exists() {
            bail!("candidate WAV does not exist: {}", candidate_wav.display());
        }
        if let Some(parent) = request.metadata_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let diagnostics = self.analyse_wav(&candidate_wav)?;
        let artifact = RunArtifact {
            schema: ARTIFACT_SCHEMA.to_string(),
            run_id: request.run_id,
            status: diagnostics.status.clone(),
            created_at_unix_ms: now_unix_ms(),
            command: vec!["external-wav-analysis".to_string()],
            exit_status: Some(0),
            stdout: String::new(),
            stderr: String::new(),
            git_revision: None,
            rig: request.label,
            effective_rig: String::new(),
            runtime: RenderRuntime {
                sample_rate: diagnostics.wav_sample_rate.unwrap_or_default().into(),
                period_size: 0,
                render_seconds: diagnostics.wav_duration_seconds.unwrap_or_default(),
                input_wav: path_for_artifact(&self.root, &candidate_wav),
                input_channel: 0,
                ir: String::new(),
                output_wav: path_for_artifact(&self.root, &candidate_wav),
                monitor: false,
            },
            artifacts: RunArtifacts {
                candidate_wav: path_for_artifact(&self.root, &candidate_wav),
                monitor_log: String::new(),
                metadata: path_for_artifact(&self.root, &request.metadata_path),
            },
            diagnostics,
            context: request.context,
        };
        fs::write(
            &request.metadata_path,
            serde_json::to_vec_pretty(&artifact)?,
        )?;
        Ok(artifact)
    }

    pub fn load_workspace(&self) -> Result<WorkspaceSnapshot> {
        let mut paths = Vec::new();
        collect_json(&self.root.join("lab/renders/mcp"), &mut paths)?;
        paths.sort_by(|left, right| right.cmp(left));
        let mut runs = Vec::new();
        for path in paths.into_iter().take(100) {
            let text = fs::read_to_string(&path)?;
            if let Ok(run) = serde_json::from_str::<RunArtifact>(&text) {
                if run.schema == ARTIFACT_SCHEMA {
                    runs.push(run);
                }
            }
        }
        runs.sort_by(|left, right| right.created_at_unix_ms.cmp(&left.created_at_unix_ms));
        Ok(WorkspaceSnapshot { runs })
    }

    pub fn waveform_preview(&self, wav: impl AsRef<Path>, bins: usize) -> Result<WaveformPreview> {
        let path = self.resolve(wav);
        let mut reader = hound::WavReader::open(&path)
            .with_context(|| format!("could not open WAV '{}'", path.display()))?;
        let spec = reader.spec();
        let samples = read_normalized_samples(&mut reader)?;
        let channels = usize::from(spec.channels.max(1));
        let frames = samples.len() / channels;
        let bins = bins.clamp(8, 4_096).min(frames.max(1));
        let mut min = vec![1.0_f32; bins];
        let mut max = vec![-1.0_f32; bins];
        for frame in 0..frames {
            let sample = samples[frame * channels];
            let bin = frame * bins / frames.max(1);
            min[bin] = min[bin].min(sample);
            max[bin] = max[bin].max(sample);
        }
        for (min, max) in min.iter_mut().zip(max.iter_mut()) {
            if *min > *max {
                *min = 0.0;
                *max = 0.0;
            }
        }
        Ok(WaveformPreview {
            sample_rate: spec.sample_rate,
            channels: spec.channels,
            duration_seconds: frames as f64 / f64::from(spec.sample_rate.max(1)),
            min,
            max,
        })
    }

    /// Returns a compact, Hann-windowed magnitude spectrum for visual analysis.
    /// It is a preview, not a substitute for the full comparison metrics.
    pub fn spectrum_preview(&self, wav: impl AsRef<Path>, bins: usize) -> Result<SpectrumPreview> {
        let path = self.resolve(wav);
        let mut reader = hound::WavReader::open(&path)
            .with_context(|| format!("could not open WAV '{}'", path.display()))?;
        let spec = reader.spec();
        let interleaved = read_normalized_samples(&mut reader)?;
        let channels = usize::from(spec.channels.max(1));
        let mono: Vec<f32> = interleaved.chunks(channels).map(|frame| frame[0]).collect();
        let fft_size = mono.len().min(4_096).next_power_of_two() / 2;
        if fft_size < 16 {
            bail!("WAV is too short for a spectrum preview");
        }
        // A render often ends in silence. Pick the most energetic overlapping
        // window so the preview shows the signal under test rather than its tail.
        let hop = (fft_size / 2).max(1);
        let start = (0..=mono.len() - fft_size)
            .step_by(hop)
            .max_by(|left, right| {
                let energy = |start: &usize| -> f32 {
                    mono[*start..*start + fft_size]
                        .iter()
                        .map(|sample| sample * sample)
                        .sum()
                };
                energy(left).total_cmp(&energy(right))
            })
            .unwrap_or(0);
        let mut signal: Vec<Complex32> = mono[start..start + fft_size]
            .iter()
            .enumerate()
            .map(|(index, sample)| {
                let phase = std::f32::consts::TAU * index as f32 / (fft_size - 1) as f32;
                Complex32::new(sample * (0.5 - 0.5 * phase.cos()), 0.0)
            })
            .collect();
        let mut planner = FftPlanner::<f32>::new();
        planner.plan_fft_forward(fft_size).process(&mut signal);
        let half = fft_size / 2;
        let bins = bins.clamp(8, 256).min(half);
        let mut frequencies_hz = Vec::with_capacity(bins);
        let mut magnitudes_db = Vec::with_capacity(bins);
        for index in 0..bins {
            let first = index * half / bins;
            let last = ((index + 1) * half / bins).max(first + 1).min(half);
            let magnitude = signal[first..last]
                .iter()
                .map(|value| value.norm() / fft_size as f32)
                .fold(0.0_f32, f32::max);
            frequencies_hz
                .push((first + last) as f32 * 0.5 * spec.sample_rate as f32 / fft_size as f32);
            magnitudes_db.push((20.0 * magnitude.max(1e-9).log10()).clamp(-120.0, 0.0));
        }
        Ok(SpectrumPreview {
            sample_rate: spec.sample_rate,
            frequencies_hz,
            magnitudes_db,
        })
    }

    /// Compare two WAVs with bounded cross-correlation alignment and RMS gain
    /// matching. This is a fast diagnostic preview; the Python workflow remains
    /// the full research-grade comparison report path.
    pub fn compare_wavs(
        &self,
        candidate: impl AsRef<Path>,
        reference: impl AsRef<Path>,
    ) -> Result<WavComparison> {
        let candidate_path = self.resolve(candidate);
        let reference_path = self.resolve(reference);
        let (candidate_spec, candidate_samples) = read_mono_wav(&candidate_path)?;
        let (reference_spec, reference_samples) = read_mono_wav(&reference_path)?;
        let mut warnings = Vec::new();
        if candidate_spec.sample_rate != reference_spec.sample_rate {
            warnings.push(format!(
                "sample-rate mismatch: candidate {} Hz, reference {} Hz",
                candidate_spec.sample_rate, reference_spec.sample_rate
            ));
        }
        let usable = candidate_samples.len().min(reference_samples.len());
        if usable < 32 {
            bail!("WAVs are too short for comparison");
        }
        let max_lag = usable.min(1_024) / 4;
        let probe = usable.min(8_192).saturating_sub(max_lag * 2).max(16);
        let mut best_lag = 0_i64;
        let mut best_score = f64::NEG_INFINITY;
        for lag in -(max_lag as i64)..=(max_lag as i64) {
            let candidate_start = lag.max(0) as usize;
            let reference_start = (-lag).max(0) as usize;
            if candidate_start + probe > candidate_samples.len()
                || reference_start + probe > reference_samples.len()
            {
                continue;
            }
            let (dot, candidate_energy, reference_energy) = candidate_samples
                [candidate_start..candidate_start + probe]
                .iter()
                .zip(&reference_samples[reference_start..reference_start + probe])
                .fold(
                    (0.0, 0.0, 0.0),
                    |(dot, candidate_energy, reference_energy), (candidate, reference)| {
                        let candidate = f64::from(*candidate);
                        let reference = f64::from(*reference);
                        (
                            dot + candidate * reference,
                            candidate_energy + candidate * candidate,
                            reference_energy + reference * reference,
                        )
                    },
                );
            let score = dot
                / (candidate_energy * reference_energy)
                    .sqrt()
                    .max(f64::EPSILON);
            if score > best_score + 1e-12
                || ((score - best_score).abs() <= 1e-12 && lag.abs() < best_lag.abs())
            {
                best_score = score;
                best_lag = lag;
            }
        }
        let candidate_start = best_lag.max(0) as usize;
        let reference_start = (-best_lag).max(0) as usize;
        let aligned = (candidate_samples.len() - candidate_start)
            .min(reference_samples.len() - reference_start);
        let candidate_slice = &candidate_samples[candidate_start..candidate_start + aligned];
        let reference_slice = &reference_samples[reference_start..reference_start + aligned];
        let candidate_rms = rms(candidate_slice);
        let reference_rms = rms(reference_slice);
        let gain = if candidate_rms > f64::EPSILON {
            reference_rms / candidate_rms
        } else {
            1.0
        };
        let residual_rms = (candidate_slice
            .iter()
            .zip(reference_slice)
            .map(|(candidate, reference)| {
                let residual = f64::from(*candidate) * gain - f64::from(*reference);
                residual * residual
            })
            .sum::<f64>()
            / aligned as f64)
            .sqrt();
        if candidate_rms <= f64::EPSILON || reference_rms <= f64::EPSILON {
            warnings.push("one input is silent; null comparison is not meaningful".to_string());
        }
        Ok(WavComparison {
            candidate: path_for_artifact(&self.root, &candidate_path),
            reference: path_for_artifact(&self.root, &reference_path),
            status: if warnings.is_empty() {
                "clean"
            } else {
                "warning"
            }
            .to_string(),
            warnings,
            sample_rate: (candidate_spec.sample_rate == reference_spec.sample_rate)
                .then_some(candidate_spec.sample_rate),
            aligned_frames: aligned as u64,
            latency_samples: best_lag,
            gain_correction_db: db(gain),
            candidate_rms: Some(candidate_rms),
            reference_rms: Some(reference_rms),
            residual_rms: Some(residual_rms),
            null_relative_db: (reference_rms > f64::EPSILON)
                .then(|| 20.0 * (residual_rms / reference_rms).max(1e-12).log10()),
        })
    }

    /// Runs the existing offline research comparison and returns its persisted
    /// Markdown report. The inputs are explicit files; this is not a generic
    /// command-execution API.
    pub fn generate_comparison_report(
        &self,
        request: ComparisonReportRequest,
    ) -> Result<ComparisonReport> {
        let candidate = self.resolve(&request.candidate);
        let reference = self.resolve(&request.reference);
        let report = self.resolve(&request.report);
        if !candidate.exists() || !reference.exists() {
            bail!("candidate and reference WAVs must both exist");
        }
        if let Some(parent) = report.parent() {
            fs::create_dir_all(parent)?;
        }
        let python_project = comparison_project_dir(&self.root);
        let mut command = Command::new("uv");
        command
            .current_dir(&self.root)
            .arg("--project")
            .arg(&python_project)
            .arg("run")
            .arg("greybound-lab")
            .arg("compare-wav")
            .arg("--candidate")
            .arg(&candidate)
            .arg("--reference")
            .arg(&reference)
            .arg("--report")
            .arg(&report);
        if let Some(metadata) = request.metadata {
            command.arg("--metadata").arg(self.resolve(metadata));
        }
        if let Some(segments) = request.segments {
            command.arg("--segments").arg(self.resolve(segments));
        }
        let command_line = command_to_vec(Path::new("uv"), &command);
        let output = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .context("could not launch the offline comparison runner 'uv'")?;
        Ok(ComparisonReport {
            status: if output.status.success() {
                "clean".to_string()
            } else {
                "severe".to_string()
            },
            report: path_for_artifact(&self.root, &report),
            command: command_line,
            stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            exit_status: output.status.code(),
        })
    }

    pub fn resolve(&self, path: impl AsRef<Path>) -> PathBuf {
        let path = path.as_ref();
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        }
    }
}

#[derive(Default)]
struct MonitorAggregate {
    entries: usize,
    input_rms_dbfs: Option<f64>,
    input_peak_dbfs: Option<f64>,
    output_rms_dbfs: Option<f64>,
    output_peak_dbfs: Option<f64>,
    input_near_clips: u64,
    input_hard_clips: u64,
    output_near_clips: u64,
    output_hard_clips: u64,
    input_xruns: u64,
    output_xruns: u64,
}

struct WavSummary {
    sample_rate: u32,
    channels: u16,
    frames: u64,
    duration_seconds: f64,
    rms: f64,
    peak: f64,
}

fn parse_monitor_log(path: &Path) -> Result<MonitorAggregate> {
    if !path.exists() {
        return Ok(MonitorAggregate::default());
    }
    let text = fs::read_to_string(path)?;
    let mut aggregate = MonitorAggregate::default();
    for line in text
        .lines()
        .filter(|line| line.contains(" MON ") || line.trim_start().starts_with("MON "))
    {
        let fields = parse_monitor_line(line);
        if fields.is_empty() {
            continue;
        }
        aggregate.entries += 1;
        aggregate.input_rms_dbfs = fields.get("input_rms_dbfs").copied();
        aggregate.input_peak_dbfs = fields.get("input_peak_dbfs").copied();
        aggregate.output_rms_dbfs = fields.get("output_rms_dbfs").copied();
        aggregate.output_peak_dbfs = fields.get("output_peak_dbfs").copied();
        aggregate.input_near_clips += fields.get("input_near").copied().unwrap_or(0.0) as u64;
        aggregate.input_hard_clips += fields.get("input_clip").copied().unwrap_or(0.0) as u64;
        aggregate.output_near_clips += fields.get("output_near").copied().unwrap_or(0.0) as u64;
        aggregate.output_hard_clips += fields.get("output_clip").copied().unwrap_or(0.0) as u64;
        aggregate.input_xruns += fields.get("xrun_in").copied().unwrap_or(0.0) as u64;
        aggregate.output_xruns += fields.get("xrun_out").copied().unwrap_or(0.0) as u64;
    }
    Ok(aggregate)
}

fn parse_monitor_line(line: &str) -> BTreeMap<&'static str, f64> {
    let mut fields = BTreeMap::new();
    if let Some(value) = value_after(line, "input rms ") {
        fields.insert("input_rms", value);
    }
    if let Some(value) = db_after(line, "input rms ") {
        fields.insert("input_rms_dbfs", value);
    }
    if let Some(value) = value_after(line, "peak ") {
        fields.insert("input_peak", value);
    }
    if let Some(value) = db_after(line, "peak ") {
        fields.insert("input_peak_dbfs", value);
    }
    if let Some(value) = pair_after(line, "near/clip ", 0) {
        fields.insert("input_near", value);
    }
    if let Some(value) = pair_after(line, "near/clip ", 1) {
        fields.insert("input_clip", value);
    }
    if let Some(output) = line.split(" | output ").nth(1) {
        if let Some(value) = value_after(output, "rms ") {
            fields.insert("output_rms", value);
        }
        if let Some(value) = db_after(output, "rms ") {
            fields.insert("output_rms_dbfs", value);
        }
        if let Some(value) = value_after(output, "peak ") {
            fields.insert("output_peak", value);
        }
        if let Some(value) = db_after(output, "peak ") {
            fields.insert("output_peak_dbfs", value);
        }
        if let Some(value) = pair_after(output, "near/clip ", 0) {
            fields.insert("output_near", value);
        }
        if let Some(value) = pair_after(output, "near/clip ", 1) {
            fields.insert("output_clip", value);
        }
    }
    if let Some(value) = pair_after(line, "xrun in/out ", 0) {
        fields.insert("xrun_in", value);
    }
    if let Some(value) = pair_after(line, "xrun in/out ", 1) {
        fields.insert("xrun_out", value);
    }
    fields
}

fn value_after(text: &str, marker: &str) -> Option<f64> {
    text.split(marker)
        .nth(1)?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

fn db_after(text: &str, marker: &str) -> Option<f64> {
    text.split(marker)
        .nth(1)?
        .split('(')
        .nth(1)?
        .split(" dBFS")
        .next()?
        .parse()
        .ok()
}

fn pair_after(text: &str, marker: &str, index: usize) -> Option<f64> {
    text.split(marker)
        .nth(1)?
        .split_whitespace()
        .next()?
        .split('/')
        .nth(index)?
        .parse()
        .ok()
}

fn wav_summary(path: &Path) -> Result<WavSummary> {
    let mut reader = hound::WavReader::open(path)?;
    let spec = reader.spec();
    let samples = read_normalized_samples(&mut reader)?;
    let mut sum_squares = 0.0_f64;
    let mut peak = 0.0_f64;
    for sample in &samples {
        let sample = f64::from(*sample);
        sum_squares += sample * sample;
        peak = peak.max(sample.abs());
    }
    let frames = samples.len() as u64 / u64::from(spec.channels.max(1));
    Ok(WavSummary {
        sample_rate: spec.sample_rate,
        channels: spec.channels,
        frames,
        duration_seconds: frames as f64 / f64::from(spec.sample_rate.max(1)),
        rms: if samples.is_empty() {
            0.0
        } else {
            (sum_squares / samples.len() as f64).sqrt()
        },
        peak,
    })
}

fn read_normalized_samples(
    reader: &mut hound::WavReader<std::io::BufReader<std::fs::File>>,
) -> Result<Vec<f32>> {
    let spec = reader.spec();
    match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into),
        hound::SampleFormat::Int => {
            let scale =
                (1_i64 << (u32::from(spec.bits_per_sample).saturating_sub(1))).max(1) as f32;
            reader
                .samples::<i32>()
                .map(|sample| sample.map(|sample| sample as f32 / scale))
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(Into::into)
        }
    }
}

fn read_mono_wav(path: &Path) -> Result<(hound::WavSpec, Vec<f32>)> {
    let mut reader = hound::WavReader::open(path)?;
    let spec = reader.spec();
    let samples = read_normalized_samples(&mut reader)?;
    let channels = usize::from(spec.channels.max(1));
    let mono = samples
        .chunks(channels)
        .map(|frame| frame.iter().copied().sum::<f32>() / frame.len() as f32)
        .collect();
    Ok((spec, mono))
}

fn rms(samples: &[f32]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    (samples
        .iter()
        .map(|sample| f64::from(*sample) * f64::from(*sample))
        .sum::<f64>()
        / samples.len() as f64)
        .sqrt()
}

fn db(value: f64) -> Option<f64> {
    (value > 0.0).then(|| 20.0 * value.log10())
}

fn resolve_adapter_executable(adapter_path: &Path, executable: &str) -> PathBuf {
    let executable = Path::new(executable);
    if executable.is_absolute() || executable.components().count() == 1 {
        executable.to_path_buf()
    } else {
        adapter_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(executable)
    }
}

fn adapter_variables(
    plugin: &Path,
    input_wav: &Path,
    output_wav: &Path,
    sample_rate: u64,
    block_size: u64,
    render_seconds: f64,
) -> BTreeMap<&'static str, String> {
    BTreeMap::from([
        ("{plugin}", plugin.display().to_string()),
        ("{input_wav}", input_wav.display().to_string()),
        ("{output_wav}", output_wav.display().to_string()),
        ("{sample_rate}", sample_rate.to_string()),
        ("{block_size}", block_size.to_string()),
        ("{render_seconds}", render_seconds.to_string()),
    ])
}

fn expand_adapter_template(template: &str, variables: &BTreeMap<&str, String>) -> Result<String> {
    let mut expanded = template.to_string();
    for (placeholder, value) in variables {
        expanded = expanded.replace(placeholder, value);
    }
    if expanded.contains('{') || expanded.contains('}') {
        bail!("unsupported host-adapter placeholder in '{template}'");
    }
    Ok(expanded)
}

fn comparison_project_dir(workspace_root: &Path) -> PathBuf {
    if let Some(path) = std::env::var_os("GREYBOUND_LAB_PYTHON_PROJECT") {
        return PathBuf::from(path);
    }
    let workspace_project = workspace_root.join("lab");
    if workspace_project.join("pyproject.toml").exists() {
        return workspace_project;
    }
    if let Ok(executable) = std::env::current_exe() {
        if let Some(bundle) = executable.parent().and_then(Path::parent) {
            let bundled_project = bundle.join("runtime/lab");
            if bundled_project.join("pyproject.toml").exists() {
                return bundled_project;
            }
        }
    }
    workspace_project
}

fn collect_json(dir: &Path, paths: &mut Vec<PathBuf>) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_json(&path, paths)?;
        }
        if path.extension().and_then(|extension| extension.to_str()) == Some("json") {
            paths.push(path);
        }
    }
    Ok(())
}

fn command_to_vec(binary: &Path, command: &Command) -> Vec<String> {
    let mut values = vec![binary.display().to_string()];
    values.extend(
        command
            .get_args()
            .map(|argument| argument.to_string_lossy().into_owned()),
    );
    values
}

fn path_for_artifact(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_ROOT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn waveform_preview_captures_signal_extrema() {
        let root = test_root();
        fs::create_dir_all(&root).unwrap();
        let wav = root.join("signal.wav");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 48_000,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::create(&wav, spec).unwrap();
        for sample in [-0.8_f32, -0.2, 0.4, 0.9] {
            writer.write_sample(sample).unwrap();
        }
        writer.finalize().unwrap();
        let preview = LabCore::new(&root)
            .waveform_preview("signal.wav", 8)
            .unwrap();
        assert_eq!(preview.min.len(), 4);
        assert!(preview.min.iter().any(|sample| *sample <= -0.8));
        assert!(preview.max.iter().any(|sample| *sample >= 0.9));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn monitor_analysis_becomes_a_versioned_health_record() {
        let root = test_root();
        fs::create_dir_all(&root).unwrap();
        let wav = root.join("signal.wav");
        let monitor = root.join("signal.monitor.log");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 48_000,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::create(&wav, spec).unwrap();
        writer.write_sample(0.25_f32).unwrap();
        writer.finalize().unwrap();
        fs::write(
            &monitor,
            "MON input rms 0.10000 (-20.0 dBFS) peak 0.20000 (-14.0 dBFS) near/clip 0/0 | output rms 0.25000 (-12.0 dBFS) peak 0.50000 (-6.0 dBFS) near/clip 1/0 | xrun in/out 0/0\n",
        )
        .unwrap();
        let diagnostics = LabCore::new(&root).analyse_render(&monitor, &wav).unwrap();
        assert_eq!(diagnostics.status, "warning");
        assert_eq!(diagnostics.monitor_entries, 1);
        assert_eq!(diagnostics.near_clip_count, 1);
        assert_eq!(diagnostics.output_peak_dbfs, Some(-6.0));
        assert_eq!(diagnostics.wav_frames, Some(1));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn external_wav_analysis_is_discoverable_in_workspace_history() {
        let root = test_root();
        fs::create_dir_all(root.join("lab/renders/mcp")).unwrap();
        let wav = root.join("external.wav");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 44_100,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::create(&wav, spec).unwrap();
        writer.write_sample(0.25_f32).unwrap();
        writer.finalize().unwrap();
        let core = LabCore::new(&root);
        let run = core
            .record_wav_analysis(WavAnalysisRequest {
                run_id: "external-smoke".to_string(),
                candidate_wav: wav,
                metadata_path: root.join("lab/renders/mcp/external-smoke.run.json"),
                label: "External plugin output".to_string(),
                context: serde_json::json!({ "initiator": "test" }),
            })
            .unwrap();
        assert_eq!(run.runtime.sample_rate, 44_100);
        assert_eq!(
            core.load_workspace().unwrap().runs[0].run_id,
            "external-smoke"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn host_adapter_runs_a_bounded_external_renderer_and_records_artifact() {
        use std::os::unix::fs::PermissionsExt;

        let root = test_root();
        let adapters = root.join("lab/adapters");
        let renders = root.join("lab/renders/mcp");
        fs::create_dir_all(&adapters).unwrap();
        fs::create_dir_all(&renders).unwrap();
        let input = root.join("input.wav");
        let output = renders.join("adapter-smoke.wav");
        let plugin = root.join("fixture.clap");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 48_000,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::create(&input, spec).unwrap();
        writer.write_sample(0.25_f32).unwrap();
        writer.finalize().unwrap();
        fs::write(&plugin, "fixture").unwrap();
        let renderer = adapters.join("copy-renderer.sh");
        fs::write(&renderer, "#!/bin/sh\ncp \"$2\" \"$3\"\n").unwrap();
        fs::set_permissions(&renderer, fs::Permissions::from_mode(0o755)).unwrap();
        let adapter = adapters.join("copy.adapter.json");
        fs::write(
            &adapter,
            serde_json::to_vec_pretty(&HostAdapterManifest {
                schema: HOST_ADAPTER_SCHEMA.to_string(),
                id: "copy-renderer".to_string(),
                name: "Copy renderer".to_string(),
                plugin_format: "clap".to_string(),
                executable: "./copy-renderer.sh".to_string(),
                arguments: vec![
                    "{plugin}".to_string(),
                    "{input_wav}".to_string(),
                    "{output_wav}".to_string(),
                ],
                environment: BTreeMap::new(),
            })
            .unwrap(),
        )
        .unwrap();
        let run = LabCore::new(&root)
            .run_plugin_host(PluginRenderRequest {
                run_id: "adapter-smoke".to_string(),
                adapter_path: adapter.clone(),
                plugin_path: plugin,
                input_wav: input,
                output_wav: output,
                metadata_path: renders.join("adapter-smoke.run.json"),
                sample_rate: 48_000,
                block_size: 64,
                render_seconds: 1.0,
                context: serde_json::json!({ "initiator": "test" }),
            })
            .unwrap();
        assert_eq!(run.status, "clean");
        assert!(root.join(&run.artifacts.candidate_wav).exists());
        assert!(root.join(&run.artifacts.monitor_log).exists());
        assert_eq!(LabCore::new(&root).list_host_adapters().unwrap().len(), 1);
        let unapproved_adapter = root.join("unapproved.adapter.json");
        fs::copy(&adapter, &unapproved_adapter).unwrap();
        assert!(LabCore::new(&root)
            .approved_adapter_path(&unapproved_adapter)
            .is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn spectrum_preview_has_requested_visual_bins() {
        let root = test_root();
        fs::create_dir_all(&root).unwrap();
        let wav = root.join("tone.wav");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 48_000,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::create(&wav, spec).unwrap();
        for index in 0..128 {
            let phase = std::f32::consts::TAU * 8.0 * index as f32 / 128.0;
            writer.write_sample(phase.sin()).unwrap();
        }
        writer.finalize().unwrap();
        let spectrum = LabCore::new(&root)
            .spectrum_preview("tone.wav", 16)
            .unwrap();
        assert_eq!(spectrum.magnitudes_db.len(), 16);
        assert_eq!(spectrum.frequencies_hz.len(), 16);
        assert!(spectrum.magnitudes_db.iter().any(|value| *value > -20.0));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn comparison_aligns_delay_and_matches_gain() {
        let root = test_root();
        fs::create_dir_all(&root).unwrap();
        let reference = root.join("reference.wav");
        let candidate = root.join("candidate.wav");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 48_000,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let signal: Vec<f32> = (0..256)
            .map(|index| (std::f32::consts::TAU * 7.0 * index as f32 / 256.0).sin())
            .collect();
        let mut reference_writer = hound::WavWriter::create(&reference, spec).unwrap();
        for sample in &signal {
            reference_writer.write_sample(*sample).unwrap();
        }
        reference_writer.finalize().unwrap();
        let mut candidate_writer = hound::WavWriter::create(&candidate, spec).unwrap();
        for _ in 0..8 {
            candidate_writer.write_sample(0.0_f32).unwrap();
        }
        for sample in &signal {
            candidate_writer.write_sample(sample * 0.5).unwrap();
        }
        candidate_writer.finalize().unwrap();
        let comparison = LabCore::new(&root)
            .compare_wavs("candidate.wav", "reference.wav")
            .unwrap();
        assert_eq!(comparison.latency_samples, 8);
        assert!(comparison.gain_correction_db.unwrap() > 5.9);
        assert!(comparison.null_relative_db.unwrap() < -80.0);
        let identity = LabCore::new(&root)
            .compare_wavs("reference.wav", "reference.wav")
            .unwrap();
        assert_eq!(identity.latency_samples, 0);
        assert!(identity.null_relative_db.unwrap() < -100.0);
        fs::remove_dir_all(root).unwrap();
    }

    fn test_root() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let sequence = TEST_ROOT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "greybound-lab-core-{}-{nonce}-{sequence}",
            std::process::id()
        ))
    }
}
