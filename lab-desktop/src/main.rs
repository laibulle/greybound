use anyhow::{bail, Context, Result};
use greybound_lab_core::{
    ComparisonReport, ComparisonReportRequest, LabCore, PluginRenderRequest, RenderRequest,
    RenderRuntime, RunArtifact, SpectrumPreview, WavAnalysisRequest, WavComparison,
    WaveformPreview, WorkspaceSnapshot,
};
use iced::widget::{button, column, container, row, scrollable, text, text_input, Space};
use iced::{Alignment, Application, Command, Element, Length, Settings, Theme};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_RIG: &str = "rigs/grey-nox.json5";
const DEFAULT_INPUT: &str = "lab/references/tone3000-inputs/Brit - Guitar.wav";
const DEFAULT_IR: &str = "lab/references/tone3000-irs/celestion.wav";

fn main() -> iced::Result {
    LabDesktop::run(Settings {
        window: iced::window::Settings {
            size: (1_280, 820),
            min_size: Some((900, 620)),
            ..iced::window::Settings::default()
        },
        antialiasing: true,
        ..Settings::default()
    })
}

struct LabDesktop {
    root: PathBuf,
    rig: String,
    input: String,
    ir: String,
    external_wav: String,
    reference_wav: String,
    host_adapter: String,
    hosted_plugin: String,
    seconds: String,
    snapshot: WorkspaceSnapshot,
    selected: Option<RunArtifact>,
    waveform: Option<WaveformPreview>,
    spectrum: Option<SpectrumPreview>,
    comparison: Option<WavComparison>,
    full_report: Option<FullReportView>,
    status: String,
    running: bool,
}

#[derive(Debug, Clone)]
enum Message {
    Refresh,
    WorkspaceLoaded(Result<WorkspaceView, String>),
    RigChanged(String),
    InputChanged(String),
    IrChanged(String),
    ExternalWavChanged(String),
    ReferenceWavChanged(String),
    HostAdapterChanged(String),
    HostedPluginChanged(String),
    SecondsChanged(String),
    PickRig,
    PickInput,
    PickIr,
    PickExternalWav,
    PickReferenceWav,
    PickHostAdapter,
    PickHostedPlugin,
    PickHostedPluginFolder,
    RigPicked(Option<PathBuf>),
    InputPicked(Option<PathBuf>),
    IrPicked(Option<PathBuf>),
    ExternalWavPicked(Option<PathBuf>),
    ReferenceWavPicked(Option<PathBuf>),
    HostAdapterPicked(Option<PathBuf>),
    HostedPluginPicked(Option<PathBuf>),
    RunExperiment,
    ExperimentFinished(Result<RunArtifact, String>),
    ImportExternalWav,
    ExternalWavImported(Result<RunArtifact, String>),
    CompareSelected,
    ComparisonFinished(Result<WavComparison, String>),
    GenerateFullReport,
    FullReportFinished(Result<FullReportView, String>),
    RunHostedPlugin,
    HostedPluginFinished(Result<RunArtifact, String>),
    SelectRun(String),
}

#[derive(Debug, Clone)]
struct WorkspaceView {
    snapshot: WorkspaceSnapshot,
    selected: Option<RunArtifact>,
    waveform: Option<WaveformPreview>,
    spectrum: Option<SpectrumPreview>,
}

#[derive(Debug, Clone)]
struct FullReportView {
    report: ComparisonReport,
    excerpt: String,
}

