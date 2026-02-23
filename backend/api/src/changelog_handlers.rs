use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde_json::json;
use shared::{
    ChangelogChangeType, ChangelogEntryResponse, ChangelogQueryParams,
    ChangelogReleaseResponse, ContractChangelogResponse, GenerateChangelogRequest,
    RawCommit, SemVer, VersionBumpRecommendation,
};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

// ─── Conventional commit parser ─────────────────────────────────────────────

struct ParsedCommit {
    change_type: ChangelogChangeType,
    scope: Option<String>,
    description: String,
    is_breaking: bool,
    hash: String,
    author: Option<String>,
}

fn parse_conventional_commit(raw: &RawCommit) -> Option<ParsedCommit> {
    let msg = raw.message.trim();

    // Pattern: type(scope)!: description  OR  type!: description  OR  type: description
    let (type_part, rest) = msg.split_once(':')?;
    let description = rest.trim().to_string();
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

    let change_type: ChangelogChangeType = type_str.parse().ok()?;

    let is_breaking = has_bang
        || description.starts_with("BREAKING CHANGE")
        || description.contains("BREAKING CHANGE:");

    Some(ParsedCommit {
        change_type,
        scope,
        description,
        is_breaking,
        hash: raw.hash.clone(),
        author: raw.author.clone(),
    })
}

fn recommend_version_bump(
    current: &str,
    commits: &[ParsedCommit],
) -> Option<VersionBumpRecommendation> {
    let semver = SemVer::parse(current)?;

    let breaking_count = commits.iter().filter(|c| c.is_breaking).count();
    let feature_count = commits
        .iter()
        .filter(|c| matches!(c.change_type, ChangelogChangeType::Feat))
        .count();
    let fix_count = commits
        .iter()
        .filter(|c| matches!(c.change_type, ChangelogChangeType::Fix))
        .count();

    let (bump_type, next) = if breaking_count > 0 {
        (
            "major",
            SemVer {
                major: semver.major + 1,
                minor: 0,
                patch: 0,
            },
        )
    } else if feature_count > 0 {
        (
            "minor",
            SemVer {
                major: semver.major,
                minor: semver.minor + 1,
                patch: 0,
            },
        )
    } else {
        (
            "patch",
            SemVer {
                major: semver.major,
                minor: semver.minor,
                patch: semver.patch + 1,
            },
        )
    };

    Some(VersionBumpRecommendation {
        current_version: current.to_string(),
        recommended_version: next.to_string(),
        bump_type: bump_type.to_string(),
        has_breaking_changes: breaking_count > 0,
        breaking_count,
        feature_count,
        fix_count,
    })
}

fn render_markdown(version: &str, title: Option<&str>, parsed: &[ParsedCommit]) -> String {
    let mut md = String::new();
    let date = chrono::Utc::now().format("%Y-%m-%d");

    if let Some(t) = title {
        md.push_str(&format!("## [{}] - {} - {}\n\n", version, t, date));
    } else {
        md.push_str(&format!("## [{}] - {}\n\n", version, date));
    }

    let breaking: Vec<&ParsedCommit> = parsed.iter().filter(|c| c.is_breaking).collect();
    let features: Vec<&ParsedCommit> = parsed
        .iter()
        .filter(|c| matches!(c.change_type, ChangelogChangeType::Feat) && !c.is_breaking)
        .collect();
    let fixes: Vec<&ParsedCommit> = parsed
        .iter()
        .filter(|c| matches!(c.change_type, ChangelogChangeType::Fix) && !c.is_breaking)
        .collect();
    let perf: Vec<&ParsedCommit> = parsed
        .iter()
        .filter(|c| matches!(c.change_type, ChangelogChangeType::Perf) && !c.is_breaking)
        .collect();
    let docs: Vec<&ParsedCommit> = parsed
        .iter()
        .filter(|c| matches!(c.change_type, ChangelogChangeType::Docs) && !c.is_breaking)
        .collect();
    let other: Vec<&ParsedCommit> = parsed
        .iter()
        .filter(|c| {
            !c.is_breaking
                && !matches!(
                    c.change_type,
                    ChangelogChangeType::Feat
                        | ChangelogChangeType::Fix
                        | ChangelogChangeType::Perf
                        | ChangelogChangeType::Docs
                )
        })
        .collect();

    fn write_section(md: &mut String, heading: &str, items: &[&ParsedCommit]) {
        if items.is_empty() {
            return;
        }
        md.push_str(&format!("### {}\n\n", heading));
        for commit in items {
            let scope_str = commit
                .scope
                .as_ref()
                .map(|s| format!("**{}:** ", s))
                .unwrap_or_default();
            let hash_str = if commit.hash.len() >= 7 {
                format!(" ({})", &commit.hash[..7])
            } else if !commit.hash.is_empty() {
                format!(" ({})", &commit.hash)
            } else {
                String::new()
            };
            md.push_str(&format!(
                "- {}{}{}\n",
                scope_str, commit.description, hash_str
            ));
        }
        md.push('\n');
    }

    write_section(&mut md, "⚠ BREAKING CHANGES", &breaking);
    write_section(&mut md, "Features", &features);
    write_section(&mut md, "Bug Fixes", &fixes);
    write_section(&mut md, "Performance", &perf);
    write_section(&mut md, "Documentation", &docs);
    write_section(&mut md, "Other Changes", &other);

    md
}

