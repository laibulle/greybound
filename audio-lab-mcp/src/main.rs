use anyhow::{anyhow, bail, Context, Result};
use greybound::rig::RigDeviceSlot;
use greybound::{amp_model_descriptor, ControlKind, DeviceConfig, RigConfig, SignalChainConfig};
use greybound_lab_core::{
    ComparisonReportRequest, LabCore, PluginRenderRequest, RenderRequest, RenderRuntime,
    WavAnalysisRequest,
};
use serde_json::{json, Map, Value};
use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const PROTOCOL_VERSION: &str = "2025-06-18";
const DEFAULT_INPUT_WAV: &str = "lab/references/tone3000-inputs/Brit - Guitar.wav";
const DEFAULT_IR: &str = "lab/references/tone3000-irs/celestion.wav";
const DEFAULT_RIG: &str = "rigs/grey-nox.json5";

fn main() {
    if let Err(error) = run() {
        eprintln!("greybound-audio-lab-mcp fatal: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let root = if let Some(root) = env::var_os("GREYBOUND_LAB_HOME") {
        let root = PathBuf::from(root);
        fs::create_dir_all(&root)?;
        root
    } else {
        env::current_dir()?
    };
    let server = Server::new(root);
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<JsonRpcRequest>(line) {
            Ok(request) => server.handle(request),
            Err(error) => Some(error_response(
                Value::Null,
                -32700,
                format!("invalid JSON-RPC message: {error}"),
            )),
        };

        if let Some(response) = response {
            serde_json::to_writer(&mut stdout, &response)?;
            stdout.write_all(b"\n")?;
            stdout.flush()?;
        }
    }

    Ok(())
}

struct Server {
    root: PathBuf,
}

