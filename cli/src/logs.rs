use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use clap::ValueEnum;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::VecDeque,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Info,
    Warn,
    Error,
    Debug,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
            LogLevel::Debug => "debug",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub service: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct LogFilter {
    pub level: Option<LogLevel>,
    pub service: Option<String>,
    pub search: Option<String>,
}

pub fn append_log(entry: &LogEntry) -> Result<()> {
    let path = default_log_path()?;
    append_log_at(&path, entry)
}

pub fn append_log_at(path: &Path, entry: &LogEntry) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("Failed to open log file: {}", path.display()))?;
    let line = serde_json::to_string(entry)?;
    file.write_all(line.as_bytes())?;
    file.write_all(b"\n")?;
    Ok(())
}

pub fn default_log_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Cannot determine home directory")?;
    Ok(home
        .join(".soroban-registry")
        .join("registry-cli.log.ndjson"))
}

pub fn read_logs(path: &Path, filter: &LogFilter, limit: usize) -> Result<Vec<LogEntry>> {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return Ok(vec![]),
    };
    let reader = BufReader::new(file);
    let mut out: VecDeque<LogEntry> = VecDeque::with_capacity(limit.max(1));
    for line in reader.lines().flatten() {
        let entry: LogEntry = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if !matches_filter(&entry, filter) {
            continue;
        }
        if out.len() == limit {
            out.pop_front();
        }
        out.push_back(entry);
    }
    Ok(out.into_iter().collect())
}

pub async fn follow_logs(
    path: &Path,
    filter: LogFilter,
    mut on_entry: impl FnMut(LogEntry) + Send + 'static,
) -> Result<()> {
    let mut pos: u64 = 0;
    if let Ok(meta) = std::fs::metadata(path) {
        pos = meta.len();
    }

    loop {
        let mut file = match File::open(path) {
            Ok(f) => f,
            Err(_) => {
                tokio::time::sleep(Duration::from_millis(500)).await;
                continue;
            }
        };

        file.seek(SeekFrom::Start(pos)).ok();
        let mut reader = BufReader::new(file);
        let mut buf = String::new();
        while reader.read_line(&mut buf)? > 0 {
            pos += buf.len() as u64;
            let line = buf.trim_end().to_string();
            buf.clear();
            if line.is_empty() {
                continue;
            }
            let entry: LogEntry = match serde_json::from_str(&line) {
                Ok(v) => v,
                Err(_) => continue,
            };
            if matches_filter(&entry, &filter) {
                on_entry(entry);
            }
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

pub fn format_entry(entry: &LogEntry, colored: bool) -> String {
    let ts = entry.timestamp.to_rfc3339();
    let base = format!(
        "{} [{}] {} - {}",
        ts,
        entry.level.as_str(),
        entry.service,
        entry.message
    );

    if !colored {
        return base;
    }

    match entry.level {
        LogLevel::Error => base.red().to_string(),
        LogLevel::Warn => base.yellow().to_string(),
        LogLevel::Info => base.green().to_string(),
        LogLevel::Debug => base.dimmed().to_string(),
    }
}

fn matches_filter(entry: &LogEntry, filter: &LogFilter) -> bool {
    if let Some(level) = filter.level {
        if entry.level != level {
            return false;
        }
    }
    if let Some(ref svc) = filter.service {
        if entry.service != *svc {
            return false;
        }
    }
    if let Some(ref q) = filter.search {
        let q = q.to_lowercase();
        let hay = format!(
            "{} {} {}",
            entry.service,
            entry.message,
            entry.context
                .as_ref()
                .map(|c| c.to_string())
                .unwrap_or_default()
        )
        .to_lowercase();
        if !hay.contains(&q) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn filters_and_exports() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("logs.ndjson");
        let e1 = LogEntry {
            timestamp: Utc::now(),
            level: LogLevel::Info,
            service: "cli".to_string(),
            message: "hello".to_string(),
            context: None,
        };
        let e2 = LogEntry {
            timestamp: Utc::now(),
            level: LogLevel::Error,
            service: "webhook".to_string(),
            message: "fail".to_string(),
            context: None,
        };
        append_log_at(&path, &e1).unwrap();
        append_log_at(&path, &e2).unwrap();

        let filter = LogFilter {
            level: Some(LogLevel::Error),
            service: None,
            search: None,
        };
        let logs = read_logs(&path, &filter, 50).unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].service, "webhook");
    }

    #[tokio::test]
    async fn follow_emits_new_entries() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("logs.ndjson");

        let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(8);
        let filter = LogFilter {
            level: None,
            service: None,
            search: Some("needle".to_string()),
        };

        let path_clone = path.clone();
        tokio::spawn(async move {
            let _ = follow_logs(&path_clone, filter, move |entry| {
                let _ = tx.blocking_send(entry.message);
            })
            .await;
        });

        tokio::time::sleep(Duration::from_millis(200)).await;
        append_log_at(
            &path,
            &LogEntry {
                timestamp: Utc::now(),
                level: LogLevel::Info,
                service: "cli".to_string(),
                message: "needle found".to_string(),
                context: None,
            },
        )
        .unwrap();

        let msg = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(msg.contains("needle"));
    }
}