impl Application for LabDesktop {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let root = workspace_root().unwrap_or_else(|_| std::env::current_dir().unwrap_or_default());
        let app = Self {
            root: root.clone(),
            rig: DEFAULT_RIG.to_string(),
            input: DEFAULT_INPUT.to_string(),
            ir: DEFAULT_IR.to_string(),
            external_wav: String::new(),
            reference_wav: String::new(),
            host_adapter: String::new(),
            hosted_plugin: String::new(),
            seconds: "3.0".to_string(),
            snapshot: WorkspaceSnapshot::default(),
            selected: None,
            waveform: None,
            spectrum: None,
            comparison: None,
            full_report: None,
            status: "Ready. Select a rig, a DI WAV, and an IR, then run a bounded experiment."
                .to_string(),
            running: false,
        };
        (
            app,
            Command::perform(load_workspace(root), Message::WorkspaceLoaded),
        )
    }

    fn title(&self) -> String {
        "Greybound Lab".to_string()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Refresh => {
                Command::perform(load_workspace(self.root.clone()), Message::WorkspaceLoaded)
            }
            Message::WorkspaceLoaded(Ok(view)) => {
                self.snapshot = view.snapshot;
                self.selected = view.selected;
                self.waveform = view.waveform;
                self.spectrum = view.spectrum;
                self.comparison = None;
                self.full_report = None;
                self.status = format!(
                    "Workspace loaded — {} reproducible run(s).",
                    self.snapshot.runs.len()
                );
                Command::none()
            }
            Message::WorkspaceLoaded(Err(error)) => {
                self.status = error;
                Command::none()
            }
            Message::RigChanged(value) => {
                self.rig = value;
                Command::none()
            }
            Message::InputChanged(value) => {
                self.input = value;
                Command::none()
            }
            Message::IrChanged(value) => {
                self.ir = value;
                Command::none()
            }
            Message::ExternalWavChanged(value) => {
                self.external_wav = value;
                Command::none()
            }
            Message::ReferenceWavChanged(value) => {
                self.reference_wav = value;
                Command::none()
            }
            Message::HostAdapterChanged(value) => {
                self.host_adapter = value;
                Command::none()
            }
            Message::HostedPluginChanged(value) => {
                self.hosted_plugin = value;
                Command::none()
            }
            Message::SecondsChanged(value) => {
                self.seconds = value;
                Command::none()
            }
            Message::PickRig => Command::perform(pick_file("Rig", &["json5"]), Message::RigPicked),
            Message::PickInput => {
                Command::perform(pick_file("Input WAV", &["wav"]), Message::InputPicked)
            }
            Message::PickIr => {
                Command::perform(pick_file("Cabinet IR", &["wav"]), Message::IrPicked)
            }
            Message::PickExternalWav => Command::perform(
                pick_file("External WAV", &["wav"]),
                Message::ExternalWavPicked,
            ),
            Message::PickReferenceWav => Command::perform(
                pick_file("Reference WAV", &["wav"]),
                Message::ReferenceWavPicked,
            ),
            Message::PickHostAdapter => Command::perform(
                pick_file("Host adapter", &["json"]),
                Message::HostAdapterPicked,
            ),
            Message::PickHostedPlugin => Command::perform(
                pick_file("Plugin file or bundle", &["clap", "vst3", "component"]),
                Message::HostedPluginPicked,
            ),
            Message::PickHostedPluginFolder => {
                Command::perform(pick_folder(), Message::HostedPluginPicked)
            }
            Message::RigPicked(path) => {
                if let Some(path) = path {
                    self.rig = display_path(&self.root, path);
                }
                Command::none()
            }
            Message::InputPicked(path) => {
                if let Some(path) = path {
                    self.input = display_path(&self.root, path);
                }
                Command::none()
            }
            Message::IrPicked(path) => {
                if let Some(path) = path {
                    self.ir = display_path(&self.root, path);
                }
                Command::none()
            }
            Message::ExternalWavPicked(path) => {
                if let Some(path) = path {
                    self.external_wav = display_path(&self.root, path);
                }
                Command::none()
            }
            Message::ReferenceWavPicked(path) => {
                if let Some(path) = path {
                    self.reference_wav = display_path(&self.root, path);
                }
                Command::none()
            }
            Message::HostAdapterPicked(path) => {
                if let Some(path) = path {
                    self.host_adapter = display_path(&self.root, path);
                }
                Command::none()
            }
            Message::HostedPluginPicked(path) => {
                if let Some(path) = path {
                    self.hosted_plugin = display_path(&self.root, path);
                }
                Command::none()
            }
            Message::RunExperiment => {
                if self.running {
                    return Command::none();
                }
                self.running = true;
                self.status = "Running bounded render…".to_string();
                Command::perform(
                    run_experiment(
                        self.root.clone(),
                        self.rig.clone(),
                        self.input.clone(),
                        self.ir.clone(),
                        self.seconds.clone(),
                    ),
                    Message::ExperimentFinished,
                )
            }
            Message::ExperimentFinished(Ok(run)) => {
                self.running = false;
                self.status = format!("{} — {}", run.run_id, run.status.to_uppercase());
                self.selected = Some(run.clone());
                self.comparison = None;
                self.full_report = None;
                let waveform = LabCore::new(&self.root)
                    .waveform_preview(&run.artifacts.candidate_wav, 96)
                    .ok();
                self.waveform = waveform;
                self.spectrum = LabCore::new(&self.root)
                    .spectrum_preview(&run.artifacts.candidate_wav, 72)
                    .ok();
                Command::perform(load_workspace(self.root.clone()), Message::WorkspaceLoaded)
            }
            Message::ExperimentFinished(Err(error)) => {
                self.running = false;
                self.status = error;
                Command::none()
            }
            Message::ImportExternalWav => {
                if self.running {
                    return Command::none();
                }
                self.running = true;
                self.status = "Importing external WAV…".to_string();
                Command::perform(
                    import_external_wav(self.root.clone(), self.external_wav.clone()),
                    Message::ExternalWavImported,
                )
            }
            Message::ExternalWavImported(Ok(run)) => {
                self.running = false;
                self.status = format!("{} — external WAV analysed", run.run_id);
                self.selected = Some(run.clone());
                self.comparison = None;
                self.full_report = None;
                self.waveform = LabCore::new(&self.root)
                    .waveform_preview(&run.artifacts.candidate_wav, 96)
                    .ok();
                self.spectrum = LabCore::new(&self.root)
                    .spectrum_preview(&run.artifacts.candidate_wav, 72)
                    .ok();
                Command::perform(load_workspace(self.root.clone()), Message::WorkspaceLoaded)
            }
            Message::ExternalWavImported(Err(error)) => {
                self.running = false;
                self.status = error;
                Command::none()
            }
            Message::CompareSelected => {
                let Some(selected) = &self.selected else {
                    self.status = "Select a run before comparing it.".to_string();
                    return Command::none();
                };
                if self.running || self.reference_wav.trim().is_empty() {
                    return Command::none();
                }
                self.running = true;
                self.status = "Comparing selected WAV against reference…".to_string();
                Command::perform(
                    compare_selected(
                        self.root.clone(),
                        selected.artifacts.candidate_wav.clone(),
                        self.reference_wav.clone(),
                    ),
                    Message::ComparisonFinished,
                )
            }
            Message::ComparisonFinished(Ok(comparison)) => {
                self.running = false;
                self.status = format!(
                    "A/B preview ready — {} samples latency, {:.1} dB null residual.",
                    comparison.latency_samples,
                    comparison.null_relative_db.unwrap_or(f64::NAN)
                );
                self.comparison = Some(comparison);
                Command::none()
            }
            Message::ComparisonFinished(Err(error)) => {
                self.running = false;
                self.status = error;
                Command::none()
            }
            Message::GenerateFullReport => {
                let Some(selected) = &self.selected else {
                    self.status = "Select a run before generating a report.".to_string();
                    return Command::none();
                };
                if self.running || self.reference_wav.trim().is_empty() {
                    return Command::none();
                }
                self.running = true;
                self.status = "Generating persistent A/B report…".to_string();
                Command::perform(
                    generate_full_report(
                        self.root.clone(),
                        selected.artifacts.candidate_wav.clone(),
                        selected.artifacts.metadata.clone(),
                        self.reference_wav.clone(),
                        selected.run_id.clone(),
                    ),
                    Message::FullReportFinished,
                )
            }
            Message::FullReportFinished(Ok(report)) => {
                self.running = false;
                self.status = format!(
                    "Full A/B report {} — {}",
                    report.report.status.to_uppercase(),
                    report.report.report
                );
                self.full_report = Some(report);
                Command::none()
            }
            Message::FullReportFinished(Err(error)) => {
                self.running = false;
                self.status = error;
                Command::none()
            }
            Message::RunHostedPlugin => {
                if self.running {
                    return Command::none();
                }
                self.running = true;
                self.status = "Rendering hosted plugin through adapter…".to_string();
                Command::perform(
                    run_hosted_plugin(
                        self.root.clone(),
                        self.host_adapter.clone(),
                        self.hosted_plugin.clone(),
                        self.input.clone(),
                        self.seconds.clone(),
                    ),
                    Message::HostedPluginFinished,
                )
            }
            Message::HostedPluginFinished(Ok(run)) => {
                self.running = false;
                self.status = format!(
                    "{} — hosted plugin {}",
                    run.run_id,
                    run.status.to_uppercase()
                );
                self.selected = Some(run.clone());
                self.comparison = None;
                self.full_report = None;
                self.waveform = LabCore::new(&self.root)
                    .waveform_preview(&run.artifacts.candidate_wav, 96)
                    .ok();
                self.spectrum = LabCore::new(&self.root)
                    .spectrum_preview(&run.artifacts.candidate_wav, 72)
                    .ok();
                Command::perform(load_workspace(self.root.clone()), Message::WorkspaceLoaded)
            }
            Message::HostedPluginFinished(Err(error)) => {
                self.running = false;
                self.status = error;
                Command::none()
            }
            Message::SelectRun(run_id) => {
                self.selected = self
                    .snapshot
                    .runs
                    .iter()
                    .find(|run| run.run_id == run_id)
                    .cloned();
                self.waveform = self.selected.as_ref().and_then(|run| {
                    LabCore::new(&self.root)
                        .waveform_preview(&run.artifacts.candidate_wav, 96)
                        .ok()
                });
                self.spectrum = self.selected.as_ref().and_then(|run| {
                    LabCore::new(&self.root)
                        .spectrum_preview(&run.artifacts.candidate_wav, 72)
                        .ok()
                });
                self.comparison = None;
                self.full_report = None;
                Command::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let form = column![
            text("NEW REPRODUCIBLE EXPERIMENT").size(16),
            labeled_input("Rig", &self.rig, Message::RigChanged),
            button("Choose rig…").on_press(Message::PickRig),
            labeled_input("Input WAV", &self.input, Message::InputChanged),
            button("Choose input WAV…").on_press(Message::PickInput),
            labeled_input("Cab IR", &self.ir, Message::IrChanged),
            button("Choose cabinet IR…").on_press(Message::PickIr),
            labeled_input("Seconds", &self.seconds, Message::SecondsChanged),
            button(if self.running {
                "Running…"
            } else {
                "Run bounded render"
            })
            .on_press_maybe((!self.running).then_some(Message::RunExperiment))
            .padding(12),
            Space::with_height(Length::Fixed(8.0)),
            text("EXTERNAL PLUGIN / HARDWARE WAV").size(16),
            labeled_input(
                "Output WAV",
                &self.external_wav,
                Message::ExternalWavChanged
            ),
            button("Choose external WAV…").on_press(Message::PickExternalWav),
            button("Analyse external WAV")
                .on_press_maybe(
                    (!self.running && !self.external_wav.trim().is_empty())
                        .then_some(Message::ImportExternalWav)
                )
                .padding(12),
            Space::with_height(Length::Fixed(8.0)),
            text("A/B REFERENCE").size(16),
            labeled_input(
                "Reference WAV",
                &self.reference_wav,
                Message::ReferenceWavChanged
            ),
            button("Choose reference WAV…").on_press(Message::PickReferenceWav),
            button(if self.running {
                "Comparing…"
            } else {
                "Compare selected run"
            })
            .on_press_maybe(
                (!self.running && self.selected.is_some() && !self.reference_wav.trim().is_empty())
                    .then_some(Message::CompareSelected)
            )
            .padding(12),
            button(if self.running {
                "Generating report…"
            } else {
                "Generate full A/B report"
            })
            .on_press_maybe(
                (!self.running && self.selected.is_some() && !self.reference_wav.trim().is_empty())
                    .then_some(Message::GenerateFullReport)
            )
            .padding(12),
            Space::with_height(Length::Fixed(8.0)),
            text("HOSTED AU / VST3 / CLAP").size(16),
            labeled_input("Adapter", &self.host_adapter, Message::HostAdapterChanged),
            button("Choose adapter…").on_press(Message::PickHostAdapter),
            labeled_input("Plugin", &self.hosted_plugin, Message::HostedPluginChanged),
            button("Choose plugin…").on_press(Message::PickHostedPlugin),
            button("Choose plugin bundle folder…").on_press(Message::PickHostedPluginFolder),
            text("Uses the Input WAV and Seconds fields above.").size(12),
            button(if self.running {
                "Rendering hosted plugin…"
            } else {
                "Run hosted plugin"
            })
            .on_press_maybe(
                (!self.running
                    && !self.host_adapter.trim().is_empty()
                    && !self.hosted_plugin.trim().is_empty()
                    && !self.input.trim().is_empty())
                .then_some(Message::RunHostedPlugin)
            )
            .padding(12),
        ]
        .spacing(10);

        let run_list = self.snapshot.runs.iter().take(24).fold(
            column![text("RUN HISTORY").size(16)].spacing(7),
            |list, run| {
                let label = format!(
                    "{}  ·  {}  ·  {:.1}s",
                    run.run_id,
                    run.status.to_uppercase(),
                    run.runtime.render_seconds
                );
                list.push(
                    button(text(label).size(13))
                        .on_press(Message::SelectRun(run.run_id.clone()))
                        .width(Length::Fill)
                        .padding(8),
                )
            },
        );

        let detail = if let Some(run) = &self.selected {
            let diagnostics = &run.diagnostics;
            let mut detail = column![
                text("ANALYSIS RESULT").size(16),
                text(format!("{}  —  {}", run.run_id, run.status.to_uppercase())).size(22),
                metric_row(
                    "Output RMS",
                    diagnostics
                        .output_rms_dbfs
                        .map(|value| format!("{value:.1} dBFS"))
                        .unwrap_or_else(|| "—".to_string())
                ),
                metric_row(
                    "Output peak",
                    diagnostics
                        .output_peak_dbfs
                        .map(|value| format!("{value:.1} dBFS"))
                        .unwrap_or_else(|| "—".to_string())
                ),
                metric_row("XRuns", diagnostics.xrun_count.to_string()),
                metric_row("Hard clips", diagnostics.hard_clip_count.to_string()),
                metric_row(
                    "Audio",
                    format!(
                        "{} Hz · {} ch · {:.2}s",
                        diagnostics.wav_sample_rate.unwrap_or(0),
                        diagnostics.wav_channels.unwrap_or(0),
                        diagnostics.wav_duration_seconds.unwrap_or(0.0)
                    )
                ),
                text("WAVEFORM PREVIEW").size(14),
                container(
                    text(
                        self.waveform
                            .as_ref()
                            .map(waveform_bars)
                            .unwrap_or_else(|| "No WAV preview available".to_string())
                    )
                    .size(20)
                )
                .padding(12)
                .width(Length::Fill),
                text("SPECTRUM PREVIEW").size(14),
                container(
                    text(
                        self.spectrum
                            .as_ref()
                            .map(spectrum_bars)
                            .unwrap_or_else(|| "No spectrum preview available".to_string())
                    )
                    .size(20)
                )
                .padding(12)
                .width(Length::Fill),
                text(if diagnostics.warnings.is_empty() {
                    "No health warnings.".to_string()
                } else {
                    diagnostics.warnings.join(" · ")
                })
                .size(13),
                text(format!("Artifact: {}", run.artifacts.candidate_wav)).size(12),
                text(format!("Metadata: {}", run.artifacts.metadata)).size(12),
            ];
            if let Some(comparison) = &self.comparison {
                detail = detail
                    .push(text("A/B PREVIEW").size(14))
                    .push(metric_row(
                        "Latency",
                        format!("{} samples", comparison.latency_samples),
                    ))
                    .push(metric_row(
                        "Gain match",
                        comparison
                            .gain_correction_db
                            .map(|value| format!("{value:+.2} dB"))
                            .unwrap_or_else(|| "—".to_string()),
                    ))
                    .push(metric_row(
                        "Null residual",
                        comparison
                            .null_relative_db
                            .map(|value| format!("{value:.1} dB"))
                            .unwrap_or_else(|| "—".to_string()),
                    ))
                    .push(metric_row(
                        "Aligned frames",
                        comparison.aligned_frames.to_string(),
                    ));
                if !comparison.warnings.is_empty() {
                    detail = detail.push(text(comparison.warnings.join(" · ")).size(13));
                }
            }
            if let Some(report) = &self.full_report {
                detail = detail
                    .push(text("FULL A/B REPORT").size(14))
                    .push(metric_row("Status", report.report.status.to_uppercase()))
                    .push(text(format!("Saved: {}", report.report.report)).size(12));
                if !report.excerpt.is_empty() {
                    detail = detail.push(
                        container(text(&report.excerpt).size(12))
                            .padding(10)
                            .width(Length::Fill),
                    );
                }
            }
            detail.spacing(10)
        } else {
            column![
                text("ANALYSIS RESULT").size(16),
                Space::with_height(Length::Fixed(24.0)),
                text("No run selected yet.").size(18)
            ]
        };

        let content = column![
            row![text("GREYBOUND LAB").size(28), Space::with_width(Length::Fill), button("Refresh").on_press(Message::Refresh)]
                .align_items(Alignment::Center),
            text("The same bounded experiment artifacts are available to people, CI, and MCP agents.").size(14),
            row![
                container(scrollable(form).height(Length::Fill)).width(Length::FillPortion(3)).padding(16),
                container(scrollable(run_list).height(Length::Fill)).width(Length::FillPortion(3)).padding(16),
                container(detail).width(Length::FillPortion(5)).padding(16),
            ].spacing(14).height(Length::Fill),
            text(&self.status).size(13),
        ].spacing(12).padding(18).height(Length::Fill);
        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

fn labeled_input<'a>(
    label: &'a str,
    value: &'a str,
    message: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    row![
        text(label).width(Length::Fixed(88.0)),
        text_input("relative or absolute path", value)
            .on_input(message)
            .padding(8)
            .width(Length::Fill)
    ]
    .align_items(Alignment::Center)
    .into()
}

fn metric_row(label: &str, value: String) -> Element<'_, Message> {
    row![
        text(label).width(Length::Fixed(110.0)),
        text(value).size(16)
    ]
    .into()
}