impl Server {
    fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn handle(&self, request: JsonRpcRequest) -> Option<Value> {
        if request.jsonrpc.as_deref() != Some("2.0") {
            return request
                .id
                .map(|id| error_response(id, -32600, "expected JSON-RPC 2.0 request".to_string()));
        }

        let id = request.id.clone();
        let result = match request.method.as_str() {
            "initialize" => Ok(json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": { "tools": {} },
                "serverInfo": {
                    "name": "greybound-audio-lab-mcp",
                    "title": "Greybound Audio Lab MCP",
                    "version": env!("CARGO_PKG_VERSION")
                }
            })),
            "notifications/initialized" => return None,
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({ "tools": tools() })),
            "tools/call" => self.handle_tool_call(request.params.unwrap_or_default()),
            _ => Err(anyhow!("unknown method '{}'", request.method)),
        };

        id.map(|id| match result {
            Ok(result) => success_response(id, result),
            Err(error) => error_response(id, -32000, format!("{error:#}")),
        })
    }

    fn handle_tool_call(&self, params: Value) -> Result<Value> {
        let name = params
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("tools/call requires params.name"))?;
        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or(Value::Object(Map::new()));

        let structured = match name {
            "list_audio_assets" => self.list_audio_assets(&arguments)?,
            "list_runs" => self.list_runs()?,
            "import_wav" => self.import_wav(&arguments)?,
            "preview_wav" => self.preview_wav(&arguments)?,
            "compare_preview" => self.compare_preview(&arguments)?,
            "list_host_adapters" => self.list_host_adapters()?,
            "render_plugin" => self.render_plugin(&arguments)?,
            "inspect_rig" => self.inspect_rig(&arguments)?,
            "render_rig" => self.render_rig(&arguments)?,
            "monitor_render" => self.monitor_render(&arguments)?,
            "compare_wav" => self.compare_wav(&arguments)?,
            "evaluate_wav" => self.evaluate_wav(&arguments)?,
            "sweep_controls" => self.sweep_controls(&arguments)?,
            "summarize_run" => self.summarize_run(&arguments)?,
            _ => bail!("unknown tool '{name}'"),
        };

        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&structured)?
            }],
            "structuredContent": structured
        }))
    }

    fn list_audio_assets(&self, arguments: &Value) -> Result<Value> {
        let kind = arguments
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("all");
        let mut result = Map::new();

        if kind == "all" || kind == "rigs" {
            result.insert("rigs".to_string(), self.list_paths("rigs", &["json5"])?);
        }
        if kind == "all" || kind == "inputs" {
            result.insert(
                "inputs".to_string(),
                self.list_paths("lab/references/tone3000-inputs", &["wav"])?,
            );
        }
        if kind == "all" || kind == "irs" {
            result.insert(
                "irs".to_string(),
                self.list_paths("lab/references/tone3000-irs", &["wav"])?,
            );
        }
        if kind == "all" || kind == "renders" {
            result.insert(
                "renders".to_string(),
                self.list_paths("lab/renders", &["wav"])?,
            );
        }
        if kind == "all" || kind == "reports" {
            result.insert(
                "reports".to_string(),
                self.list_paths("lab/reports", &["md"])?,
            );
        }

        Ok(Value::Object(result))
    }

    fn list_runs(&self) -> Result<Value> {
        Ok(serde_json::to_value(
            LabCore::new(&self.root).load_workspace()?,
        )?)
    }

    fn import_wav(&self, arguments: &Value) -> Result<Value> {
        let candidate_wav = required_path(arguments, "candidate_wav")?;
        let run_id = arguments
            .get("run_id")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("external-{}", run_id_now()));
        let metadata_path = arguments
            .get("metadata")
            .and_then(Value::as_str)
            .map(|path| self.resolve(path))
            .unwrap_or_else(|| {
                self.root
                    .join("lab/renders/mcp")
                    .join(format!("{run_id}.run.json"))
            });
        let label = arguments
            .get("label")
            .and_then(Value::as_str)
            .unwrap_or("External WAV")
            .to_string();
        let artifact = LabCore::new(&self.root).record_wav_analysis(WavAnalysisRequest {
            run_id,
            candidate_wav: self.resolve(candidate_wav),
            metadata_path,
            label,
            context: json!({ "initiator": "mcp", "experiment_kind": "external_wav_analysis" }),
        })?;
        Ok(serde_json::to_value(artifact)?)
    }

    fn preview_wav(&self, arguments: &Value) -> Result<Value> {
        let candidate_wav = required_path(arguments, "candidate_wav")?;
        let bins = arguments.get("bins").and_then(Value::as_u64).unwrap_or(96) as usize;
        let core = LabCore::new(&self.root);
        Ok(json!({
            "diagnostics": core.analyse_wav(&candidate_wav)?,
            "waveform": core.waveform_preview(&candidate_wav, bins)?,
            "spectrum": core.spectrum_preview(&candidate_wav, bins)?,
        }))
    }

    fn compare_preview(&self, arguments: &Value) -> Result<Value> {
        let candidate = required_path(arguments, "candidate")?;
        let reference = required_path(arguments, "reference")?;
        Ok(serde_json::to_value(
            LabCore::new(&self.root).compare_wavs(candidate, reference)?,
        )?)
    }

    fn list_host_adapters(&self) -> Result<Value> {
        let adapters = LabCore::new(&self.root)
            .list_host_adapters()?
            .into_iter()
            .map(|(path, manifest)| json!({ "path": path, "manifest": manifest }))
            .collect::<Vec<_>>();
        Ok(json!({ "adapters": adapters }))
    }

    fn render_plugin(&self, arguments: &Value) -> Result<Value> {
        let adapter = required_path(arguments, "adapter")?;
        let plugin = required_path(arguments, "plugin")?;
        let input_wav = required_path(arguments, "input_wav")?;
        let run_id = arguments
            .get("run_id")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("plugin-{}", run_id_now()));
        let runtime = arguments.get("runtime").cloned().unwrap_or_default();
        let output_wav = runtime_path(
            &runtime,
            "output_wav",
            self.root
                .join("lab/renders/mcp")
                .join(format!("{run_id}.wav")),
        );
        let metadata_path = runtime_path(
            &runtime,
            "metadata",
            self.root
                .join("lab/renders/mcp")
                .join(format!("{run_id}.run.json")),
        );
        let artifact = LabCore::new(&self.root).run_plugin_host(PluginRenderRequest {
            run_id,
            adapter_path: self.resolve(&adapter),
            plugin_path: self.resolve(&plugin),
            input_wav: self.resolve(&input_wav),
            output_wav: self.resolve(output_wav),
            metadata_path: self.resolve(metadata_path),
            sample_rate: runtime_u64(&runtime, "sample_rate", 48_000),
            block_size: runtime_u64(&runtime, "block_size", 64),
            render_seconds: runtime_f64(&runtime, "render_seconds", 10.0),
            context: json!({
                "initiator": "mcp",
                "experiment_kind": "external_plugin_render",
                "adapter": adapter,
                "plugin": plugin,
            }),
        })?;
        Ok(serde_json::to_value(artifact)?)
    }

    fn inspect_rig(&self, arguments: &Value) -> Result<Value> {
        let rig_path = arguments
            .get("rig")
            .and_then(Value::as_str)
            .unwrap_or(DEFAULT_RIG);
        let rig = self.read_rig(rig_path)?;
        let chain = rig.signal_chain_config()?;
        let amp_descriptor = amp_model_descriptor(&rig.amp.model);

        Ok(json!({
            "rig": rig_path,
            "name": rig.name,
            "amp": {
                "model": rig.amp.model,
                "label": amp_descriptor.label,
                "bypassed": rig.amp.bypassed,
                "controls": {
                    "volume": rig.amp.controls.volume,
                    "bass": rig.amp.controls.bass,
                    "treble": rig.amp.controls.treble,
                    "cut": rig.amp.controls.cut,
                    "drive": rig.amp.controls.drive,
                    "presence": rig.amp.controls.presence,
                    "sag": rig.amp.controls.sag
                }
            },
            "chain": chain_summary(&chain),
            "pre_amp": slots_summary(&rig.pre_amp)?,
            "fx_loop": slots_summary(&rig.fx_loop)?,
            "post_amp": slots_summary(&rig.post_amp)?,
            "cab": rig.cab.as_ref().map(|cab| json!({
                "ir": cab.ir,
                "bypassed": cab.bypassed,
                "enabled": !cab.bypassed
            }))
        }))
    }

    fn render_rig(&self, arguments: &Value) -> Result<Value> {
        let rig = arguments
            .get("rig")
            .and_then(Value::as_str)
            .unwrap_or(DEFAULT_RIG);
        let runtime = arguments.get("runtime").cloned().unwrap_or_default();
        let build = arguments
            .get("build")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let run_id = arguments
            .get("run_id")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| run_id_from(rig));
        let render_dir = self.root.join("lab/renders/mcp");
        let scratch_dir = self.root.join("target/greybound-mcp").join(&run_id);
        fs::create_dir_all(&render_dir)?;
        fs::create_dir_all(&scratch_dir)?;

        let mut rig_arg = self.resolve_existing(rig)?;
        let overrides = arguments.get("overrides").cloned().unwrap_or_default();
        if !overrides.is_null() && overrides != json!({}) {
            rig_arg = self.write_override_rig(&rig_arg, &overrides, &scratch_dir)?;
        }

        let output_wav = runtime_path(
            &runtime,
            "output_wav",
            render_dir.join(format!("{run_id}.wav")),
        );
        let monitor_log = self.resolve(runtime_path(
            &runtime,
            "monitor_log",
            render_dir.join(format!("{run_id}.monitor.log")),
        ));
        let metadata_path = render_dir.join(format!("{run_id}.run.json"));
        let sample_rate = runtime_u64(&runtime, "sample_rate", 48_000);
        let period_size = runtime_u64(&runtime, "period_size", 32);
        let render_seconds = runtime_f64(&runtime, "render_seconds", 10.0);
        let input_wav = runtime_path(&runtime, "input_wav", self.root.join(DEFAULT_INPUT_WAV));
        let input_channel = runtime_u64(&runtime, "input_channel", 0);
        let ir = runtime_path(&runtime, "ir", self.root.join(DEFAULT_IR));
        let monitor = runtime
            .get("monitor")
            .and_then(Value::as_bool)
            .unwrap_or(true);

        if build {
            self.ensure_release_cli()?;
        }

        let git_revision = self.git_revision().ok();
        let artifact = LabCore::new(&self.root).run_render(RenderRequest {
            run_id,
            cli: self.root.join("target/release/greybound-cli"),
            rig: rig.to_string(),
            effective_rig: rig_arg,
            runtime: RenderRuntime {
                sample_rate,
                period_size,
                render_seconds,
                input_wav: path_for_json(&input_wav),
                input_channel,
                ir: path_for_json(&ir),
                output_wav: path_for_json(&output_wav),
                monitor,
            },
            monitor_log,
            metadata_path,
            git_revision,
            context: json!({ "initiator": "mcp", "overrides": overrides }),
        })?;
        let diagnostics = serde_json::to_value(&artifact.diagnostics)?;

        Ok(json!({
            "run_id": artifact.run_id,
            "status": artifact.status,
            "candidate_wav": artifact.artifacts.candidate_wav,
            "monitor_log": artifact.artifacts.monitor_log,
            "metadata": artifact.artifacts.metadata,
            "report": null,
            "diagnostics": diagnostics,
            "command": artifact.command,
            "stdout": artifact.stdout,
            "stderr": artifact.stderr
        }))
    }

    fn monitor_render(&self, arguments: &Value) -> Result<Value> {
        let monitor_log = arguments
            .get("monitor_log")
            .and_then(Value::as_str)
            .map(|path| self.resolve(path))
            .ok_or_else(|| anyhow!("monitor_render requires monitor_log"))?;
        let candidate_wav = arguments
            .get("candidate_wav")
            .and_then(Value::as_str)
            .map(|path| self.resolve(path));
        self.monitor_render_paths(
            &monitor_log,
            candidate_wav.as_deref().unwrap_or(Path::new("")),
        )
    }

    fn compare_wav(&self, arguments: &Value) -> Result<Value> {
        let candidate = required_path(arguments, "candidate")?;
        let reference = required_path(arguments, "reference")?;
        let report = arguments
            .get("report")
            .and_then(Value::as_str)
            .map(|path| self.resolve(path))
            .unwrap_or_else(|| {
                self.root
                    .join("lab/reports/mcp")
                    .join(format!("{}-comparison.md", run_id_now()))
            });
        let comparison =
            LabCore::new(&self.root).generate_comparison_report(ComparisonReportRequest {
                candidate: self.resolve(candidate),
                reference: self.resolve(reference),
                metadata: arguments
                    .get("metadata")
                    .and_then(Value::as_str)
                    .map(|path| self.resolve(path)),
                segments: arguments
                    .get("segments")
                    .and_then(Value::as_str)
                    .map(|path| self.resolve(path)),
                report,
            })?;
        Ok(serde_json::to_value(comparison)?)
    }

    fn evaluate_wav(&self, arguments: &Value) -> Result<Value> {
        let candidate = required_path(arguments, "candidate_wav")?;
        let monitor_log = arguments.get("monitor_log").and_then(Value::as_str);
        let diagnostics = if let Some(monitor_log) = monitor_log {
            self.monitor_render_paths(&self.resolve(monitor_log), &self.resolve(&candidate))?
        } else {
            serde_json::to_value(LabCore::new(&self.root).analyse_wav(&candidate)?)?
        };
        Ok(json!({
            "status": diagnostics.get("status").and_then(Value::as_str).unwrap_or("warning"),
            "profile": arguments.get("profile").and_then(Value::as_str).unwrap_or("regression"),
            "diagnostics": diagnostics,
            "note": "evaluate_wav currently reports runtime and WAV health; use compare_wav when a reference is available."
        }))
    }

    fn sweep_controls(&self, _arguments: &Value) -> Result<Value> {
        Ok(json!({
            "status": "warning",
            "note": "sweep_controls is reserved for batch renders; render_rig is implemented for single bounded runs."
        }))
    }

    fn summarize_run(&self, arguments: &Value) -> Result<Value> {
        let metadata = arguments
            .get("metadata")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("summarize_run requires metadata"))?;
        let text = fs::read_to_string(self.resolve(metadata))?;
        let value: Value = serde_json::from_str(&text)?;
        Ok(json!({
            "status": value.get("status").cloned().unwrap_or(json!("warning")),
            "run_id": value.get("run_id").cloned(),
            "runtime": value.get("runtime").cloned(),
            "diagnostics": value.get("diagnostics").cloned(),
            "command": value.get("command").cloned()
        }))
    }

    fn monitor_render_paths(&self, monitor_log: &Path, candidate_wav: &Path) -> Result<Value> {
        Ok(serde_json::to_value(
            LabCore::new(&self.root).analyse_render(monitor_log, candidate_wav)?,
        )?)
    }

    fn list_paths(&self, dir: &str, extensions: &[&str]) -> Result<Value> {
        let dir = self.root.join(dir);
        if !dir.exists() {
            return Ok(json!([]));
        }
        let mut entries = Vec::new();
        collect_paths(&dir, extensions, &mut entries)?;
        entries.sort();
        Ok(json!(entries
            .into_iter()
            .take(200)
            .map(|path| {
                let absolute = path.clone();
                json!({
                    "path": path_for_json(&absolute),
                    "label": path.file_name().and_then(|name| name.to_str()).unwrap_or_default(),
                    "kind": path.extension().and_then(|ext| ext.to_str()).unwrap_or_default()
                })
            })
            .collect::<Vec<_>>()))
    }

    fn read_rig(&self, path: &str) -> Result<RigConfig> {
        let path = self.resolve_existing(path)?;
        let text = fs::read_to_string(&path)
            .with_context(|| format!("could not read rig '{}'", path.display()))?;
        RigConfig::from_json5(&text)
            .with_context(|| format!("could not parse rig JSON5 '{}'", path.display()))
    }

    fn write_override_rig(
        &self,
        rig_path: &Path,
        overrides: &Value,
        scratch_dir: &Path,
    ) -> Result<PathBuf> {
        let text = fs::read_to_string(rig_path)?;
        let mut value: Value = json5::from_str(&text)?;
        apply_overrides(&mut value, overrides)?;
        let target = scratch_dir.join("rig.override.json");
        fs::write(&target, serde_json::to_vec_pretty(&value)?)?;
        Ok(target)
    }

    fn ensure_release_cli(&self) -> Result<()> {
        let cli = self.root.join("target/release/greybound-cli");
        if cli.exists() {
            return Ok(());
        }
        let status = Command::new("cargo")
            .current_dir(&self.root)
            .arg("build")
            .arg("--release")
            .arg("-p")
            .arg("greybound-cli")
            .status()?;
        if !status.success() {
            bail!("cargo build --release -p greybound-cli failed");
        }
        Ok(())
    }

    fn git_revision(&self) -> Result<String> {
        let output = Command::new("git")
            .current_dir(&self.root)
            .arg("rev-parse")
            .arg("HEAD")
            .output()?;
        if !output.status.success() {
            bail!("git rev-parse failed");
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn resolve(&self, path: impl AsRef<Path>) -> PathBuf {
        let path = path.as_ref();
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        }
    }

    fn resolve_existing(&self, path: &str) -> Result<PathBuf> {
        let path = self.resolve(path);
        if !path.exists() {
            bail!("path does not exist: {}", path.display());
        }
        Ok(path)
    }
}

