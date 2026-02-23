use anyhow::{bail, Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::process::Command;

// ─── Conventional Commit Types ──────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangeType {
    Feat,
    Fix,
    Docs,
    Style,
    Refactor,
    Perf,
    Test,
    Build,
    Ci,
    Chore,
    Revert,
    Breaking,
}

impl fmt::Display for ChangeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Feat => write!(f, "feat"),
            Self::Fix => write!(f, "fix"),
            Self::Docs => write!(f, "docs"),
            Self::Style => write!(f, "style"),
            Self::Refactor => write!(f, "refactor"),
            Self::Perf => write!(f, "perf"),
            Self::Test => write!(f, "test"),
            Self::Build => write!(f, "build"),
            Self::Ci => write!(f, "ci"),
            Self::Chore => write!(f, "chore"),
            Self::Revert => write!(f, "revert"),
            Self::Breaking => write!(f, "breaking"),
        }
    }
}

impl std::str::FromStr for ChangeType {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "feat" | "feature" => Ok(Self::Feat),
            "fix" | "bugfix" => Ok(Self::Fix),
            "docs" | "doc" => Ok(Self::Docs),
            "style" => Ok(Self::Style),
            "refactor" => Ok(Self::Refactor),
            "perf" | "performance" => Ok(Self::Perf),
            "test" | "tests" => Ok(Self::Test),
            "build" => Ok(Self::Build),
            "ci" => Ok(Self::Ci),
            "chore" => Ok(Self::Chore),
            "revert" => Ok(Self::Revert),
            "breaking" => Ok(Self::Breaking),
            _ => Err(format!("unknown change type: {}", s)),
        }
    }
}

impl ChangeType {
    fn heading(&self) -> &'static str {
        match self {
            Self::Feat => "Features",
            Self::Fix => "Bug Fixes",
            Self::Docs => "Documentation",
            Self::Style => "Styles",
            Self::Refactor => "Code Refactoring",
            Self::Perf => "Performance Improvements",
            Self::Test => "Tests",
            Self::Build => "Build System",
            Self::Ci => "CI/CD",
            Self::Chore => "Chores",
            Self::Revert => "Reverts",
            Self::Breaking => "Breaking Changes",
        }
    }
}

// ─── Parsed Commit ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedCommit {
    pub hash: String,
    pub short_hash: String,
    pub change_type: ChangeType,
    pub scope: Option<String>,
    pub description: String,
    pub body: Option<String>,
    pub is_breaking: bool,
    pub breaking_description: Option<String>,
    pub author: String,
    pub date: String,
}

// ─── SemVer (local lightweight copy) ────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
struct SemVer {
    major: u64,
    minor: u64,
    patch: u64,
}

impl SemVer {
    fn parse(s: &str) -> Option<Self> {
        let s = s.strip_prefix('v').unwrap_or(s);
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        Some(SemVer {
            major: parts[0].parse().ok()?,
            minor: parts[1].parse().ok()?,
            patch: parts[2].parse().ok()?,
        })
    }
}

impl fmt::Display for SemVer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl PartialOrd for SemVer {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SemVer {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.major
            .cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
    }
}

// ─── Git helpers ────────────────────────────────────────────────────────────

fn git_log_raw(from: Option<&str>, to: &str) -> Result<Vec<String>> {
    let range = match from {
        Some(f) => format!("{}..{}", f, to),
        None => to.to_string(),
    };

    let output = Command::new("git")
        .args(["log", &range, "--format=%H|%h|%an|%ai|%s|%b%x00"])
        .output()
        .context("Failed to run git log. Is this a git repository?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git log failed: {}", stderr);
    }

    let raw = String::from_utf8_lossy(&output.stdout);
    Ok(raw
        .split('\0')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect())
}

fn get_latest_tag() -> Option<String> {
    let output = Command::new("git")
        .args(["describe", "--tags", "--abbrev=0"])
        .output()
        .ok()?;

    if output.status.success() {
        let tag = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !tag.is_empty() {
            return Some(tag);
        }
    }
    None
}

// ─── Commit Parser ──────────────────────────────────────────────────────────