// ─── Handlers ───────────────────────────────────────────────────────────────

pub async fn get_contract_changelog(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<ChangelogQueryParams>,
) -> ApiResult<Json<ContractChangelogResponse>> {
    let limit = params.limit.unwrap_or(20).min(100);
    let offset = params.offset.unwrap_or(0);
    let include_pre = params.include_prereleases.unwrap_or(false);

    let changelogs = if include_pre {
        sqlx::query_as::<_, shared::ContractChangelog>(
            "SELECT * FROM contract_changelogs
             WHERE contract_id = $1
             ORDER BY release_date DESC
             LIMIT $2 OFFSET $3",
        )
        .bind(id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await
    } else {
        sqlx::query_as::<_, shared::ContractChangelog>(
            "SELECT * FROM contract_changelogs
             WHERE contract_id = $1 AND is_prerelease = FALSE
             ORDER BY release_date DESC
             LIMIT $2 OFFSET $3",
        )
        .bind(id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await
    }
    .map_err(|e| {
        tracing::error!(error = ?e, "failed to fetch changelogs");
        ApiError::internal("Failed to fetch changelogs")
    })?;

    let total: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM contract_changelogs WHERE contract_id = $1",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "failed to count changelogs");
        ApiError::internal("Failed to count changelogs")
    })?;

    let mut releases = Vec::with_capacity(changelogs.len());
    for cl in &changelogs {
        let entries = sqlx::query_as::<_, shared::ChangelogEntry>(
            "SELECT * FROM changelog_entries WHERE changelog_id = $1 ORDER BY created_at ASC",
        )
        .bind(cl.id)
        .fetch_all(&state.db)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "failed to fetch changelog entries");
            ApiError::internal("Failed to fetch changelog entries")
        })?;

        let to_response = |e: &shared::ChangelogEntry| ChangelogEntryResponse {
            change_type: e.change_type.to_string(),
            scope: e.scope.clone(),
            description: e.description.clone(),
            commit_hash: e.commit_hash.clone(),
            is_breaking: e.is_breaking,
            author: e.author.clone(),
        };

        let breaking_changes: Vec<_> = entries.iter().filter(|e| e.is_breaking).map(to_response).collect();
        let features: Vec<_> = entries
            .iter()
            .filter(|e| e.change_type == ChangelogChangeType::Feat && !e.is_breaking)
            .map(to_response)
            .collect();
        let fixes: Vec<_> = entries
            .iter()
            .filter(|e| e.change_type == ChangelogChangeType::Fix && !e.is_breaking)
            .map(to_response)
            .collect();
        let other: Vec<_> = entries
            .iter()
            .filter(|e| {
                !e.is_breaking
                    && !matches!(
                        e.change_type,
                        ChangelogChangeType::Feat | ChangelogChangeType::Fix
                    )
            })
            .map(to_response)
            .collect();

        releases.push(ChangelogReleaseResponse {
            version: cl.version.clone(),
            title: cl.title.clone(),
            release_date: cl.release_date,
            is_prerelease: cl.is_prerelease,
            breaking_changes,
            features,
            fixes,
            other,
            markdown: cl.markdown.clone(),
        });
    }

    Ok(Json(ContractChangelogResponse {
        contract_id: id,
        releases,
        total_releases: total.0,
    }))
}