#[derive(serde::Deserialize)]
struct JsonRpcRequest {
    jsonrpc: Option<String>,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

fn tools() -> Value {
    json!([
        tool(
            "list_audio_assets",
            "List known rigs, input WAVs, IRs, renders, and reports.",
            json!({
                "type": "object",
                "properties": { "kind": { "type": "string", "enum": ["all", "rigs", "inputs", "irs", "renders", "reports"], "default": "all" } }
            })
        ),
        tool(
            "list_runs",
            "List versioned experiment artifacts shared with Greybound Lab desktop.",
            json!({ "type": "object" })
        ),
        tool(
            "import_wav",
            "Create a versioned Lab artifact from a WAV rendered by any external plugin host or hardware workflow.",
            json!({
                "type": "object",
                "required": ["candidate_wav"],
                "properties": {
                    "candidate_wav": { "type": "string" },
                    "label": { "type": "string" },
                    "run_id": { "type": "string" },
                    "metadata": { "type": "string" }
                }
            })
        ),
        tool(
            "preview_wav",
            "Return shared health, waveform, and spectrum preview data for a WAV artifact.",
            json!({
                "type": "object",
                "required": ["candidate_wav"],
                "properties": { "candidate_wav": { "type": "string" }, "bins": { "type": "integer", "default": 96 } }
            })
        ),
        tool(
            "compare_preview",
            "Fast aligned A/B diagnostic: latency, gain correction, and null residual. Use compare_wav for the full report.",
            json!({
                "type": "object",
                "required": ["candidate", "reference"],
                "properties": { "candidate": { "type": "string" }, "reference": { "type": "string" } }
            })
        ),
        tool(
            "list_host_adapters",
            "List valid declarative AU/VST3/CLAP/external host adapters installed under lab/adapters.",
            json!({ "type": "object" })
        ),
        tool(
            "render_plugin",
            "Run a plugin through a selected declarative host adapter. The adapter uses direct process arguments, never a shell command.",
            json!({
                "type": "object",
                "required": ["adapter", "plugin", "input_wav"],
                "properties": {
                    "adapter": { "type": "string", "description": "Path to a greybound.lab.host-adapter.v1 JSON file." },
                    "plugin": { "type": "string", "description": "AU/VST3/CLAP bundle or plugin file consumed by the adapter." },
                    "input_wav": { "type": "string" },
                    "run_id": { "type": "string" },
                    "runtime": {
                        "type": "object",
                        "properties": {
                            "output_wav": { "type": "string" },
                            "metadata": { "type": "string" },
                            "sample_rate": { "type": "integer", "default": 48000 },
                            "block_size": { "type": "integer", "default": 64 },
                            "render_seconds": { "type": "number", "default": 10.0 }
                        }
                    }
                }
            })
        ),
        tool(
            "inspect_rig",
            "Inspect a Greybound rig chain and its current controls.",
            json!({
                "type": "object",
                "properties": { "rig": { "type": "string", "default": DEFAULT_RIG } }
            })
        ),
        tool(
            "render_rig",
            "Render a rig from an input WAV through greybound-cli and return monitor diagnostics.",
            json!({
                "type": "object",
                "properties": {
                    "rig": { "type": "string", "default": DEFAULT_RIG },
                    "run_id": { "type": "string" },
                    "build": { "type": "boolean", "default": true },
                    "runtime": { "type": "object" },
                    "overrides": { "type": "object" }
                }
            })
        ),
        tool(
            "monitor_render",
            "Summarize monitor log and WAV health diagnostics.",
            json!({
                "type": "object",
                "required": ["monitor_log"],
                "properties": { "monitor_log": { "type": "string" }, "candidate_wav": { "type": "string" } }
            })
        ),
        tool(
            "compare_wav",
            "Run greybound-lab compare-wav against a reference.",
            json!({
                "type": "object",
                "required": ["candidate", "reference"],
                "properties": {
                    "candidate": { "type": "string" },
                    "reference": { "type": "string" },
                    "metadata": { "type": "string" },
                    "segments": { "type": "string" },
                    "report": { "type": "string" }
                }
            })
        ),
        tool(
            "evaluate_wav",
            "Evaluate candidate WAV health without a reference.",
            json!({
                "type": "object",
                "required": ["candidate_wav"],
                "properties": { "candidate_wav": { "type": "string" }, "monitor_log": { "type": "string" }, "profile": { "type": "string" } }
            })
        ),
        tool(
            "sweep_controls",
            "Reserved batch sweep entry point.",
            json!({ "type": "object" })
        ),
        tool(
            "summarize_run",
            "Summarize a previously written MCP run metadata file.",
            json!({
                "type": "object",
                "required": ["metadata"],
                "properties": { "metadata": { "type": "string" } }
            })
        )
    ])
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema
    })
}