fn parse_conventional_commit(raw_line: &str) -> Option<ParsedCommit> {
    let parts: Vec<&str> = raw_line.splitn(6, '|').collect();
    if parts.len() < 5 {
        return None;
    }

    let hash = parts[0].to_string();
    let short_hash = parts[1].to_string();
    let author = parts[2].to_string();
    let date = parts[3].to_string();
    let subject = parts[4].trim();
    let body = parts.get(5).map(|b| b.trim().to_string()).filter(|b| !b.is_empty());

    // Parse: type(scope)!: description
    let (type_part, description) = subject.split_once(':')?;
    let description = description.trim().to_string();
    if description.is_empty() {
        return None;
    }

    let type_part = type_part.trim();
    let has_bang = type_part.ends_with('!');
    let type_part = type_part.trim_end_matches('!');

    let (type_str, scope) = if let Some(paren_start) = type_part.find('(') {
        let paren_end = type_part.find(')')?;
        let scope = type_part[paren_start + 1..paren_end].trim().to_string();
        let t = type_part[..paren_start].trim();
        (t, if scope.is_empty() { None } else { Some(scope) })
    } else {
        (type_part, None)
    };

    let change_type: ChangeType = type_str.parse().ok()?;

    let body_has_breaking = body
        .as_ref()
        .map(|b| b.contains("BREAKING CHANGE:") || b.contains("BREAKING-CHANGE:"))
        .unwrap_or(false);
    let is_breaking = has_bang || body_has_breaking || description.starts_with("BREAKING CHANGE");

    let breaking_description = if is_breaking {
        body.as_ref().and_then(|b| {
            b.lines()
                .find(|l| l.starts_with("BREAKING CHANGE:") || l.starts_with("BREAKING-CHANGE:"))
                .map(|l| {
                    l.trim_start_matches("BREAKING CHANGE:")
                        .trim_start_matches("BREAKING-CHANGE:")
                        .trim()
                        .to_string()
                })
        })
    } else {
        None
    };

    Some(ParsedCommit {
        hash,
        short_hash,
        change_type,
        scope,
        description,
        body,
        is_breaking,
        breaking_description,
        author,
        date,
    })
}

// ─── Markdown Renderer ──────────────────────────────────────────────────────

fn render_changelog_markdown(
    version: &str,
    title: Option<&str>,
    commits: &[ParsedCommit],
) -> String {
    let mut md = String::new();
    let date = chrono::Utc::now().format("%Y-%m-%d");

    md.push_str("# Changelog\n\n");
    md.push_str("All notable changes to this project will be documented in this file.\n\n");
    md.push_str("The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),\n");
    md.push_str("and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).\n\n");

    if let Some(t) = title {
        md.push_str(&format!("## [{}] - {} - {}\n\n", version, t, date));
    } else {
        md.push_str(&format!("## [{}] - {}\n\n", version, date));
    }

    // Breaking changes first
    let breaking: Vec<&ParsedCommit> = commits.iter().filter(|c| c.is_breaking).collect();
    if !breaking.is_empty() {
        md.push_str("### ⚠ BREAKING CHANGES\n\n");
        for commit in &breaking {
            let scope_str = commit
                .scope
                .as_ref()
                .map(|s| format!("**{}:** ", s))
                .unwrap_or_default();
            let desc = commit
                .breaking_description
                .as_ref()
                .unwrap_or(&commit.description);
            md.push_str(&format!(
                "- {}{} ({}) — {}\n",
                scope_str, desc, commit.short_hash, commit.author
            ));
        }
        md.push('\n');
    }

    // Group by type
    let mut grouped: BTreeMap<&ChangeType, Vec<&ParsedCommit>> = BTreeMap::new();
    for commit in commits.iter().filter(|c| !c.is_breaking) {
        grouped
            .entry(&commit.change_type)
            .or_default()
            .push(commit);
    }

    for (change_type, group_commits) in &grouped {
        md.push_str(&format!("### {}\n\n", change_type.heading()));
        for commit in group_commits {
            let scope_str = commit
                .scope
                .as_ref()
                .map(|s| format!("**{}:** ", s))
                .unwrap_or_default();
            md.push_str(&format!(
                "- {}{} ({}) — {}\n",
                scope_str, commit.description, commit.short_hash, commit.author
            ));
        }
        md.push('\n');
    }

    md
}

fn compute_version_bump(current: &SemVer, commits: &[ParsedCommit]) -> (SemVer, &'static str) {
    let has_breaking = commits.iter().any(|c| c.is_breaking);
    let has_features = commits
        .iter()
        .any(|c| matches!(c.change_type, ChangeType::Feat));

    if has_breaking {
        (
            SemVer {
                major: current.major + 1,
                minor: 0,
                patch: 0,
            },
            "major",
        )
    } else if has_features {
        (
            SemVer {
                major: current.major,
                minor: current.minor + 1,
                patch: 0,
            },
            "minor",
        )
    } else {
        (
            SemVer {
                major: current.major,
                minor: current.minor,
                patch: current.patch + 1,
            },
            "patch",
        )
    }
}

// ─── Public entry points ────────────────────────────────────────────────────