fn waveform_bars(preview: &WaveformPreview) -> String {
    const BARS: &[char] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    preview
        .max
        .iter()
        .zip(&preview.min)
        .map(|(max, min)| {
            let magnitude = max.abs().max(min.abs()).clamp(0.0, 1.0);
            BARS[(magnitude * (BARS.len() - 1) as f32).round() as usize]
        })
        .collect()
}

fn spectrum_bars(preview: &SpectrumPreview) -> String {
    const BARS: &[char] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    preview
        .magnitudes_db
        .iter()
        .map(|magnitude| {
            let normalized = ((magnitude + 96.0) / 96.0).clamp(0.0, 1.0);
            BARS[(normalized * (BARS.len() - 1) as f32).round() as usize]
        })
        .collect()
}

async fn load_workspace(root: PathBuf) -> Result<WorkspaceView, String> {
    let core = LabCore::new(&root);
    let snapshot = core
        .load_workspace()
        .map_err(|error| format!("Could not load lab workspace: {error:#}"))?;
    let selected = snapshot.runs.first().cloned();
    let waveform = selected
        .as_ref()
        .and_then(|run| core.waveform_preview(&run.artifacts.candidate_wav, 96).ok());
    let spectrum = selected
        .as_ref()
        .and_then(|run| core.spectrum_preview(&run.artifacts.candidate_wav, 72).ok());
    Ok(WorkspaceView {
        snapshot,
        selected,
        waveform,
        spectrum,
    })
}