fn success_response(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error_response(id: Value, code: i64, message: String) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn slots_summary(slots: &[RigDeviceSlot]) -> Result<Value> {
    Ok(json!(slots
        .iter()
        .enumerate()
        .map(|(index, slot)| {
            let config = parse_device_config(&slot.device)?;
            let descriptor = config.model_descriptor();
            Ok(json!({
                "index": index,
                "id": slot.id,
                "model": slot.device,
                "label": descriptor.label,
                "category": descriptor.category,
                "bypassed": slot.bypassed,
                "controls": controls_for_slot(slot),
                "control_descriptors": descriptor.controls.iter().map(|control| json!({
                    "id": control.id,
                    "label": control.label,
                    "kind": control_kind(control.kind),
                    "min": control.min,
                    "max": control.max,
                    "display_scale": control.display_scale,
                    "step": control.step,
                    "large_step": control.large_step
                })).collect::<Vec<_>>()
            }))
        })
        .collect::<Result<Vec<_>>>()?))
}

fn chain_summary(chain: &SignalChainConfig) -> Value {
    json!({
        "amp_model": chain.amp_model,
        "pre_amp_count": chain.pre_amp.len(),
        "fx_loop_count": chain.fx_loop.len(),
        "post_amp_count": chain.post_amp.len(),
        "cable_capacitance_farads": chain.cable_capacitance_farads
    })
}

fn controls_for_slot(slot: &RigDeviceSlot) -> Value {
    let controls = slot.controls;
    match slot.device.as_str() {
        "minotaur" => {
            json!({ "gain": controls.gain, "treble": controls.treble, "output": controls.output })
        }
        "springfield" => {
            json!({ "dwell": controls.dwell, "tone": controls.tone, "mix": controls.mix })
        }
        "muffin" => {
            json!({ "sustain": controls.sustain, "tone": controls.tone, "level": controls.level })
        }
        "lumen" => json!({
            "peak_reduction": controls.peak_reduction,
            "gain": controls.gain,
            "emphasis": controls.emphasis,
            "mix": controls.mix
        }),
        "muon" => json!({
            "sensitivity": controls.sensitivity,
            "range": controls.range,
            "resonance": controls.resonance,
            "mix": controls.mix
        }),
        "monarch" => {
            json!({ "gain": controls.gain, "tone": controls.tone, "output": controls.output })
        }
        "godess-one" => json!({
            "distortion": controls.distortion,
            "tone": controls.tone,
            "level": controls.level,
            "mode": format!("{:?}", controls.mode)
        }),
        "dartford" => json!({
            "rate_hz": controls.rate_hz,
            "depth": controls.depth,
            "level": controls.level,
            "wave": format!("{:?}", controls.wave)
        }),
        "tron" => json!({
            "rate_hz": controls.rate_hz,
            "depth": controls.depth,
            "feedback": controls.feedback,
            "mix": controls.mix
        }),
        "jetstream" => json!({
            "manual": controls.manual,
            "rate_hz": controls.rate_hz,
            "depth": controls.depth,
            "feedback": controls.feedback,
            "mix": controls.mix
        }),
        "celeste" => json!({
            "rate_hz": controls.rate_hz,
            "depth": controls.depth,
            "tone": controls.tone,
            "mix": controls.mix
        }),
        "brigade" => json!({
            "time_ms": controls.time_ms,
            "repeats": controls.repeats,
            "tone": controls.tone,
            "mix": controls.mix
        }),
        "studioverb" => json!({
            "algorithm": format!("{:?}", controls.algorithm),
            "decay": controls.decay,
            "size": controls.size,
            "pre_delay_ms": controls.pre_delay_ms,
            "diffusion": controls.diffusion,
            "tone": controls.tone,
            "low_cut": controls.low_cut,
            "mod_depth": controls.mod_depth,
            "mix": controls.mix
        }),
        _ => json!({
            "gain": controls.gain,
            "tone": controls.tone,
            "level": controls.level,
            "mix": controls.mix,
            "rate_hz": controls.rate_hz,
            "depth": controls.depth
        }),
    }
}

fn control_kind(kind: ControlKind) -> &'static str {
    match kind {
        ControlKind::Pot => "pot",
        ControlKind::Slider => "slider",
        ControlKind::Switch => "switch",
        ControlKind::Footswitch => "footswitch",
    }
}