pub fn generate(
    output: &str,
    version: Option<&str>,
    title: Option<&str>,
    from_ref: Option<&str>,
    to_ref: Option<&str>,
    json_output: bool,
) -> Result<()> {
    let to = to_ref.unwrap_or("HEAD");
    let from = from_ref.map(|s| s.to_string()).or_else(get_latest_tag);

    println!(
        "\n{} Generating changelog...",
        "●".cyan()
    );
    if let Some(ref f) = from {
        println!("  Range: {} .. {}", f.bright_black(), to.bright_black());
    } else {
        println!("  Range: all commits up to {}", to.bright_black());
    }

    let raw_entries = git_log_raw(from.as_deref(), to)?;
    if raw_entries.is_empty() {
        println!("{}", "  No commits found in range.".yellow());
        return Ok(());
    }

    let parsed: Vec<ParsedCommit> = raw_entries
        .iter()
        .filter_map(|line| parse_conventional_commit(line))
        .collect();

    let skipped = raw_entries.len() - parsed.len();
    println!(
        "  Parsed {} conventional commits ({} skipped)",
        parsed.len().to_string().green(),
        skipped.to_string().yellow()
    );

    let breaking_count = parsed.iter().filter(|c| c.is_breaking).count();
    let feature_count = parsed
        .iter()
        .filter(|c| matches!(c.change_type, ChangeType::Feat))
        .count();
    let fix_count = parsed
        .iter()
        .filter(|c| matches!(c.change_type, ChangeType::Fix))
        .count();

    // Determine version
    let effective_version = if let Some(v) = version {
        v.to_string()
    } else {
        let current = from
            .as_ref()
            .and_then(|t| SemVer::parse(t))
            .unwrap_or(SemVer {
                major: 0,
                minor: 0,
                patch: 0,
            });
        let (next, _) = compute_version_bump(&current, &parsed);
        next.to_string()
    };

    // Version bump validation
    if let Some(ref f) = from {
        if let Some(current_sv) = SemVer::parse(f) {
            if let Some(target_sv) = SemVer::parse(&effective_version) {
                if breaking_count > 0 && target_sv.major <= current_sv.major && current_sv.major > 0
                {
                    println!(
                        "\n{}",
                        "  ⚠ WARNING: Breaking changes detected but major version not bumped!"
                            .red()
                            .bold()
                    );
                    println!(
                        "  {} breaking change(s) found. Recommended: {}.0.0",
                        breaking_count,
                        current_sv.major + 1
                    );
                }
            }
        }
    }

    if json_output {
        let summary = serde_json::json!({
            "version": effective_version,
            "total_commits": raw_entries.len(),
            "parsed_commits": parsed.len(),
            "skipped_commits": skipped,
            "breaking_changes": breaking_count,
            "features": feature_count,
            "fixes": fix_count,
            "commits": parsed,
            "version_recommendation": from.as_ref().and_then(|f| {
                SemVer::parse(f).map(|sv| {
                    let (next, bump_type) = compute_version_bump(&sv, &parsed);
                    serde_json::json!({
                        "current": f,
                        "recommended": next.to_string(),
                        "bump_type": bump_type,
                    })
                })
            }),
        });
        println!("{}", serde_json::to_string_pretty(&summary)?);
        return Ok(());
    }

    let markdown = render_changelog_markdown(&effective_version, title, &parsed);

    std::fs::write(output, &markdown)
        .with_context(|| format!("Failed to write changelog to {}", output))?;

    // Summary
    println!("\n{}", "  Changelog Summary".bold().cyan());
    println!("  {}", "─".repeat(50).cyan());
    println!("  Version:          {}", effective_version.green().bold());
    println!("  Total commits:    {}", raw_entries.len());
    println!("  Parsed:           {}", parsed.len().to_string().green());
    println!("  Skipped:          {}", skipped.to_string().yellow());

    if breaking_count > 0 {
        println!(
            "  Breaking changes: {}",
            breaking_count.to_string().red().bold()
        );
    } else {
        println!("  Breaking changes: {}", "0".green());
    }

    println!("  Features:         {}", feature_count);
    println!("  Bug fixes:        {}", fix_count);
    println!("  Output:           {}", output.bright_black());

    if breaking_count > 0 {
        println!("\n  {} Breaking Changes:", "⚠".red().bold());
        for commit in parsed.iter().filter(|c| c.is_breaking) {
            let scope_str = commit
                .scope
                .as_ref()
                .map(|s| format!("({})", s))
                .unwrap_or_default();
            println!(
                "    {} {}{}: {}",
                "✗".red(),
                commit.change_type.to_string().red(),
                scope_str.bright_black(),
                commit.description
            );
        }
    }

    // Version bump recommendation
    if let Some(ref f) = from {
        if let Some(current_sv) = SemVer::parse(f) {
            let (recommended, bump_type) = compute_version_bump(&current_sv, &parsed);
            println!(
                "\n  {} Version bump: {} → {} ({})",
                "→".cyan(),
                f.bright_black(),
                recommended.to_string().green().bold(),
                bump_type.cyan()
            );
        }
    }

    println!("\n{}", "  ✓ Changelog generated successfully".green().bold());

    Ok(())
}