async fn run_experiment(
    root: PathBuf,
    rig: String,
    input: String,
    ir: String,
    seconds: String,
) -> Result<RunArtifact, String> {
    run_experiment_sync(root, rig, input, ir, seconds)
        .map_err(|error| format!("Experiment failed: {error:#}"))
}

async fn import_external_wav(root: PathBuf, candidate_wav: String) -> Result<RunArtifact, String> {
    let candidate_wav = resolve(&root, &candidate_wav);
    if !candidate_wav.exists() {
        return Err(format!(
            "External WAV does not exist: {}",
            candidate_wav.display()
        ));
    }
    let run_id = format!("external-{}", now_unix_ms());
    LabCore::new(&root)
        .record_wav_analysis(WavAnalysisRequest {
            run_id: run_id.clone(),
            candidate_wav,
            metadata_path: root
                .join("lab/renders/mcp")
                .join(format!("{run_id}.run.json")),
            label: "External plugin or hardware WAV".to_string(),
            context: json!({ "initiator": "iced", "experiment_kind": "external_wav_analysis" }),
        })
        .map_err(|error| format!("Could not analyse external WAV: {error:#}"))
}

async fn compare_selected(
    root: PathBuf,
    candidate_wav: String,
    reference_wav: String,
) -> Result<WavComparison, String> {
    let reference_wav = resolve(&root, &reference_wav);
    if !reference_wav.exists() {
        return Err(format!(
            "Reference WAV does not exist: {}",
            reference_wav.display()
        ));
    }
    LabCore::new(&root)
        .compare_wavs(candidate_wav, reference_wav)
        .map_err(|error| format!("A/B comparison failed: {error:#}"))
}