fn parse_device_config(device: &str) -> Result<DeviceConfig> {
    match device {
        "lumen" => Ok(DeviceConfig::Lumen),
        "muon" => Ok(DeviceConfig::Muon),
        "muffin" => Ok(DeviceConfig::Muffin),
        "minotaur" => Ok(DeviceConfig::Minotaur),
        "monarch" => Ok(DeviceConfig::Monarch),
        "godess-one" => Ok(DeviceConfig::GodessOne),
        "dartford" => Ok(DeviceConfig::Dartford),
        "tron" => Ok(DeviceConfig::Tron),
        "jetstream" => Ok(DeviceConfig::Jetstream),
        "celeste" => Ok(DeviceConfig::Celeste),
        "brigade" => Ok(DeviceConfig::Brigade),
        "springfield" => Ok(DeviceConfig::Springfield),
        "studioverb" => Ok(DeviceConfig::StudioVerb),
        _ => bail!("unknown rig device '{device}'"),
    }
}

fn apply_overrides(rig: &mut Value, overrides: &Value) -> Result<()> {
    if let Some(amp) = overrides.get("amp") {
        let amp_controls = object_path_mut(rig, &["amp", "controls"])?;
        merge_object(amp_controls, amp)?;
    }
    if let Some(cab) = overrides.get("cab") {
        let cab_value = rig
            .get_mut("cab")
            .ok_or_else(|| anyhow!("cab override requested, but rig has no cab"))?;
        merge_object(cab_value, cab)?;
        if let Some(enabled) = cab.get("enabled").and_then(Value::as_bool) {
            cab_value["bypassed"] = json!(!enabled);
        }
    }
    if let Some(devices) = overrides.get("devices").and_then(Value::as_array) {
        for override_slot in devices {
            let section = override_slot
                .get("section")
                .and_then(Value::as_str)
                .unwrap_or("pre_amp");
            let index = override_slot
                .get("index")
                .and_then(Value::as_u64)
                .ok_or_else(|| anyhow!("device override requires index"))?
                as usize;
            let slot = rig
                .get_mut(section)
                .and_then(Value::as_array_mut)
                .and_then(|slots| slots.get_mut(index))
                .ok_or_else(|| anyhow!("no device slot at {section}[{index}]"))?;
            if let Some(bypassed) = override_slot.get("bypassed") {
                slot["bypassed"] = bypassed.clone();
            }
            if let Some(controls) = override_slot.get("controls") {
                let slot_controls = object_path_mut(slot, &["controls"])?;
                merge_object(slot_controls, controls)?;
            }
        }
    }
    Ok(())
}