pub fn validate(version: &str, from_ref: Option<&str>) -> Result<()> {
    let from = from_ref.map(|s| s.to_string()).or_else(get_latest_tag);

    println!(
        "\n{} Validating release {}...",
        "●".cyan(),
        version.green().bold()
    );

    let target_sv = SemVer::parse(version).context("Invalid target version")?;

    let raw_entries = git_log_raw(from.as_deref(), "HEAD")?;
    let parsed: Vec<ParsedCommit> = raw_entries
        .iter()
        .filter_map(|line| parse_conventional_commit(line))
        .collect();

    let breaking_count = parsed.iter().filter(|c| c.is_breaking).count();
    let has_features = parsed
        .iter()
        .any(|c| matches!(c.change_type, ChangeType::Feat));

    let mut errors: Vec<String> = Vec::new();

    if let Some(ref f) = from {
        if let Some(current_sv) = SemVer::parse(f) {
            if target_sv <= current_sv {
                errors.push(format!(
                    "Version {} must be greater than current version {}",
                    version, f
                ));
            }

            if breaking_count > 0 && target_sv.major <= current_sv.major && current_sv.major > 0 {
                errors.push(format!(
                    "Breaking changes require major version bump: expected {}.0.0+, got {}",
                    current_sv.major + 1,
                    version
                ));
            }

            if has_features
                && target_sv.major == current_sv.major
                && target_sv.minor <= current_sv.minor
                && breaking_count == 0
            {
                errors.push(format!(
                    "New features require at least a minor version bump: expected {}.{}.0+, got {}",
                    current_sv.major,
                    current_sv.minor + 1,
                    version
                ));
            }
        }
    }

    if errors.is_empty() {
        println!(
            "  {} Version {} is valid for this release",
            "✓".green().bold(),
            version.green()
        );
        println!("    {} conventional commits", parsed.len());
        println!("    {} breaking change(s)", breaking_count);
        Ok(())
    } else {
        println!("  {} Validation failed:", "✗".red().bold());
        for err in &errors {
            println!("    {} {}", "✗".red(), err);
        }
        bail!(
            "Version validation failed: {} error(s)",
            errors.len()
        );
    }
}

pub async fn push_changelog(
    api_url: &str,
    contract_id: &str,
    version: &str,
    title: Option<&str>,
    from_ref: Option<&str>,
    to_ref: Option<&str>,
    is_prerelease: bool,
) -> Result<()> {
    let to = to_ref.unwrap_or("HEAD");
    let from = from_ref.map(|s| s.to_string()).or_else(get_latest_tag);

    println!(
        "\n{} Pushing changelog to registry for contract {}...",
        "●".cyan(),
        contract_id.bright_black()
    );

    let raw_entries = git_log_raw(from.as_deref(), to)?;
    let parsed: Vec<ParsedCommit> = raw_entries
        .iter()
        .filter_map(|line| parse_conventional_commit(line))
        .collect();

    if parsed.is_empty() {
        bail!("No conventional commits found to push");
    }

    let commits: Vec<serde_json::Value> = parsed
        .iter()
        .map(|c| {
            serde_json::json!({
                "hash": c.hash,
                "message": format!("{}{}{}: {}",
                    c.change_type,
                    c.scope.as_ref().map(|s| format!("({})", s)).unwrap_or_default(),
                    if c.is_breaking { "!" } else { "" },
                    c.description
                ),
                "author": c.author,
            })
        })
        .collect();

    let body = serde_json::json!({
        "contract_id": contract_id,
        "version": version,
        "title": title,
        "commits": commits,
        "is_prerelease": is_prerelease,
    });

    let client = reqwest::Client::new();
    let url = format!("{}/api/contracts/{}/changelog", api_url, contract_id);

    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .context("Failed to push changelog to registry")?;

    let status = response.status();
    let resp_body: serde_json::Value = response.json().await?;

    if status.is_success() {
        println!("  {} Changelog pushed successfully", "✓".green().bold());
        if let Some(id) = resp_body.get("changelog_id") {
            println!("  Changelog ID: {}", id.to_string().bright_black());
        }
        if let Some(count) = resp_body.get("entries_count") {
            println!("  Entries: {}", count);
        }
        if let Some(breaking) = resp_body.get("breaking_changes_count") {
            let n = breaking.as_u64().unwrap_or(0);
            if n > 0 {
                println!(
                    "  Breaking changes: {}",
                    n.to_string().red().bold()
                );
            }
        }
    } else {
        let msg = resp_body
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown error");
        bail!("Failed to push changelog (HTTP {}): {}", status, msg);
    }

    Ok(())
}