async fn generate_full_report(
    root: PathBuf,
    candidate_wav: String,
    metadata: String,
    reference_wav: String,
    run_id: String,
) -> Result<FullReportView, String> {
    let report_path = root
        .join("lab/reports/mcp")
        .join(format!("{run_id}-comparison.md"));
    let report = LabCore::new(&root)
        .generate_comparison_report(ComparisonReportRequest {
            candidate: resolve(&root, candidate_wav),
            reference: resolve(&root, reference_wav),
            metadata: (!metadata.trim().is_empty()).then(|| resolve(&root, metadata)),
            segments: None,
            report: report_path,
        })
        .map_err(|error| format!("Could not generate A/B report: {error:#}"))?;
    let excerpt = std::fs::read_to_string(resolve(&root, &report.report))
        .map(|text| truncate_report(&text, 1_800))
        .unwrap_or_else(|_| report.stderr.clone());
    Ok(FullReportView { report, excerpt })
}

async fn run_hosted_plugin(
    root: PathBuf,
    adapter: String,
    plugin: String,
    input_wav: String,
    seconds: String,
) -> Result<RunArtifact, String> {
    let seconds = seconds
        .parse::<f64>()
        .map_err(|_| "seconds must be a number".to_string())?;
    if !(0.01..=600.0).contains(&seconds) {
        return Err("seconds must be between 0.01 and 600".to_string());
    }
    let run_id = format!("plugin-{}", now_unix_ms());
    let render_dir = root.join("lab/renders/mcp");
    LabCore::new(&root)
        .run_plugin_host(PluginRenderRequest {
            run_id: run_id.clone(),
            adapter_path: resolve(&root, adapter),
            plugin_path: resolve(&root, plugin),
            input_wav: resolve(&root, input_wav),
            output_wav: render_dir.join(format!("{run_id}.wav")),
            metadata_path: render_dir.join(format!("{run_id}.run.json")),
            sample_rate: 48_000,
            block_size: 64,
            render_seconds: seconds,
            context: json!({ "initiator": "iced", "experiment_kind": "external_plugin_render" }),
        })
        .map_err(|error| format!("Hosted plugin render failed: {error:#}"))
}