pub async fn generate_changelog(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<GenerateChangelogRequest>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let contract_exists: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM contracts WHERE id = $1")
            .bind(id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| {
                tracing::error!(error = ?e, "failed to look up contract");
                ApiError::internal("Failed to look up contract")
            })?;

    if contract_exists.is_none() {
        return Err(ApiError::not_found(
            "ContractNotFound",
            format!("Contract {} not found", id),
        ));
    }

    if SemVer::parse(&req.version).is_none() {
        return Err(ApiError::bad_request(
            "InvalidVersion",
            format!("'{}' is not a valid semver version", req.version),
        ));
    }

    let existing: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM contract_changelogs WHERE contract_id = $1 AND version = $2",
    )
    .bind(id)
    .bind(&req.version)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "failed to check existing changelog");
        ApiError::internal("Failed to check existing changelog")
    })?;

    if existing.is_some() {
        return Err(ApiError::conflict(
            "ChangelogExists",
            format!(
                "Changelog for version {} already exists for contract {}",
                req.version, id
            ),
        ));
    }

    let parsed: Vec<ParsedCommit> = req
        .commits
        .iter()
        .filter_map(parse_conventional_commit)
        .collect();

    if parsed.is_empty() {
        return Err(ApiError::bad_request(
            "NoValidCommits",
            "None of the provided commits follow conventional commit format (type: description)",
        ));
    }

    // Validate version bump against breaking changes
    let latest_version: Option<(String,)> = sqlx::query_as(
        "SELECT version FROM contract_changelogs
         WHERE contract_id = $1
         ORDER BY release_date DESC LIMIT 1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "failed to fetch latest version");
        ApiError::internal("Failed to fetch latest version")
    })?;

    let has_breaking = parsed.iter().any(|c| c.is_breaking);
    if let Some((prev,)) = &latest_version {
        if let (Some(prev_sv), Some(new_sv)) = (SemVer::parse(prev), SemVer::parse(&req.version)) {
            if has_breaking && new_sv.major <= prev_sv.major && prev_sv.major > 0 {
                return Err(ApiError::bad_request(
                    "VersionBumpRequired",
                    format!(
                        "Breaking changes detected but version {} does not bump the major version from {}. \
                         Bump to {}.0.0 or higher.",
                        req.version, prev, prev_sv.major + 1
                    ),
                ));
            }
        }
    }

    let markdown =
        render_markdown(&req.version, req.title.as_deref(), &parsed);

    let recommendation = latest_version
        .as_ref()
        .and_then(|(v,)| recommend_version_bump(v, &parsed));

    let is_prerelease = req.is_prerelease.unwrap_or(false);

    let changelog_id: (Uuid,) = sqlx::query_as(
        "INSERT INTO contract_changelogs (contract_id, version, title, is_prerelease, markdown, metadata)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING id",
    )
    .bind(id)
    .bind(&req.version)
    .bind(&req.title)
    .bind(is_prerelease)
    .bind(&markdown)
    .bind(json!({"commit_count": req.commits.len(), "parsed_count": parsed.len()}))
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "failed to insert changelog");
        ApiError::internal("Failed to insert changelog")
    })?;

    for commit in &parsed {
        sqlx::query(
            "INSERT INTO changelog_entries (changelog_id, change_type, scope, description, commit_hash, is_breaking, author)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(changelog_id.0)
        .bind(&commit.change_type)
        .bind(&commit.scope)
        .bind(&commit.description)
        .bind(&commit.hash)
        .bind(commit.is_breaking)
        .bind(&commit.author)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "failed to insert changelog entry");
            ApiError::internal("Failed to insert changelog entry")
        })?;
    }

    let breaking_count = parsed.iter().filter(|c| c.is_breaking).count();

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "changelog_id": changelog_id.0,
            "version": req.version,
            "entries_count": parsed.len(),
            "breaking_changes_count": breaking_count,
            "skipped_commits": req.commits.len() - parsed.len(),
            "markdown": markdown,
            "version_recommendation": recommendation,
        })),
    ))
}