fn object_path_mut<'a>(value: &'a mut Value, path: &[&str]) -> Result<&'a mut Value> {
    let mut current = value;
    for key in path {
        if !current.get(key).is_some() {
            current[key] = json!({});
        }
        current = current
            .get_mut(key)
            .ok_or_else(|| anyhow!("missing object path '{}'", path.join(".")))?;
    }
    if !current.is_object() {
        bail!("path '{}' is not an object", path.join("."));
    }
    Ok(current)
}

fn merge_object(target: &mut Value, source: &Value) -> Result<()> {
    let target = target
        .as_object_mut()
        .ok_or_else(|| anyhow!("target is not an object"))?;
    let source = source
        .as_object()
        .ok_or_else(|| anyhow!("override is not an object"))?;
    for (key, value) in source {
        if key == "enabled" {
            continue;
        }
        target.insert(key.clone(), value.clone());
    }
    Ok(())
}

fn collect_paths(dir: &Path, extensions: &[&str], output: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_paths(&path, extensions, output)?;
        } else if path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| {
                extensions
                    .iter()
                    .any(|candidate| ext.eq_ignore_ascii_case(candidate))
            })
            .unwrap_or(false)
        {
            output.push(path);
        }
    }
    Ok(())
}

fn runtime_path(runtime: &Value, key: &str, fallback: PathBuf) -> PathBuf {
    runtime
        .get(key)
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .unwrap_or(fallback)
}

fn runtime_u64(runtime: &Value, key: &str, fallback: u64) -> u64 {
    runtime.get(key).and_then(Value::as_u64).unwrap_or(fallback)
}

fn runtime_f64(runtime: &Value, key: &str, fallback: f64) -> f64 {
    runtime.get(key).and_then(Value::as_f64).unwrap_or(fallback)
}

fn required_path(arguments: &Value, key: &str) -> Result<String> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow!("{key} is required"))
}

fn path_for_json(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn run_id_from(rig: &str) -> String {
    let stem = Path::new(rig)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("rig");
    format!("{}-{}", run_id_now(), sanitize_id(stem))
}

fn run_id_now() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    format!("{seconds}")
}

fn sanitize_id(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}
