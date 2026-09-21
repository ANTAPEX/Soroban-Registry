#![allow(unused_variables)]

mod cli;
mod commands;
mod config;
mod dispatch;
mod support;

use anyhow::Result;
use clap::Parser;

use crate::cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    let mut cli = Cli::parse();

    if cli.check_updates {
        let update_checks_enabled = crate::config::user::load()
            .map(|cfg| cfg.update_checks_enabled)
            .unwrap_or(true);
        if update_checks_enabled {
            let _ = crate::commands::version::check_version(true, false, None).await;
        }
    }

    let cli_api_base = if cli.api_url.trim().is_empty() {
        None
    } else {
        Some(cli.api_url.clone())
    };
    let runtime = crate::config::resolve_runtime_config(
        cli.network.clone(),
        cli_api_base,
        cli.timeout,
        cli.profile.clone(),
    )?;
    cli.api_url = runtime.api_base;
    cli.network = Some(runtime.network.to_string());
    cli.timeout = Some(runtime.timeout);

    crate::support::cached_http::init(crate::support::cached_http::HttpCacheOptions {
        no_cache: cli.no_cache,
        verbose: cli.verbose,
    });
    // The shared registry client picks up the same resolved timeout.
    crate::support::registry::init(cli.timeout);

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

    crate::dispatch::handle_command(cli).await
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
