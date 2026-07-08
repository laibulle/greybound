use anyhow::{Context, Result};
use std::fs::{self, File};
use std::path::PathBuf;
use std::process::{Child, Command as ProcessCommand, Stdio};

pub(crate) struct AudioLabMcpSidecar {
    child: Child,
    command: PathBuf,
    stderr_log: PathBuf,
}

impl AudioLabMcpSidecar {
    pub(crate) fn start() -> Result<Self> {
        let root = workspace_root()?;
        let command = find_audio_lab_mcp_binary()?;
        let log_dir = root.join("target/greybound-mcp");
        fs::create_dir_all(&log_dir)?;
        let stderr_log = log_dir.join("audio-lab-mcp.stderr.log");
        let stderr = File::create(&stderr_log)?;
        let child = ProcessCommand::new(&command)
            .current_dir(&root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::from(stderr))
            .spawn()
            .with_context(|| format!("could not start MCP sidecar '{}'", command.display()))?;
        eprintln!(
            "Greybound Audio Lab MCP started: {} (stderr {})",
            command.display(),
            stderr_log.display()
        );
        Ok(Self {
            child,
            command,
            stderr_log,
        })
    }
}

impl Drop for AudioLabMcpSidecar {
    fn drop(&mut self) {
        if let Err(error) = self.child.kill() {
            eprintln!(
                "Greybound Audio Lab MCP shutdown warning for {}: {error}",
                self.command.display()
            );
        }
        let _ = self.child.wait();
        eprintln!(
            "Greybound Audio Lab MCP stopped; stderr log was {}",
            self.stderr_log.display()
        );
    }
}

fn workspace_root() -> Result<PathBuf> {
    let mut dir = std::env::current_dir()?;
    loop {
        if dir.join("Cargo.toml").exists() && dir.join("knowledge").exists() {
            return Ok(dir);
        }
        if !dir.pop() {
            break;
        }
    }
    std::env::current_dir().context("could not resolve workspace root")
}

fn find_audio_lab_mcp_binary() -> Result<PathBuf> {
    let current_exe = std::env::current_exe()?;
    let current_dir = current_exe
        .parent()
        .ok_or_else(|| anyhow::anyhow!("current executable has no parent directory"))?;
    let root = workspace_root()?;
    let candidates = [
        current_dir.join("greybound-audio-lab-mcp"),
        root.join("target/release/greybound-audio-lab-mcp"),
        root.join("target/debug/greybound-audio-lab-mcp"),
    ];
    candidates
        .into_iter()
        .find(|path| path.exists())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "greybound-audio-lab-mcp binary not found; build it with cargo build --release -p greybound-audio-lab-mcp"
            )
        })
}