async fn pick_file(label: &'static str, extensions: &'static [&'static str]) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter(label, extensions)
        .pick_file()
}

async fn pick_folder() -> Option<PathBuf> {
    rfd::FileDialog::new().pick_folder()
}

fn truncate_report(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        text.to_string()
    } else {
        format!("{}\n…", text.chars().take(limit).collect::<String>())
    }
}

fn run_experiment_sync(
    root: PathBuf,
    rig: String,
    input: String,
    ir: String,
    seconds: String,
) -> Result<RunArtifact> {
    for required in [&rig, &input, &ir] {
        let path = resolve(&root, required);
        if !path.exists() {
            bail!("required artifact is missing: {}", path.display());
        }
    }
    let seconds = seconds.parse::<f64>().context("seconds must be a number")?;
    if !(0.01..=120.0).contains(&seconds) {
        bail!("seconds must be between 0.01 and 120");
    }
    let cli = ensure_release_cli(&root)?;
    let run_id = format!("lab-{}", now_unix_ms());
    let render_dir = root.join("lab/renders/mcp");
    let output_wav = render_dir.join(format!("{run_id}.wav"));
    let monitor_log = render_dir.join(format!("{run_id}.monitor.log"));
    let metadata_path = render_dir.join(format!("{run_id}.run.json"));
    let core = LabCore::new(&root);
    core.run_render(RenderRequest {
        run_id,
        cli,
        rig: rig.clone(),
        effective_rig: resolve(&root, rig),
        runtime: RenderRuntime {
            sample_rate: 48_000,
            period_size: 32,
            render_seconds: seconds,
            input_wav: input,
            input_channel: 0,
            ir,
            output_wav: output_wav
                .strip_prefix(&root)
                .unwrap_or(&output_wav)
                .display()
                .to_string(),
            monitor: true,
        },
        monitor_log,
        metadata_path,
        git_revision: git_revision(&root),
        context: json!({ "initiator": "iced", "experiment_kind": "greybound_rig_render" }),
    })
}

