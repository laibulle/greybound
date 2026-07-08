use std::collections::VecDeque;
use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

pub struct RotatingMonitorLog {
    path: PathBuf,
    capacity: usize,
    lines: VecDeque<String>,
}

impl RotatingMonitorLog {
    pub fn new(path: PathBuf, capacity: usize) -> Self {
        Self {
            path,
            capacity,
            lines: VecDeque::with_capacity(capacity.min(1024)),
        }
    }

    pub fn push_many(&mut self, lines: impl IntoIterator<Item = String>) -> io::Result<()> {
        for line in lines {
            self.lines.push_back(line);
            while self.lines.len() > self.capacity {
                self.lines.pop_front();
            }
        }
        self.flush()
    }

    fn flush(&self) -> io::Result<()> {
        let mut file = File::create(&self.path)?;
        for line in &self.lines {
            writeln!(file, "{line}")?;
        }
        Ok(())
    }
}
