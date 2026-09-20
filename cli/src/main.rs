#![allow(unused_variables)]

mod cli;
mod dispatch;

mod analytics;
mod analyze;
mod api_key;
mod audit_command;
mod auth;
mod backup;
mod batch_audit;
mod batch_deploy;
mod batch_export;
mod batch_import;
mod batch_migrate;
mod batch_notify;
mod batch_ops;
mod batch_register;
mod batch_update;
mod batch_verify;
mod cache;
mod cached_http;
mod category;
mod cicd;
mod codegen;
mod commands;
mod compare;
mod completion;
mod config;
mod contract_audit;
mod contract_compatibility;
mod contract_dependency;
mod contract_dependency_graph;
mod contract_deploy;
mod contract_deprecate;
mod contract_highlight;
mod contract_interaction;
mod contract_interfaces;
mod contract_list;
mod contract_provenance;
mod contract_register;
mod contract_risk;
mod contract_search;
mod contract_snapshot;
mod contract_update;
mod contract_verify;
mod contract_verify_build;
mod contracts;
mod conversions;
mod coverage;
mod dashboard;
mod deploy;
mod env;
mod events;
mod export;
mod formal_verification;
mod fuzz;
mod import;
mod incident;
mod io_utils;
mod manifest;
mod migration;
mod multisig;
mod net;
mod network;
mod notification;
mod package_signing;
mod patch;
mod plugins;
mod profiler;
mod publisher;
mod registry;
mod release_notes;
mod shell;
mod sla;
mod table_format;
mod test_framework;
mod track_deployment;
mod upgrade;
mod user_config;
mod user_profile;
mod verification;
mod version;
mod webhook;
mod wizard;

mod diagnostic;
mod output_format;
mod search;
mod search_pagination;
mod snapshot;

use anyhow::Result;
use clap::Parser;

use crate::cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    let mut cli = Cli::parse();

    if cli.check_updates {
        let update_checks_enabled = user_config::load()
            .map(|cfg| cfg.update_checks_enabled)
            .unwrap_or(true);
        if update_checks_enabled {
            let _ = version::check_version(true, false, None).await;
        }
    }

    let cli_api_base = if cli.api_url.trim().is_empty() {
        None
    } else {
        Some(cli.api_url.clone())
    };
    let runtime = config::resolve_runtime_config(
        cli.network.clone(),
        cli_api_base,
        cli.timeout,
        cli.profile.clone(),
    )?;
    cli.api_url = runtime.api_base;
    cli.network = Some(runtime.network.to_string());
    cli.timeout = Some(runtime.timeout);

    cached_http::init(cached_http::HttpCacheOptions {
        no_cache: cli.no_cache,
        verbose: cli.verbose,
    });
    // The shared registry client picks up the same resolved timeout.
    registry::init(cli.timeout);

    // ── Initialise logger ─────────────────────────────────────────────────────
    // -v counts; each level raises verbosity by one step.
    let log_level = match cli.verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    env_logger::Builder::new()
        .parse_filters(log_level)
        .format_timestamp(None) // no timestamps in CLI output
        .format_module_path(cli.verbose > 0) // show module path only when verbose
        .init();

    log::debug!("Verbose mode enabled");
    log::debug!("API URL: {}", cli.api_url);

    dispatch::handle_command(cli).await
}

/// Runs `f` on a thread with a large stack and propagates its panics unchanged.
///
/// Clap's tree walkers -- `try_parse_from`, `Command::debug_assert`, and the
/// `clap_complete` generators -- recurse over the entire command tree. This CLI has 60+
/// top-level commands nested several levels deep, which in a debug build overflows the
/// 2 MiB stack libtest allocates per test thread. Only tests need this: the shipped
/// binary walks the same tree on the main thread, whose default stack is 8 MiB.
///
/// Panics are re-raised with `resume_unwind` so assertion failures keep their original
/// message and location instead of surfacing as a generic join error.
#[cfg(test)]
pub(crate) fn with_large_stack<T, F>(f: F) -> T
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    const STACK_SIZE: usize = 16 * 1024 * 1024;

    std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(f)
        .expect("spawn large-stack test thread")
        .join()
        .unwrap_or_else(|payload| std::panic::resume_unwind(payload))
}
