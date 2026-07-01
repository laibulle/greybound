use anyhow::{anyhow, bail, Context, Result};
use greybound::rig::RigDeviceSlot;
use greybound::{amp_model_descriptor, ControlKind, DeviceConfig, RigConfig, SignalChainConfig};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
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
    let server = Server::new(env::current_dir()?);
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
        let monitor_log = runtime_path(
            &runtime,
            "monitor_log",
            render_dir.join(format!("{run_id}.monitor.log")),
        );
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

        let cli = self.root.join("target/release/greybound-cli");
        let mut command = Command::new(&cli);
        command
            .current_dir(&self.root)
            .arg("--rig")
            .arg(&rig_arg)
            .arg("--input-wav")
            .arg(&input_wav)
            .arg("--input-channel")
            .arg((input_channel + 1).to_string())
            .arg("--output-wav")
            .arg(&output_wav)
            .arg("--render-seconds")
            .arg(render_seconds.to_string())
            .arg("--sample-rate")
            .arg(sample_rate.to_string())
            .arg("--period-size")
            .arg(period_size.to_string())
            .arg("--ir")
            .arg(&ir)
            .arg("--monitor-log")
            .arg(&monitor_log);
        if monitor {
            command.arg("--monitor");
        }

        let command_vec = command_to_vec(&cli, &command);
        let output = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .with_context(|| format!("failed to launch {}", cli.display()))?;
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let diagnostics = self.monitor_render_paths(&monitor_log, &output_wav)?;
        let status = if !output.status.success() {
            "severe"
        } else {
            diagnostics
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("warning")
        };
        let git_revision = self
            .git_revision()
            .unwrap_or_else(|_| "unknown".to_string());

        let metadata = json!({
            "run_id": run_id,
            "status": status,
            "command": command_vec,
            "exit_status": output.status.code(),
            "stdout": stdout,
            "stderr": stderr,
            "git_revision": git_revision,
            "rig": rig,
            "effective_rig": path_for_json(&rig_arg),
            "runtime": {
                "sample_rate": sample_rate,
                "period_size": period_size,
                "render_seconds": render_seconds,
                "input_wav": path_for_json(&input_wav),
                "input_channel": input_channel,
                "ir": path_for_json(&ir),
                "output_wav": path_for_json(&output_wav),
                "monitor": monitor
            },
            "overrides": overrides,
            "diagnostics": diagnostics
        });
        fs::write(&metadata_path, serde_json::to_vec_pretty(&metadata)?)?;

        Ok(json!({
            "run_id": run_id,
            "status": status,
            "candidate_wav": path_for_json(&output_wav),
            "monitor_log": path_for_json(&monitor_log),
            "metadata": path_for_json(&metadata_path),
            "report": null,
            "diagnostics": diagnostics,
            "command": command_vec,
            "stdout": stdout,
            "stderr": stderr
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
        fs::create_dir_all(parent_or_root(&report))?;

        let mut command = Command::new("uv");
        command
            .current_dir(&self.root)
            .arg("--project")
            .arg("lab")
            .arg("run")
            .arg("greybound-lab")
            .arg("compare-wav")
            .arg("--candidate")
            .arg(self.resolve(&candidate))
            .arg("--reference")
            .arg(self.resolve(&reference))
            .arg("--report")
            .arg(&report);
        if let Some(metadata) = arguments.get("metadata").and_then(Value::as_str) {
            command.arg("--metadata").arg(self.resolve(metadata));
        }
        if let Some(segments) = arguments.get("segments").and_then(Value::as_str) {
            command.arg("--segments").arg(self.resolve(segments));
        }

        let command_vec = command_to_vec(Path::new("uv"), &command);
        let output = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;
        Ok(json!({
            "status": if output.status.success() { "clean" } else { "severe" },
            "report": path_for_json(&report),
            "command": command_vec,
            "stdout": String::from_utf8_lossy(&output.stdout).trim(),
            "stderr": String::from_utf8_lossy(&output.stderr).trim(),
            "exit_status": output.status.code()
        }))
    }

    fn evaluate_wav(&self, arguments: &Value) -> Result<Value> {
        let candidate = required_path(arguments, "candidate_wav")?;
        let monitor_log = arguments.get("monitor_log").and_then(Value::as_str);
        let diagnostics = if let Some(monitor_log) = monitor_log {
            self.monitor_render_paths(&self.resolve(monitor_log), &self.resolve(&candidate))?
        } else {
            wav_summary(&self.resolve(&candidate))?
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
        let mut diagnostics = Map::new();
        diagnostics.insert("monitor_log".to_string(), json!(path_for_json(monitor_log)));
        if candidate_wav.exists() {
            diagnostics.insert(
                "candidate_wav".to_string(),
                json!(path_for_json(candidate_wav)),
            );
            if let Value::Object(wav) = wav_summary(candidate_wav)? {
                diagnostics.extend(wav);
            }
        }

        let monitor = parse_monitor_log(monitor_log)?;
        let mut warnings = Vec::new();
        let xrun_count = monitor.input_xruns + monitor.output_xruns;
        let hard_clip_count = monitor.input_hard_clips + monitor.output_hard_clips;
        let near_clip_count = monitor.input_near_clips + monitor.output_near_clips;
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
        if let Some(output_peak) = diagnostics.get("peak").and_then(Value::as_f64) {
            if output_peak <= 0.000001 {
                warnings.push("candidate output appears silent".to_string());
            }
        }

        let status = if xrun_count > 0 || hard_clip_count > 0 {
            "severe"
        } else if warnings.is_empty() {
            "clean"
        } else {
            "warning"
        };

        diagnostics.insert("status".to_string(), json!(status));
        diagnostics.insert("warnings".to_string(), json!(warnings));
        diagnostics.insert("monitor_entries".to_string(), json!(monitor.entries));
        diagnostics.insert("input_rms_dbfs".to_string(), json!(monitor.input_rms_dbfs));
        diagnostics.insert(
            "input_peak_dbfs".to_string(),
            json!(monitor.input_peak_dbfs),
        );
        diagnostics.insert(
            "output_rms_dbfs".to_string(),
            json!(monitor.output_rms_dbfs),
        );
        diagnostics.insert(
            "output_peak_dbfs".to_string(),
            json!(monitor.output_peak_dbfs),
        );
        diagnostics.insert("xrun_count".to_string(), json!(xrun_count));
        diagnostics.insert("hard_clip_count".to_string(), json!(hard_clip_count));
        diagnostics.insert("near_clip_count".to_string(), json!(near_clip_count));
        diagnostics.insert(
            "input_near_clip_count".to_string(),
            json!(monitor.input_near_clips),
        );
        diagnostics.insert(
            "input_hard_clip_count".to_string(),
            json!(monitor.input_hard_clips),
        );
        diagnostics.insert(
            "output_near_clip_count".to_string(),
            json!(monitor.output_near_clips),
        );
        diagnostics.insert(
            "output_hard_clip_count".to_string(),
            json!(monitor.output_hard_clips),
        );
        diagnostics.insert("input_xruns".to_string(), json!(monitor.input_xruns));
        diagnostics.insert("output_xruns".to_string(), json!(monitor.output_xruns));
        Ok(Value::Object(diagnostics))
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

fn parse_monitor_log(path: &Path) -> Result<MonitorAggregate> {
    if !path.exists() {
        return Ok(MonitorAggregate::default());
    }
    let text = fs::read_to_string(path)?;
    let mut aggregate = MonitorAggregate::default();
    for line in text.lines().filter(|line| line.contains(" MON ")) {
        aggregate.entries += 1;
        let fields = parse_monitor_line(line);
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

fn wav_summary(path: &Path) -> Result<Value> {
    let mut reader = hound::WavReader::open(path)?;
    let spec = reader.spec();
    let mut samples = 0_u64;
    let mut sum_sq = 0.0_f64;
    let mut peak = 0.0_f64;

    match (spec.sample_format, spec.bits_per_sample) {
        (hound::SampleFormat::Float, 32) => {
            for sample in reader.samples::<f32>() {
                let value = sample? as f64;
                samples += 1;
                sum_sq += value * value;
                peak = peak.max(value.abs());
            }
        }
        (hound::SampleFormat::Int, bits) if bits <= 16 => {
            let scale = i16::MAX as f64;
            for sample in reader.samples::<i16>() {
                let value = sample? as f64 / scale;
                samples += 1;
                sum_sq += value * value;
                peak = peak.max(value.abs());
            }
        }
        (hound::SampleFormat::Int, _) => {
            let scale = i32::MAX as f64;
            for sample in reader.samples::<i32>() {
                let value = sample? as f64 / scale;
                samples += 1;
                sum_sq += value * value;
                peak = peak.max(value.abs());
            }
        }
        _ => {}
    }

    let rms = if samples == 0 {
        0.0
    } else {
        (sum_sq / samples as f64).sqrt()
    };
    Ok(json!({
        "sample_rate": spec.sample_rate,
        "channels": spec.channels,
        "samples": samples,
        "duration_seconds": if spec.sample_rate == 0 || spec.channels == 0 { 0.0 } else { samples as f64 / spec.sample_rate as f64 / spec.channels as f64 },
        "rms": rms,
        "rms_dbfs": amp_to_db(rms),
        "peak": peak,
        "peak_dbfs": amp_to_db(peak),
        "status": if samples == 0 || peak <= 0.000001 { "warning" } else { "clean" }
    }))
}

fn amp_to_db(value: f64) -> Option<f64> {
    (value > 0.0).then(|| 20.0 * value.log10())
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

fn parent_or_root(path: &Path) -> &Path {
    path.parent().unwrap_or_else(|| Path::new("."))
}

fn path_for_json(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn command_to_vec(program: &Path, command: &Command) -> Vec<String> {
    let mut values = vec![program.to_string_lossy().to_string()];
    values.extend(
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().to_string()),
    );
    values
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