fn ensure_release_cli(root: &Path) -> Result<PathBuf> {
    let cli = root.join("target/release/greybound-cli");
    if cli.exists() {
        return Ok(cli);
    }
    let status = ProcessCommand::new("cargo")
        .current_dir(root)
        .args(["build", "--release", "-p", "greybound-cli"])
        .status()?;
    if !status.success() {
        bail!("could not build greybound-cli");
    }
    Ok(cli)
}

fn workspace_root() -> Result<PathBuf> {
    if let Some(root) = std::env::var_os("GREYBOUND_LAB_HOME") {
        let root = PathBuf::from(root);
        std::fs::create_dir_all(&root).with_context(|| {
            format!(
                "could not create GREYBOUND_LAB_HOME workspace '{}'",
                root.display()
            )
        })?;
        return Ok(root);
    }
    let mut directory = std::env::current_dir()?;
    loop {
        if directory.join("Cargo.toml").exists() && directory.join("knowledge").exists() {
            return Ok(directory);
        }
        if !directory.pop() {
            return std::env::current_dir().context("could not determine Lab workspace root");
        }
    }
}

fn resolve(root: &Path, value: impl AsRef<Path>) -> PathBuf {
    let value = value.as_ref();
    if value.is_absolute() {
        value.to_path_buf()
    } else {
        root.join(value)
    }
}

fn display_path(root: &Path, path: PathBuf) -> String {
    path.strip_prefix(root)
        .unwrap_or(&path)
        .display()
        .to_string()
}

fn git_revision(root: &Path) -> Option<String> {
    let output = ProcessCommand::new("git")
        .current_dir(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
