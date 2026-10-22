//! Runs the command the user asked for.
//!
//! `handle_command` resolves the network and the rest is one arm per command,
//! each handing off to the module under `commands` that implements it.

use anyhow::Result;
use colored::Colorize;

use crate::cli::*;
use crate::commands::patch::Severity;

pub async fn handle_command(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Repl {
            network: shell_network,
        } => crate::commands::shell::run(&cli.api_url, shell_network).await,
        _ => {
            // ── Resolve network ───────────────────────────────────────────────────────
            let cfg_network =
                crate::config::resolve_network(cli.network.clone(), cli.profile.clone())?;
            let mut net_str = cfg_network.to_string();
            if net_str == "auto" {
                net_str = "mainnet".to_string();
            }
            let network: crate::support::network::Network = net_str.parse().unwrap();

            dispatch_command(cli, network, cfg_network).await
        }
    }
}

pub async fn dispatch_command(
    cli: Cli,
    network: crate::support::network::Network,
    cfg_network: crate::config::Network,
) -> Result<()> {
    log::debug!("Network: {:?}", network);

    match cli.command {
        Commands::Repl { .. } => {
            // Already handled at top level, but for completeness or nested calls:
            // We could call crate::commands::shell::run here again but to break recursion we don't.
            println!("{}", "Warning: REPL already running".yellow());
            return Ok(());
        }
        Commands::TrackDeployment {
            contract_id,
            network,
            tx_hash,
            wait_timeout,
            json,
        } => {
            log::debug!(
                "Command: track-deployment | contract_id={} network={} tx_hash={:?} wait_timeout={} json={}",
                contract_id, network, tx_hash, wait_timeout, json
            );
            crate::commands::track_deployment::run(
                &cli.api_url,
                &contract_id,
                &network,
                tx_hash.as_deref(),
                wait_timeout,
                json,
            )
            .await?;
        }
        Commands::Plugins { action } => match action {
            PluginCommands::List { json } => {
                let installed = crate::commands::plugins::discover_installed()?;
                if json {
                    let out: Vec<serde_json::Value> = installed
                        .into_iter()
                        .map(|p| {
                            serde_json::json!({
                                "manifest": p.manifest,
                                "path": p.manifest_path.to_string_lossy().to_string()
                            })
                        })
                        .collect();
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({ "plugins": out }))?
                    );
                } else {
                    if installed.is_empty() {
                        println!("{}", "No plugins installed.".yellow());
                    } else {
                        println!("\n{}", "Installed Plugins:".bold().cyan());
                        println!("{}", "=".repeat(80).cyan());
                        for p in installed {
                            let desc = p.manifest.description.clone().unwrap_or_default();
                            println!(
                                "  {}@{}  {}",
                                p.manifest.name.bold(),
                                p.manifest.version.bright_blue(),
                                desc.bright_black()
                            );
                            for cmd in &p.manifest.commands {
                                println!(
                                    "    - {}  {}",
                                    cmd.name.bright_green(),
                                    cmd.description.clone().unwrap_or_default().bright_black()
                                );
                            }
                        }
                    }
                }
            }
            PluginCommands::Marketplace { json } => {
                let marketplace = crate::commands::plugins::fetch_marketplace(&cli.api_url).await?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&marketplace)?);
                } else {
                    if marketplace.plugins.is_empty() {
                        println!("{}", "Marketplace returned no plugins.".yellow());
                    } else {
                        println!("\n{}", "Plugin Marketplace:".bold().cyan());
                        println!("{}", "=".repeat(80).cyan());
                        for p in marketplace.plugins {
                            println!(
                                "  {}@{}  {}",
                                p.name.bold(),
                                p.version.bright_blue(),
                                p.description.unwrap_or_default().bright_black()
                            );
                            for cmd in p.commands {
                                println!(
                                    "    - {}  {}",
                                    cmd.name.bright_green(),
                                    cmd.description.unwrap_or_default().bright_black()
                                );
                            }
                        }
                    }
                }
            }
            PluginCommands::Install { name, version } => {
                crate::commands::plugins::install_from_registry(
                    &cli.api_url,
                    &name,
                    version.as_deref(),
                )
                .await?;
            }
            PluginCommands::Uninstall { name, version } => {
                crate::commands::plugins::uninstall(&name, version.as_deref())?;
            }
            PluginCommands::Run { command, args } => {
                let result = crate::commands::plugins::run_installed_command(
                    &cli.api_url,
                    &network.to_string(),
                    &command,
                    args,
                )
                .await?;
                print!("{}", result.stdout);
            }
            PluginCommands::Config { action } => match action {
                PluginConfigCommands::Get { name } => {
                    let cfg = crate::commands::plugins::get_plugin_config(&name)?;
                    println!("{}", serde_json::to_string_pretty(&cfg)?);
                }
                PluginConfigCommands::Set { name, json } => {
                    crate::commands::plugins::set_plugin_config_json(&name, &json)?;
                    println!("{} Updated config for {}", "[OK]".green(), name.bold());
                }
                PluginConfigCommands::Disable { name } => {
                    crate::commands::plugins::set_plugin_enabled(&name, false)?;
                    println!("{} Disabled {}", "[OK]".green(), name.bold());
                }
                PluginConfigCommands::Enable { name } => {
                    crate::commands::plugins::set_plugin_enabled(&name, true)?;
                    println!("{} Enabled {}", "[OK]".green(), name.bold());
                }
            },
        },
        Commands::External(args) => {
            if args.is_empty() {
                anyhow::bail!("No external command provided");
            }
            let cmd = args[0].clone();
            let rest = args.into_iter().skip(1).collect::<Vec<_>>();
            let result = crate::commands::plugins::run_installed_command(
                &cli.api_url,
                &network.to_string(),
                &cmd,
                rest,
            )
            .await?;
            print!("{}", result.stdout);
        }
        Commands::Search {
            query,
            verified_only,
            networks: filter_networks,
            category,
            sort,
            limit,
            offset,
            json,
        } => {
            let networks_vec: Vec<String> = filter_networks
                .map(|n| n.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_default();
            log::debug!(
                "Command: search | query={:?} verified_only={} networks={:?} category={:?} sort={:?}",
                query,
                verified_only,
                networks_vec,
                category,
                sort
            );
            crate::commands::search::search(
                &cli.api_url,
                &query,
                network,
                verified_only,
                networks_vec,
                category.as_deref(),
                sort.as_deref(),
                limit,
                offset,
                json,
            )
            .await?;
        }
        Commands::Info { id, json, raw } => {
            let use_json = json || raw;
            crate::commands::contract::info::info(&cli.api_url, &id, use_json).await?;
        }
        Commands::Compare {
            ids,
            json,
            export,
            format,
            exit_code,
            diff,
            fields,
        } => {
            let diff_format = crate::commands::compare::DiffFormat::parse(&diff)?;
            let field_filter = fields.map(|values| values.join(","));
            let code = crate::commands::compare::run(
                &cli.api_url,
                ids,
                json,
                export.as_deref(),
                format.as_deref(),
                crate::commands::compare::CompareOptions {
                    exit_code,
                    diff_format,
                    fields: field_filter,
                },
            )
            .await?;
            if exit_code && code != crate::commands::compare::EXIT_IDENTICAL {
                std::process::exit(code);
            }
        }
        Commands::Completion { shell } => {
            crate::commands::completion::generate_script(shell);
            eprintln!("\n{}", crate::commands::completion::install_hint(shell));
        }
        Commands::Analytics {
            query,
            period,
            format,
            sort,
            export,
        } => {
            let parsed_query = crate::commands::analytics::AnalyticsQuery::parse(&query)?;
            crate::commands::analytics::run(
                &cli.api_url,
                parsed_query,
                &period,
                &format,
                sort.as_deref(),
                export.as_deref(),
            )
            .await?;
        }
        Commands::Stats {
            timeframe,
            format,
            output,
        } => {
            log::debug!("Command: stats | timeframe={} format={}", timeframe, format);
            crate::commands::stats::stats(&cli.api_url, &timeframe, &format, output.as_deref())
                .await?;
        }
        Commands::Version {
            check_updates,
            auto_update,
            rollback,
        } => {
            crate::commands::version::check_version(check_updates, auto_update, rollback).await?;
        }
        Commands::Publish {
            contract_id,
            name,
            description,
            network: _publish_network,
            category,
            tags,
            publisher,
            contract_path,
            test_command,
            require_coverage,
            coverage_threshold,
            skip_tests,
        } => {
            let tags_vec = tags
                .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_default();
            log::debug!(
                "Command: publish | contract_id={} name={} tags={:?}",
                contract_id,
                name,
                tags_vec
            );
            crate::commands::publish::publish(
                &cli.api_url,
                &contract_id,
                &name,
                description.as_deref(),
                network,
                category.as_deref(),
                tags_vec,
                &publisher,
                false,
                &contract_path,
                test_command.as_deref(),
                require_coverage,
                coverage_threshold,
                skip_tests,
            )
            .await?;
        }
        Commands::List {
            limit,
            offset,
            networks: filter_networks,
            category,
            format,
        } => {
            let networks_vec: Vec<String> = filter_networks
                .map(|n| n.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_default();
            crate::commands::list::contract_list(
                &cli.api_url,
                limit,
                offset,
                Some(cfg_network),
                networks_vec,
                category,
                &format,
            )
            .await?;
        }
        Commands::Dashboard {
            refresh_rate,
            category,
            ws_url,
        } => {
            log::debug!(
                "Command: dashboard | refresh_rate={} network={:?} category={:?}",
                refresh_rate,
                cli.network,
                category
            );
            crate::commands::dashboard::run_dashboard(
                crate::commands::dashboard::DashboardParams {
                    refresh_rate_ms: refresh_rate,
                    network: cli.network.clone(),
                    category,
                    ws_url,
                },
            )
            .await?;
        }
        Commands::BreakingChanges {
            old_id,
            new_id,
            json,
        } => {
            log::debug!("Command: breaking-changes | old={} new={}", old_id, new_id);
            crate::commands::breaking_changes::breaking_changes(
                &cli.api_url,
                &old_id,
                &new_id,
                json,
            )
            .await?;
        }
        Commands::UpgradeAnalyze { old, new, json } => {
            log::debug!("Command: upgrade analyze | old={} new={}", old, new);
            crate::commands::upgrade::upgrade_analyze(&cli.api_url, &old, &new, json).await?;
        }
        Commands::Migrate { action } => match action {
            MigrateCommands::Preview { old_id, new_id } => {
                log::debug!(
                    "Command: migrate preview | old_id={} new_id={}",
                    old_id,
                    new_id
                );
                crate::commands::migration::preview(&old_id, &new_id)?;
            }
            MigrateCommands::Analyze { old_id, new_id } => {
                log::debug!(
                    "Command: migrate analyze | old_id={} new_id={}",
                    old_id,
                    new_id
                );
                crate::commands::migration::analyze(&old_id, &new_id)?;
            }
            MigrateCommands::Generate {
                old_id,
                new_id,
                language,
                output,
            } => {
                log::debug!(
                    "Command: migrate generate | old_id={} new_id={} language={}",
                    old_id,
                    new_id,
                    language
                );
                crate::commands::migration::generate_template(
                    &old_id,
                    &new_id,
                    &language,
                    output.as_deref(),
                )?;
            }
            MigrateCommands::Validate { old_id, new_id } => {
                log::debug!(
                    "Command: migrate validate | old_id={} new_id={}",
                    old_id,
                    new_id
                );
                crate::commands::migration::validate(&old_id, &new_id)?;
            }
            MigrateCommands::Apply { old_id, new_id } => {
                log::debug!(
                    "Command: migrate apply | old_id={} new_id={}",
                    old_id,
                    new_id
                );
                crate::commands::migration::apply(&old_id, &new_id)?;
            }
            MigrateCommands::Rollback { migration_id } => {
                log::debug!("Command: migrate rollback | migration_id={}", migration_id);
                crate::commands::migration::rollback(&migration_id)?;
            }
            MigrateCommands::History { limit } => {
                log::debug!("Command: migrate history | limit={}", limit);
                crate::commands::migration::history(limit)?;
            }
        },
        Commands::Export {
            id,
            output,
            contract_dir,
            format,
            filters,
            page_size,
        } => {
            log::debug!(
                "Command: export | id={:?} output={:?} format={:?}",
                id,
                output,
                format
            );
            crate::commands::export::export(
                &cli.api_url,
                id.as_deref(),
                output.as_deref(),
                &contract_dir,
                format.as_deref(),
                filters,
                page_size,
            )
            .await?;
        }
        Commands::Import {
            file,
            format,
            output_dir,
            validate,
            dry_run,
        } => {
            let network = cli.network.as_deref();
            log::debug!(
                "Command: import | file={} format={:?} output_dir={} validate={} dry_run={}",
                file,
                format,
                output_dir,
                validate,
                dry_run
            );
            let opts = crate::commands::import::ImportOptions {
                api_url: &cli.api_url,
                file_path: &file,
                format: format.as_deref(),
                network_flag: network,
                output_dir: &output_dir,
                validate,
                dry_run,
                on_duplicate: crate::commands::import::OnDuplicate::Skip,
                network_map: std::collections::HashMap::new(),
                atomic: false,
                report_output: None,
            };
            crate::commands::import::run(opts).await?;
        }
        Commands::Doc {
            contract_path,
            output,
        } => {
            log::debug!(
                "Command: doc | contract_path={} output={}",
                contract_path,
                output
            );
            crate::commands::doc::doc(&contract_path, &output)?;
        }
        Commands::Openapi {
            contract_path,
            output,
            format,
        } => {
            log::debug!(
                "Command: openapi | contract_path={} output={} format={}",
                contract_path,
                output,
                format
            );
            crate::commands::openapi::openapi(&contract_path, &output, &format)?;
        }
        Commands::Deploy {} => {
            log::debug!("Command: deploy");
            crate::commands::deploy::run_interactive().await?;
        }
        Commands::VersionSemver { action } => match action {
            VersionCommands::List { contract_id } => {
                log::debug!("Command: version list | contract_id={}", contract_id);
                crate::commands::upgrade::version::list(&contract_id)?;
            }
            VersionCommands::Bump { current, level } => {
                log::debug!(
                    "Command: version bump | current={} level={}",
                    current,
                    level
                );
                let next = crate::commands::upgrade::version::bump(&current, &level)?;
                println!("Next version: {}", next.green().bold());
            }
        },
        Commands::Upgrade { action } => match action {
            UpgradeSubcommands::Analyze { old_wasm, new_wasm } => {
                log::debug!(
                    "Command: upgrade analyze | old={} new={}",
                    old_wasm,
                    new_wasm
                );
                crate::commands::upgrade::manager::analyze(&old_wasm, &new_wasm).await?;
            }
            UpgradeSubcommands::Apply {
                contract_id,
                new_wasm,
            } => {
                log::debug!(
                    "Command: upgrade apply | contract_id={} new={}",
                    contract_id,
                    new_wasm
                );
                crate::commands::upgrade::manager::apply(&contract_id, &new_wasm).await?;
            }
            UpgradeSubcommands::Rollback {
                contract_id,
                version,
            } => {
                log::debug!(
                    "Command: upgrade rollback | contract_id={} version={}",
                    contract_id,
                    version
                );
                crate::commands::upgrade::manager::rollback(&contract_id, &version).await?;
            }
            UpgradeSubcommands::Generate {
                old_id,
                new_id,
                language,
                output,
            } => {
                log::debug!(
                    "Command: upgrade generate | old={} new={} lang={}",
                    old_id,
                    new_id,
                    language
                );
                crate::commands::migration::generate_template(
                    &old_id,
                    &new_id,
                    &language,
                    output.as_deref(),
                )?;
            }
        },
        Commands::Wizard {} => {
            log::debug!("Command: wizard");
            crate::commands::wizard::run(&cli.api_url).await?;
        }
        Commands::History { search, limit } => {
            log::debug!("Command: history | search={:?} limit={}", search, limit);
            crate::commands::wizard::show_history(search.as_deref(), limit)?;
        }
        Commands::Incident { action } => match action {
            IncidentCommands::Trigger {
                contract_id,
                severity,
            } => {
                log::debug!(
                    "Command: incident trigger | contract_id={} severity={}",
                    contract_id,
                    severity
                );
                crate::commands::incident::incident_trigger(&contract_id, &severity)?;
            }
            IncidentCommands::Update { incident_id, state } => {
                log::debug!(
                    "Command: incident update | incident_id={} state={}",
                    incident_id,
                    state
                );
                crate::commands::incident::incident_update(&incident_id, &state)?;
            }
        },
        Commands::Patch { action } => match action {
            PatchCommands::Create {
                version,
                hash,
                severity,
                rollout,
            } => {
                let sev = severity.parse::<Severity>()?;
                log::debug!(
                    "Command: patch create | version={} rollout={}",
                    version,
                    rollout
                );
                crate::commands::patch::patch_create(&cli.api_url, &version, &hash, sev, rollout)
                    .await?;
            }
            PatchCommands::Notify { patch_id } => {
                log::debug!("Command: patch notify | patch_id={}", patch_id);
                crate::commands::patch::patch_notify(&cli.api_url, &patch_id).await?;
            }
            PatchCommands::Apply {
                contract_id,
                patch_id,
            } => {
                log::debug!(
                    "Command: patch apply | contract_id={} patch_id={}",
                    contract_id,
                    patch_id
                );
                crate::commands::patch::patch_apply(&cli.api_url, &contract_id, &patch_id).await?;
            }
            PatchCommands::Deps { command } => match command {
                DepsCommands::List { contract_id } => {
                    crate::commands::deps::deps_list(&cli.api_url, &contract_id).await?;
                }
            },
        },
        // ── Multi-sig commands (issue #47) ───────────────────────────────────
        Commands::Multisig { action } => match action {
            MultisigCommands::CreatePolicy {
                name,
                threshold,
                signers,
                expiry_secs,
                created_by,
            } => {
                let signer_vec: Vec<String> =
                    signers.split(',').map(|s| s.trim().to_string()).collect();
                log::debug!(
                    "Command: multisig create-policy | name={} threshold={} signers={:?}",
                    name,
                    threshold,
                    signer_vec
                );
                crate::commands::multisig::create_policy(
                    &cli.api_url,
                    &name,
                    threshold,
                    signer_vec,
                    expiry_secs,
                    &created_by,
                )
                .await?;
            }
            MultisigCommands::CreateProposal {
                contract_name,
                contract_id,
                wasm_hash,
                network: net_str,
                policy_id,
                proposer,
                description,
            } => {
                log::debug!(
                    "Command: multisig create-proposal | contract_id={} policy_id={}",
                    contract_id,
                    policy_id
                );
                crate::commands::multisig::create_proposal(
                    &cli.api_url,
                    &contract_name,
                    &contract_id,
                    &wasm_hash,
                    &net_str,
                    &policy_id,
                    &proposer,
                    description.as_deref(),
                )
                .await?;
            }
            MultisigCommands::Sign {
                proposal_id,
                signer,
                signature_data,
            } => {
                log::debug!("Command: multisig sign | proposal_id={}", proposal_id);
                crate::commands::multisig::sign_proposal(
                    &cli.api_url,
                    &proposal_id,
                    &signer,
                    signature_data.as_deref(),
                )
                .await?;
            }
            MultisigCommands::Execute { proposal_id } => {
                log::debug!("Command: multisig execute | proposal_id={}", proposal_id);
                crate::commands::multisig::execute_proposal(&cli.api_url, &proposal_id).await?;
            }
            MultisigCommands::Info { proposal_id } => {
                log::debug!("Command: multisig info | proposal_id={}", proposal_id);
                crate::commands::multisig::proposal_info(&cli.api_url, &proposal_id).await?;
            }
            MultisigCommands::ListProposals { status, limit } => {
                log::debug!(
                    "Command: multisig list-proposals | status={:?} limit={}",
                    status,
                    limit
                );
                crate::commands::multisig::list_proposals(&cli.api_url, status.as_deref(), limit)
                    .await?;
            }
        },
        Commands::Fuzz {
            contract_path,
            duration,
            timeout,
            threads,
            max_cases,
            output,
            minimize,
        } => {
            crate::commands::fuzz::run_fuzzer(
                &contract_path,
                &duration.to_string(),
                &timeout.to_string(),
                threads as usize,
                max_cases as u64,
                &output,
                minimize,
            )
            .await?;
        }
        Commands::Perf {
            contract_path,
            method,
            output,
            flamegraph,
            compare,
            recommendations,
        } => {
            log::debug!(
                "Command: perf | contract_path={} method={:?} output={:?} flamegraph={:?} compare={:?} recommendations={}",
                contract_path,
                method,
                output,
                flamegraph,
                compare,
                recommendations
            );
            crate::commands::perf::profile(
                &contract_path,
                method.as_deref(),
                output.as_deref(),
                flamegraph.as_deref(),
                compare.as_deref(),
                recommendations,
            )?;
        }
        // ── User profile management (#841) ───────────────────────────────────
        Commands::Profile { action } => match action {
            ProfileCommands::View { address, json } => {
                log::debug!(
                    "Command: profile view | address={:?} json={}",
                    address,
                    json
                );
                crate::commands::profile::view(&cli.api_url, address.as_deref(), json).await?;
            }
            ProfileCommands::Edit {
                name,
                bio,
                website,
                email,
                github,
                avatar,
            } => {
                log::debug!("Command: profile edit");
                crate::commands::profile::edit(
                    &cli.api_url,
                    name.as_deref(),
                    bio.as_deref(),
                    website.as_deref(),
                    email.as_deref(),
                    github.as_deref(),
                    avatar.as_deref(),
                )
                .await?;
            }
            ProfileCommands::Update { field, value } => {
                log::debug!("Command: profile update | field={} value={}", field, value);
                crate::commands::profile::update_field(&cli.api_url, &field, &value).await?;
            }
            ProfileCommands::ListContracts {
                address,
                limit,
                format,
                json,
            } => {
                log::debug!(
                    "Command: profile list-contracts | address={:?} limit={} format={}",
                    address,
                    limit,
                    format
                );
                crate::commands::profile::list_contracts(
                    &cli.api_url,
                    address.as_deref(),
                    limit,
                    &format,
                    json,
                )
                .await?;
            }
            ProfileCommands::Export { address, format } => {
                log::debug!(
                    "Command: profile export | address={:?} format={}",
                    address,
                    format
                );
                crate::commands::profile::export(&cli.api_url, address.as_deref(), &format).await?;
            }
        },
        Commands::Test {
            test_file,
            contract_path,
            test_command,
            junit,
            coverage,
            require_coverage,
            coverage_threshold,
            setup_hook,
            teardown_hook,
            mock_config,
            report,
            profile_output,
            load_iterations,
        } => {
            crate::commands::test::run_test_suite(crate::commands::test::TestSuiteOptions {
                test_file: test_file.as_deref(),
                contract_path: contract_path.as_deref().unwrap_or("."),
                test_command: test_command.as_deref(),
                junit_output: junit.as_deref(),
                show_coverage: coverage,
                // Verbosity comes from the global -v/--verbose flag; a local `--verbose`
                // here collided with it and made every `test` invocation panic.
                verbose: cli.verbose > 0,
                require_coverage,
                coverage_threshold,
                setup_hook: setup_hook.as_deref(),
                teardown_hook: teardown_hook.as_deref(),
                mock_config: mock_config.as_deref(),
                report_output: report.as_deref(),
                profile_output: profile_output.as_deref(),
                load_iterations,
            })
            .await?;
        }
        Commands::Audit {
            contract_path,
            format,
            output,
            fail_on,
        } => {
            log::debug!(
                "Command: audit | contract_path={} format={} output={:?} fail_on={:?}",
                contract_path,
                format,
                output,
                fail_on
            );
            crate::commands::audit::run(
                &contract_path,
                &format,
                output.as_deref(),
                fail_on.as_deref(),
            )?;
        }
        Commands::Sla { action } => match action {
            SlaCommands::Record {
                id,
                uptime,
                latency,
                error_rate,
            } => {
                log::debug!(
                    "Command: sla record | id={} uptime={} latency={} error_rate={}",
                    id,
                    uptime,
                    latency,
                    error_rate
                );
                crate::commands::sla::sla_record(&id, uptime, latency, error_rate)?;
            }
            SlaCommands::Status { id } => {
                log::debug!("Command: sla status | id={}", id);
                crate::commands::sla::sla_status(&id)?;
            }
        },
        Commands::Config { action } => match action {
            ConfigSubcommands::UserGet { key } => {
                crate::config::user::validate_key(&key)?;
                let value = crate::config::user::get_key(&key)?;
                match value {
                    Some(v) => println!("{}", v),
                    None => anyhow::bail!("Key '{}' was not found in user config.", key),
                }
            }
            ConfigSubcommands::UserSet { key, value } => {
                crate::config::user::set_key(&key, &value)?;
                println!("Updated '{}' in user config.", key);
            }
            ConfigSubcommands::UserList {} => {
                let cfg = crate::config::user::list()?;
                println!("{}", serde_json::to_string_pretty(&cfg)?);
            }
            ConfigSubcommands::UserReset {} => {
                let cfg = crate::config::user::reset_to_defaults()?;
                println!("User config reset to defaults:");
                println!("{}", serde_json::to_string_pretty(&cfg)?);
            }
            ConfigSubcommands::ContractGet {
                contract_id,
                environment,
            } => {
                crate::commands::config::config_get(&cli.api_url, &contract_id, &environment)
                    .await?;
            }
            ConfigSubcommands::ContractSet {
                contract_id,
                environment,
                config_data,
                secrets_data,
                created_by,
            } => {
                crate::commands::config::config_set(
                    &cli.api_url,
                    &contract_id,
                    &environment,
                    &config_data,
                    secrets_data.as_deref(),
                    &created_by,
                )
                .await?;
            }
            ConfigSubcommands::ContractHistory {
                contract_id,
                environment,
            } => {
                crate::commands::config::config_history(&cli.api_url, &contract_id, &environment)
                    .await?;
            }
            ConfigSubcommands::ContractRollback {
                contract_id,
                environment,
                version,
                created_by,
            } => {
                crate::commands::config::config_rollback(
                    &cli.api_url,
                    &contract_id,
                    &environment,
                    version,
                    &created_by,
                )
                .await?;
            }
        },
        Commands::Auth { action } => match action {
            AuthCommands::Login {
                method,
                identity,
                secret,
                scopes,
                expires,
            } => {
                let method = match method {
                    Some(method) => method,
                    None => {
                        let selected = crate::commands::wizard::prompt_with_validation(
                            "Authentication method [github|stellar|api-key]",
                            Some("stellar".to_string()),
                            |value| {
                                matches!(
                                    value.trim().to_ascii_lowercase().as_str(),
                                    "github" | "stellar" | "api-key"
                                )
                            },
                            "Choose github, stellar, or api-key.",
                        )?;
                        match selected.trim().to_ascii_lowercase().as_str() {
                            "github" => crate::commands::auth::AuthMethod::Github,
                            "stellar" => crate::commands::auth::AuthMethod::Stellar,
                            "api-key" => crate::commands::auth::AuthMethod::ApiKey,
                            _ => unreachable!(),
                        }
                    }
                };
                log::debug!(
                    "Command: auth login | method={} identity={:?} scopes={:?} expires={:?}",
                    method,
                    identity,
                    scopes,
                    expires
                );
                crate::commands::auth::login(
                    &cli.api_url,
                    method,
                    identity.as_deref(),
                    secret.as_deref(),
                    scopes,
                    expires.as_deref(),
                )
                .await?;
            }
            AuthCommands::Logout {} => {
                log::debug!("Command: auth logout");
                crate::commands::auth::logout()?;
            }
            AuthCommands::Status {} => {
                log::debug!("Command: auth status");
                crate::commands::auth::status(&cli.api_url).await?;
            }
            AuthCommands::Token { scopes, expires } => {
                log::debug!(
                    "Command: auth token | scopes={:?} expires={:?}",
                    scopes,
                    expires
                );
                crate::commands::auth::token(&cli.api_url, scopes, expires.as_deref()).await?;
            }
        },
        Commands::Backup { action } => match action {
            BackupCommands::Create {
                contract_id,
                include_state,
            } => {
                crate::commands::backup::create_backup(&cli.api_url, &contract_id, include_state)
                    .await?;
            }
            BackupCommands::List { contract_id } => {
                crate::commands::backup::list_backups(&cli.api_url, &contract_id).await?;
            }
            BackupCommands::Restore {
                contract_id,
                backup_date,
            } => {
                crate::commands::backup::restore_backup(&cli.api_url, &contract_id, &backup_date)
                    .await?;
            }
            BackupCommands::Verify {
                contract_id,
                backup_date,
            } => {
                crate::commands::backup::verify_backup(&cli.api_url, &contract_id, &backup_date)
                    .await?;
            }
            BackupCommands::Stats { contract_id } => {
                crate::commands::backup::backup_stats(&cli.api_url, &contract_id).await?;
            }
        },
        Commands::State { action } => match action {
            StateSubcommands::Get {
                contract_id,
                key,
                json,
            } => {
                crate::commands::state::get::state_get(
                    &cli.api_url,
                    &contract_id,
                    &key,
                    network,
                    json,
                )
                .await?;
            }
            StateSubcommands::Set {
                contract_id,
                key,
                value,
                json,
            } => {
                crate::commands::state::set::state_set(
                    &cli.api_url,
                    &contract_id,
                    &key,
                    &value,
                    network,
                    json,
                )
                .await?;
            }
            StateSubcommands::Dump { contract_id, json } => {
                crate::commands::state::dump::state_dump(&contract_id, network, json)?;
            }
            StateSubcommands::Snapshot {
                contract_id,
                label,
                json,
            } => {
                crate::commands::state::snapshot::state_snapshot_create(
                    &contract_id,
                    network,
                    label.as_deref(),
                    json,
                )?;
            }
            StateSubcommands::Snapshots {
                contract_id,
                limit,
                json,
            } => {
                crate::commands::state::snapshot::state_snapshot_list(
                    &contract_id,
                    network,
                    limit,
                    json,
                )?;
            }
            StateSubcommands::History {
                contract_id,
                key,
                limit,
                json,
            } => {
                crate::commands::state::history::state_history(
                    &contract_id,
                    network,
                    key.as_deref(),
                    limit,
                    json,
                )?;
            }
        },
        Commands::VerifyFormal {
            contract_path,
            properties,
            output,
            post,
        } => {
            crate::commands::formal_verification::run(
                &cli.api_url,
                &contract_path,
                &properties,
                &output,
                post,
            )
            .await?;
        }
        Commands::ScanDeps {
            contract_id,
            dependencies,
            fail_on_high,
        } => {
            crate::commands::scan_deps::scan_deps(
                &cli.api_url,
                &contract_id,
                &dependencies,
                fail_on_high,
            )
            .await?;
        }
        Commands::Coverage {
            contract_path,
            tests,
            threshold,
            output,
        } => {
            crate::commands::coverage::run(&contract_path, &tests, threshold, &output).await?;
        }
        Commands::Sign {
            package,
            private_key,
            contract_id,
            version,
            expires_at,
        } => {
            log::debug!(
                "Command: sign | package={} contract_id={} version={}",
                package,
                contract_id,
                version
            );
            crate::commands::package_signing::sign_package(
                &cli.api_url,
                &package,
                &private_key,
                &contract_id,
                &version,
                expires_at.as_deref(),
            )
            .await?;
        }
        Commands::VerifyPackage {
            package,
            contract_id,
            version,
            signature,
        } => {
            log::debug!(
                "Command: verify-package | package={} contract_id={}",
                package,
                contract_id
            );
            crate::commands::package_signing::verify_package(
                &cli.api_url,
                &package,
                &contract_id,
                version.as_deref(),
                signature.as_deref(),
            )
            .await?;
        }
        Commands::Verify {
            id,
            submit,
            check,
            history,
            level,
            json,
            path,
            notes,
        } => {
            log::debug!(
                "Command: verify | id={:?} submit={} check={}",
                id,
                submit,
                check
            );
            crate::commands::verification::run(
                &cli.api_url,
                id,
                submit,
                check,
                history,
                level,
                json,
                &path,
                notes,
            )
            .await?;
        }
        Commands::VerifyContract {
            wasm_path,
            contract_id,
            version,
            signature,
            public_key,
        } => {
            log::debug!(
                "Command: verify-contract | wasm_path={} contract_id={} version={}",
                wasm_path,
                contract_id,
                version
            );
            crate::commands::package_signing::verify_contract_local(
                &wasm_path,
                &contract_id,
                &version,
                &signature,
                &public_key,
            )?;
        }
        Commands::Keys { action } => match action {
            KeysCommands::Generate {} => {
                log::debug!("Command: keys generate");
                crate::commands::package_signing::generate_keypair()?;
            }
            KeysCommands::Revoke {
                signature_id,
                revoked_by,
                reason,
            } => {
                log::debug!("Command: keys revoke | signature_id={}", signature_id);
                crate::commands::package_signing::revoke_signature(
                    &cli.api_url,
                    &signature_id,
                    &revoked_by,
                    &reason,
                )
                .await?;
            }
            KeysCommands::Custody { contract_id } => {
                log::debug!("Command: keys custody | contract_id={}", contract_id);
                crate::commands::package_signing::get_chain_of_custody(&cli.api_url, &contract_id)
                    .await?;
            }
            KeysCommands::Log {
                contract_id,
                entry_type,
                limit,
            } => {
                log::debug!("Command: keys log");
                crate::commands::package_signing::get_transparency_log(
                    &cli.api_url,
                    contract_id.as_deref(),
                    entry_type.as_deref(),
                    limit,
                )
                .await?;
            }
        },
        Commands::BatchVerify {
            file,
            contracts,
            network,
            category,
            age,
            initiated_by,
            level,
            export,
            output,
            schedule,
            json,
        } => {
            log::debug!(
                "Command: batch-verify | contracts={:?} initiated_by={}",
                contracts,
                initiated_by
            );
            crate::commands::batch::verify::run_batch_verify(
                crate::commands::batch::verify::BatchVerifyArgs {
                    api_url: &cli.api_url,
                    file: file.as_deref(),
                    contracts: contracts.as_deref(),
                    network: network.as_deref(),
                    category: category.as_deref(),
                    age,
                    initiated_by: &initiated_by,
                    level: &level,
                    export: export.as_deref(),
                    output: output.as_deref(),
                    schedule: schedule.as_deref(),
                    json,
                },
            )
            .await?;
        }
        Commands::Webhook { action } => match action {
            WebhookCommands::Create {
                url,
                events,
                secret,
            } => {
                let event_list: Vec<String> =
                    events.split(',').map(|s| s.trim().to_string()).collect();
                log::debug!(
                    "Command: webhook create | url={} events={:?}",
                    url,
                    event_list
                );
                crate::commands::webhook::create_webhook(
                    &cli.api_url,
                    &url,
                    event_list,
                    secret.as_deref(),
                )
                .await?;
            }
            WebhookCommands::List {} => {
                log::debug!("Command: webhook list");
                crate::commands::webhook::list_webhooks(&cli.api_url).await?;
            }
            WebhookCommands::Delete { webhook_id } => {
                log::debug!("Command: webhook delete | id={}", webhook_id);
                crate::commands::webhook::delete_webhook(&cli.api_url, &webhook_id).await?;
            }
            WebhookCommands::Test { webhook_id } => {
                log::debug!("Command: webhook test | id={}", webhook_id);
                crate::commands::webhook::test_webhook(&cli.api_url, &webhook_id).await?;
            }
            WebhookCommands::Logs { webhook_id, limit } => {
                log::debug!("Command: webhook logs | id={} limit={}", webhook_id, limit);
                crate::commands::webhook::webhook_logs(&cli.api_url, &webhook_id, limit).await?;
            }
            WebhookCommands::Retry { delivery_id } => {
                log::debug!("Command: webhook retry | delivery_id={}", delivery_id);
                crate::commands::webhook::retry_delivery(&cli.api_url, &delivery_id).await?;
            }
            WebhookCommands::VerifySig {
                secret,
                payload,
                signature,
            } => {
                log::debug!("Command: webhook verify-sig");
                crate::commands::webhook::verify_signature_cmd(&secret, &payload, &signature)?;
            }
        },
        // ── Contract verify command (#522) ───────────────────────────────────
        Commands::Contract { action } => match action {
            ContractCommands::List {
                limit,
                offset,
                networks,
                category,
                format,
            } => {
                log::debug!(
                    "Command: contract list | limit={} offset={} networks={:?} category={:?} format={}",
                    limit,
                    offset,
                    networks,
                    category,
                    format
                );
                crate::commands::contract::list::run(
                    &cli.api_url,
                    crate::commands::contract::list::ListOptions {
                        limit,
                        offset,
                        networks,
                        category,
                        format,
                    },
                )
                .await?;
            }
            ContractCommands::Search {
                query,
                networks,
                category,
                tags,
                verified_only,
                limit,
                offset,
                cursor,
                pagination,
                all,
                max_items,
                max_pages,
                json,
            } => {
                log::debug!(
                    "Command: contract search | query={:?} all={} limit={} offset={:?} cursor={} pagination={:?}",
                    query,
                    all,
                    limit,
                    offset,
                    cursor.is_some(),
                    pagination
                );
                crate::commands::contract::search::run(
                    &cli.api_url,
                    crate::support::search_pagination::SearchOptions {
                        query,
                        networks,
                        category,
                        tags,
                        verified_only,
                        limit,
                        offset,
                        cursor,
                        pagination,
                        all,
                        max_items,
                        max_pages,
                        json,
                    },
                )
                .await?;
            }
            ContractCommands::Register { file, batch, json } => {
                log::debug!(
                    "Command: contract register | file={:?} batch={} json={}",
                    file,
                    batch,
                    json
                );
                crate::commands::contract::register::run(
                    &cli.api_url,
                    cfg_network,
                    file.as_deref(),
                    batch,
                    json,
                )
                .await?;
            }
            ContractCommands::Verify {
                address,
                wasm,
                network,
                json,
                strict,
                batch,
                no_cache,
            } => {
                // Local verbosity is driven by the global -v/--verbose flag.
                let verbose = cli.verbose > 0;
                // Exactly one of --wasm (local) or <address> (on-chain) is required.
                match (wasm, address) {
                    (Some(_), Some(_)) => {
                        anyhow::bail!(
                            "Pass either a contract <address> (on-chain verification) or \
                             --wasm <path> (local verification), not both."
                        );
                    }
                    (Some(wasm_path), None) => {
                        log::debug!(
                            "Command: contract verify (local) | wasm={} verbose={} json={}",
                            wasm_path,
                            verbose,
                            json
                        );
                        crate::commands::contract::verify::run_local(&wasm_path, verbose, json)
                            .await?;
                    }
                    (None, Some(address)) => {
                        log::debug!(
                            "Command: contract verify | address={} network={} json={} strict={} batch={} no_cache={}",
                            address,
                            network,
                            json,
                            strict,
                            batch,
                            no_cache
                        );
                        crate::commands::contract::verify::run(
                            &cli.api_url,
                            &address,
                            &network,
                            json,
                            strict,
                            batch,
                            no_cache,
                        )
                        .await?;
                    }
                    (None, None) => {
                        anyhow::bail!(
                            "Provide a contract <address> to verify on-chain, or --wasm <path> \
                             to verify a local artifact before publishing."
                        );
                    }
                }
            }
            ContractCommands::Interfaces { wasm, json } => {
                log::debug!("Command: contract interfaces | wasm={} json={}", wasm, json);
                crate::commands::contract::interfaces::run_local(&wasm, json).await?;
            }
            ContractCommands::Provenance { manifest, json } => {
                log::debug!(
                    "Command: contract provenance | manifest={} json={}",
                    manifest,
                    json
                );
                crate::commands::contract::provenance::run_local(&manifest, json).await?;
            }
            ContractCommands::VerifyBuild {
                manifest,
                source_dir,
                expected_hash,
                allow_toolchain_mismatch,
                json,
            } => {
                log::debug!(
                    "Command: contract verify-build | manifest={} source_dir={} json={}",
                    manifest,
                    source_dir,
                    json
                );
                crate::commands::contract::verify_build::run(
                    &manifest,
                    &source_dir,
                    &expected_hash,
                    allow_toolchain_mismatch,
                    json,
                )
                .await?;
            }
            ContractCommands::Compatibility {
                from,
                to,
                from_network_passphrase,
                to_network_passphrase,
                strict,
                fail_on,
                json,
            } => {
                log::debug!(
                    "Command: contract compatibility | from={} to={} strict={} fail_on={} json={}",
                    from,
                    to,
                    strict,
                    fail_on,
                    json
                );
                let fail_on = crate::commands::contract::compatibility::FailOn::parse(&fail_on)?;
                crate::commands::contract::compatibility::run(
                    &from,
                    &to,
                    from_network_passphrase,
                    to_network_passphrase,
                    strict,
                    json,
                    fail_on,
                )
                .await?;
            }
            ContractCommands::Details {
                address,
                network,
                json,
            } => {
                log::debug!(
                    "Command: contract details | address={} network={} json={}",
                    address,
                    network,
                    json
                );
                crate::commands::contract::info::run_details(
                    &cli.api_url,
                    &address,
                    &network,
                    json,
                )
                .await?;
            }
            ContractCommands::Deploy {
                wasm_path,
                name,
                description,
                category,
                network,
                icon,
                interactive,
                publisher,
                tags,
                skip_abi,
                json,
            } => {
                log::debug!(
                    "Command: contract deploy | wasm_path={} network={} interactive={}",
                    wasm_path,
                    network,
                    interactive
                );
                crate::commands::contract::deploy::run_deploy(
                    &cli.api_url,
                    &wasm_path,
                    name.as_deref(),
                    description.as_deref(),
                    category.as_deref(),
                    &network,
                    icon.as_deref(),
                    interactive,
                    publisher.as_deref(),
                    tags.as_deref(),
                    skip_abi,
                    json,
                )
                .await?;
            }
            ContractCommands::Risk {
                address,
                network,
                threshold,
                json,
            } => {
                log::debug!(
                    "Command: contract risk | address={} network={} threshold={:?} json={}",
                    address,
                    network,
                    threshold,
                    json
                );
                crate::commands::contract::risk::run(
                    &cli.api_url,
                    &address,
                    &network,
                    threshold.as_deref(),
                    json,
                )
                .await?;
            }
            ContractCommands::Stats {
                network,
                category,
                top_n,
                format,
                output,
                compare,
            } => {
                log::debug!(
                    "Command: contract stats | network={:?} category={:?} format={}",
                    network,
                    category,
                    format
                );
                crate::commands::contract::stats::contract_stats(
                    &cli.api_url,
                    network.as_deref(),
                    category.as_deref(),
                    top_n,
                    &format,
                    output.as_deref(),
                    compare.as_deref(),
                )
                .await?;
            }
            ContractCommands::Export {
                output_file,
                output,
                format,
                network,
                category,
                since,
                compress,
                include_related,
                page_size,
            } => {
                let resolved_output = output.or(output_file);
                log::debug!(
                    "Command: contract export | output={:?} format={} network={:?} category={:?}",
                    resolved_output,
                    format,
                    network,
                    category
                );
                crate::commands::contract::export::contract_export(
                    &cli.api_url,
                    resolved_output.as_deref(),
                    &format,
                    network.as_deref(),
                    category.as_deref(),
                    since.as_deref(),
                    compress,
                    include_related,
                    page_size,
                )
                .await?;
            }
            ContractCommands::Highlight {
                address,
                action,
                token,
                json,
            } => {
                log::debug!("Command: contract highlight | action={}", action);
                crate::commands::contract::highlight::run(
                    &cli.api_url,
                    address.as_deref(),
                    &action,
                    token.as_deref(),
                    json,
                )
                .await?;
            }
            ContractCommands::Interaction {
                address,
                limit,
                json,
            } => {
                log::debug!("Command: contract interaction | address={}", address);
                crate::commands::contract::interaction::run(&cli.api_url, &address, limit, json)
                    .await?;
            }
            ContractCommands::Dependencies {
                address,
                network,
                transitive,
                depth,
                include_telemetry,
                json,
            } => {
                log::debug!(
                    "Command: contract dependencies | address={} network={:?} transitive={}",
                    address,
                    network,
                    transitive
                );
                let opts = crate::commands::contract::dependency_graph::GraphOptions {
                    network,
                    depth,
                    transitive,
                    include_telemetry,
                    json,
                };
                crate::commands::contract::dependency_graph::dependencies(
                    &cli.api_url,
                    &address,
                    &opts,
                )
                .await?;
            }
            ContractCommands::Dependents {
                address,
                network,
                transitive,
                depth,
                include_telemetry,
                json,
            } => {
                log::debug!(
                    "Command: contract dependents | address={} network={:?} transitive={}",
                    address,
                    network,
                    transitive
                );
                let opts = crate::commands::contract::dependency_graph::GraphOptions {
                    network,
                    depth,
                    transitive,
                    include_telemetry,
                    json,
                };
                crate::commands::contract::dependency_graph::dependents(
                    &cli.api_url,
                    &address,
                    &opts,
                )
                .await?;
            }
            ContractCommands::DependencyRisk {
                address,
                network,
                depth,
                fail_on,
                json,
            } => {
                log::debug!(
                    "Command: contract dependency-risk | address={} network={:?} fail_on={:?}",
                    address,
                    network,
                    fail_on
                );
                let threshold = fail_on
                    .as_deref()
                    .map(crate::commands::contract::dependency_graph::Severity::parse)
                    .transpose()?;
                let opts = crate::commands::contract::dependency_graph::GraphOptions {
                    network,
                    depth,
                    // Risk is only meaningful over the closure: a direct-only
                    // report would omit exactly the inherited findings this
                    // command exists to surface.
                    transitive: true,
                    include_telemetry: false,
                    json,
                };
                let breached = crate::commands::contract::dependency_graph::risk(
                    &cli.api_url,
                    &address,
                    &opts,
                    threshold,
                )
                .await?;
                if breached {
                    std::process::exit(1);
                }
            }
            ContractCommands::Dependency {
                address,
                depth,
                format,
                summary,
            } => {
                log::debug!(
                    "Command: contract dependency | address={} depth={}",
                    address,
                    depth
                );
                let fmt = crate::support::output_format::validate_format(&format)
                    .unwrap_or(crate::support::output_format::OutputFormat::Table);
                crate::commands::contract::dependency::run(
                    &cli.api_url,
                    &address,
                    depth,
                    fmt,
                    summary,
                )
                .await?;
            }
            ContractCommands::Category { action } => match action {
                CategoryCommands::List {
                    network,
                    format,
                    export,
                } => {
                    log::debug!(
                        "Command: contract category list | network={:?} format={}",
                        network,
                        format
                    );
                    let fmt = crate::support::output_format::validate_format(&format)?;
                    crate::commands::category::list(
                        &cli.api_url,
                        network.as_deref(),
                        fmt,
                        export.as_deref(),
                    )
                    .await?;
                }
                CategoryCommands::Stats {
                    network,
                    format,
                    export,
                } => {
                    log::debug!(
                        "Command: contract category stats | network={:?} format={}",
                        network,
                        format
                    );
                    let fmt = crate::support::output_format::validate_format(&format)?;
                    crate::commands::category::stats(
                        &cli.api_url,
                        network.as_deref(),
                        fmt,
                        export.as_deref(),
                    )
                    .await?;
                }
            },
            ContractCommands::Update {
                address,
                name,
                description,
                category,
                tags,
                icon,
                homepage,
                dry_run,
                yes,
                json,
            } => {
                let tags_vec = tags.map(|t| {
                    t.split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<_>>()
                });
                crate::commands::contract::update::run(
                    crate::commands::contract::update::UpdateArgs {
                        api_url: &cli.api_url,
                        address: &address,
                        name,
                        description,
                        category,
                        tags: tags_vec,
                        icon,
                        homepage,
                        dry_run,
                        yes,
                        json,
                    },
                )
                .await?;
            }
            ContractCommands::Import {
                input_file,
                format,
                on_duplicate,
                network_map,
                dry_run,
                validate,
                atomic,
                report_output,
                output_dir,
            } => {
                log::debug!(
                    "Command: contract import | file={} format={:?} on_duplicate={} dry_run={} validate={} atomic={}",
                    input_file,
                    format,
                    on_duplicate,
                    dry_run,
                    validate,
                    atomic
                );
                let dup_strategy = crate::commands::import::OnDuplicate::parse(&on_duplicate)?;
                let net_map = crate::commands::import::parse_network_map(&network_map)?;
                let opts = crate::commands::import::ImportOptions {
                    api_url: &cli.api_url,
                    file_path: &input_file,
                    format: format.as_deref(),
                    network_flag: cli.network.as_deref(),
                    output_dir: &output_dir,
                    validate,
                    dry_run,
                    on_duplicate: dup_strategy,
                    network_map: net_map,
                    atomic,
                    report_output,
                };
                crate::commands::import::run(opts).await?;
            }
            ContractCommands::Audit {
                lockfile,
                fix,
                init,
                contracts,
                format,
                json,
            } => {
                log::debug!(
                    "Command: contract audit | lockfile={} fix={} init={} contracts={:?}",
                    lockfile,
                    fix,
                    init,
                    contracts
                );
                let fmt = if json { "json" } else { &format };
                crate::commands::contract::audit::run(
                    &cli.api_url,
                    &lockfile,
                    fix,
                    init,
                    &contracts,
                    fmt,
                )
                .await?;
            }
            ContractCommands::Snapshot { id, output, json } => {
                log::debug!("Command: contract snapshot | id={} output={}", id, output);
                crate::commands::contract::snapshot::run_export(&cli.api_url, &id, &output, json)
                    .await?;
            }

            ContractCommands::VerifySnapshot {
                file,
                expect_key,
                max_age_days,
                fetch_key,
                json,
            } => {
                log::debug!("Command: contract verify-snapshot | file={}", file);
                crate::commands::contract::snapshot::run_verify(
                    &cli.api_url,
                    &file,
                    expect_key.as_deref(),
                    max_age_days,
                    fetch_key,
                    json,
                )
                .await?;
            }

            ContractCommands::Deprecate {
                address,
                reason,
                replacement,
                private_key,
                migration_guide,
                grace_period_days,
                yes,
                json,
            } => {
                log::debug!(
                    "Command: contract deprecate | address={} reason={} replacement={:?}",
                    address,
                    reason,
                    replacement
                );
                crate::commands::contract::deprecate::run(
                    &cli.api_url,
                    &address,
                    &reason,
                    replacement.as_deref(),
                    &private_key,
                    migration_guide.as_deref(),
                    grace_period_days,
                    yes,
                    json,
                )
                .await?;
            }
            ContractCommands::Rollback {
                address,
                reason,
                private_key,
                yes,
                json,
            } => {
                log::debug!(
                    "Command: contract rollback | address={} reason={}",
                    address,
                    reason
                );
                crate::commands::contract::deprecate::rollback(
                    &cli.api_url,
                    &address,
                    &reason,
                    &private_key,
                    yes,
                    json,
                )
                .await?;
            }
            ContractCommands::Notification { action } => {
                /// Splits a comma-separated argument, dropping empty entries so
                /// `--alerts ""` means "none" rather than one empty alert type.
                fn split_list(value: &str) -> Vec<String> {
                    value
                        .split(',')
                        .map(|item| item.trim().to_string())
                        .filter(|item| !item.is_empty())
                        .collect()
                }

                match action {
                    NotificationCommands::Subscribe {
                        address,
                        alerts,
                        channels,
                        frequency,
                        networks,
                        categories,
                        target,
                    } => {
                        log::debug!("Command: contract notification subscribe | address={address}");
                        crate::commands::notification::subscribe(
                            &address,
                            split_list(&alerts),
                            split_list(&channels),
                            &frequency,
                            split_list(&networks),
                            split_list(&categories),
                            target,
                        )?;
                    }
                    NotificationCommands::Unsubscribe { address } => {
                        log::debug!(
                            "Command: contract notification unsubscribe | address={address}"
                        );
                        crate::commands::notification::unsubscribe(&address)?;
                    }
                    NotificationCommands::List { address, json } => {
                        log::debug!("Command: contract notification list");
                        crate::commands::notification::list(address.as_deref(), json)?;
                    }
                    NotificationCommands::Configure {
                        address,
                        alerts,
                        channels,
                        frequency,
                        networks,
                        categories,
                        target,
                    } => {
                        log::debug!("Command: contract notification configure | address={address}");
                        crate::commands::notification::configure(
                            &address,
                            alerts.as_deref().map(split_list),
                            channels.as_deref().map(split_list),
                            frequency,
                            networks.as_deref().map(split_list),
                            categories.as_deref().map(split_list),
                            target,
                        )?;
                    }
                    NotificationCommands::Test { address } => {
                        log::debug!("Command: contract notification test | address={address}");
                        crate::commands::notification::test_notification(&address)?;
                    }
                }
            }
            ContractCommands::Drift {
                id,
                all,
                status,
                lockfile,
                network,
                json,
            } => {
                log::debug!(
                    "Command: contract drift | id={:?} all={} status={:?} lockfile={:?}",
                    id,
                    all,
                    status,
                    lockfile
                );
                let net = network.or_else(|| cli.network.clone());
                let has_drift = crate::commands::contract::drift::run(
                    crate::commands::contract::drift::DriftCliOptions {
                        api_url: &cli.api_url,
                        id: id.as_deref(),
                        all,
                        status: status.as_deref(),
                        lockfile: lockfile.as_deref(),
                        network: net.as_deref(),
                        json,
                    },
                )
                .await?;
                if has_drift {
                    std::process::exit(1);
                }
            }
        },
        Commands::ApiKey { action } => match action {
            ApiKeyCommands::Create {
                expires,
                scopes,
                json,
            } => {
                log::debug!("Command: api-key create");
                crate::commands::api_key::create(
                    &cli.api_url,
                    expires.as_deref(),
                    scopes.as_deref(),
                    json,
                )
                .await?;
            }
            ApiKeyCommands::List { json } => {
                log::debug!("Command: api-key list");
                crate::commands::api_key::list(&cli.api_url, json).await?;
            }
            ApiKeyCommands::Delete { id, json } => {
                log::debug!("Command: api-key delete | id={}", id);
                crate::commands::api_key::delete(&cli.api_url, &id, false, json).await?;
            }
            ApiKeyCommands::Revoke { id, json } => {
                log::debug!("Command: api-key revoke | id={}", id);
                crate::commands::api_key::delete(&cli.api_url, &id, true, json).await?;
            }
        },
        // ── Release Notes commands ───────────────────────────────────────────
        Commands::ReleaseNotes { action } => match action {
            ReleaseNotesCommands::Generate {
                contract_id,
                version,
                previous_version,
                changelog,
                contract_address,
                json,
            } => {
                log::debug!(
                    "Command: release-notes generate | contract_id={} version={}",
                    contract_id,
                    version
                );
                crate::commands::release_notes::generate(
                    &cli.api_url,
                    &contract_id,
                    &version,
                    previous_version.as_deref(),
                    changelog.as_deref(),
                    contract_address.as_deref(),
                    json,
                )
                .await?;
            }
            ReleaseNotesCommands::View {
                contract_id,
                version,
                json,
            } => {
                log::debug!(
                    "Command: release-notes view | contract_id={} version={}",
                    contract_id,
                    version
                );
                crate::commands::release_notes::view(&cli.api_url, &contract_id, &version, json)
                    .await?;
            }
            ReleaseNotesCommands::Edit {
                contract_id,
                version,
                file,
                text,
                json,
            } => {
                log::debug!(
                    "Command: release-notes edit | contract_id={} version={}",
                    contract_id,
                    version
                );
                crate::commands::release_notes::edit(
                    &cli.api_url,
                    &contract_id,
                    &version,
                    file.as_deref(),
                    text.as_deref(),
                    json,
                )
                .await?;
            }
            ReleaseNotesCommands::Publish {
                contract_id,
                version,
                skip_version_update,
                json,
            } => {
                log::debug!(
                    "Command: release-notes publish | contract_id={} version={}",
                    contract_id,
                    version
                );
                crate::commands::release_notes::publish(
                    &cli.api_url,
                    &contract_id,
                    &version,
                    skip_version_update,
                    json,
                )
                .await?;
            }
            ReleaseNotesCommands::List { contract_id, json } => {
                log::debug!("Command: release-notes list | contract_id={}", contract_id);
                crate::commands::release_notes::list(&cli.api_url, &contract_id, json).await?;
            }
        },

        Commands::Cicd { action } => match action {
            CicdCommands::Run {
                contract_path,
                network,
                skip_scan,
                auto_register,
                json,
            } => {
                log::debug!(
                    "Command: cicd run | path={} network={}",
                    contract_path,
                    network
                );
                crate::commands::cicd::run_pipeline(
                    &cli.api_url,
                    &contract_path,
                    &network,
                    skip_scan,
                    auto_register,
                    json,
                )
                .await?;
            }
            CicdCommands::Validate { contract_path } => {
                log::debug!("Command: cicd validate | path={}", contract_path);
                crate::commands::cicd::validate_env(&contract_path).await?;
            }
        },

        // ── Network commands (issue #523) ────────────────────────────────────
        Commands::Network { action } => match action {
            NetworkCommands::Status { json } => {
                log::debug!("Command: network status");
                crate::commands::network::status(json).await?;
            }
        },

        // ── Advanced contract analysis (issue #530) ─────────────────────────
        Commands::Analyze {
            contract_id,
            network: net_str,
            report_format,
            output,
        } => {
            log::debug!(
                "Command: analyze | contract_id={} network={} format={}",
                contract_id,
                net_str,
                report_format
            );
            crate::commands::analyze::run(
                &cli.api_url,
                &contract_id,
                &net_str,
                &report_format,
                output.as_deref(),
            )
            .await?;
        }

        // ── Bulk contract registration (issue #525) ──────────────────────────
        Commands::BatchRegister {
            manifest,
            publisher,
            dry_run,
            json,
        } => {
            log::debug!(
                "Command: batch-register | manifest={} dry_run={} publisher={:?}",
                manifest,
                dry_run,
                publisher
            );
            crate::commands::batch::register::run_batch_register(
                &cli.api_url,
                &manifest,
                publisher.as_deref(),
                dry_run,
                json,
                // Documented defaults: None = sequential (1 at a time), no retry pass.
                // `batch-register` exposes no flags for these yet.
                None,
                false,
            )
            .await?;
        }
        Commands::BatchAudit {
            file,
            format,
            output_dir,
            fail_on,
            high_risk,
            profile,
            export,
            json,
        } => {
            log::debug!("Command: batch-audit | file={}", file);
            crate::commands::batch::audit::run_batch_audit(
                &file,
                &format,
                output_dir.as_deref(),
                fail_on.as_deref(),
                high_risk,
                &profile,
                export.as_deref(),
                json,
            )?;
        }
        Commands::BatchDeploy {
            wasm_file,
            networks,
            signer,
            atomic,
            json,
        } => {
            log::debug!("Command: batch-deploy | wasm={}", wasm_file);
            crate::commands::batch::deploy::run_batch_deploy(
                &wasm_file, &networks, &signer, atomic, json,
            )?;
        }
        Commands::BatchExport {
            output_dir,
            filter,
            format,
            organize,
            compress,
            json,
        } => {
            log::debug!("Command: batch-export | output_dir={}", output_dir);
            crate::commands::batch::export::run_batch_export(
                &cli.api_url,
                &output_dir,
                filter.as_deref(),
                &format,
                organize,
                compress,
                json,
            )
            .await?;
        }
        Commands::BatchUpdate {
            file,
            filter,
            preview,
            r#if: condition,
            user_id,
            rollback_on_error,
            json,
        } => {
            crate::commands::batch::update::run_batch_update(
                crate::commands::batch::update::BatchUpdateArgs {
                    api_url: &cli.api_url,
                    file: file.as_deref(),
                    filter: filter.as_deref(),
                    preview,
                    condition: condition.as_deref(),
                    user_id: user_id.as_deref(),
                    rollback_on_error,
                    json,
                },
            )
            .await?;
        }
        Commands::BatchImport {
            input_dir,
            format,
            on_duplicate,
            dry_run,
            atomic,
            output_dir,
            json,
        } => {
            log::debug!("Command: batch-import | input_dir={}", input_dir);
            crate::commands::batch::import::run_batch_import(
                &cli.api_url,
                &input_dir,
                format.as_deref(),
                &on_duplicate,
                dry_run,
                atomic,
                &output_dir,
                json,
            )
            .await?;
        }
        Commands::Batch {
            operation,
            contracts,
            file,
            value,
            rollback_on_error,
            recipients,
            message_type,
            template,
            preview,
            schedule,
            channels,
            filter,
            atomic,
            report,
            json,
        } => {
            if operation == "notify" {
                let recipients = recipients
                    .as_deref()
                    .ok_or_else(|| anyhow::anyhow!("batch notify requires --recipients"))?;
                let message = contracts.join(" ");
                crate::commands::batch::notify::run_batch_notify(
                    &cli.api_url,
                    &message,
                    recipients,
                    &message_type,
                    template.as_deref(),
                    preview,
                    schedule.as_deref(),
                    channels,
                    json,
                )
                .await?;
                return Ok(());
            }
            if operation == "migrate" {
                anyhow::ensure!(
                    contracts.len() >= 2,
                    "batch migrate requires SOURCE and DESTINATION"
                );
                crate::commands::batch::migrate::run_batch_migrate(
                    &contracts[0],
                    &contracts[1],
                    filter.as_deref(),
                    preview,
                    atomic,
                    report.as_deref(),
                    json,
                )
                .await?;
                return Ok(());
            }
            let op = crate::commands::batch::ops::BatchOperation::parse(&operation)?;
            crate::commands::batch::ops::run(
                &cli.api_url,
                op,
                contracts,
                file.as_deref(),
                value.as_deref(),
                rollback_on_error,
                json,
            )
            .await?;
        }
        // ── Local cache management (#845) ────────────────────────────────────
        Commands::Cache { action } => match action {
            CacheCommands::Clear { level, key } => {
                log::debug!("Command: cache clear | level={} key={:?}", level, key);
                crate::commands::cache::clear(&level, key.as_deref())?;
            }
            CacheCommands::Status { json } => {
                log::debug!("Command: cache status | json={}", json);
                crate::commands::cache::status(json)?;
            }
            CacheCommands::Configure {
                ttl,
                max_size,
                compression,
                auto_refresh,
                json,
            } => {
                log::debug!("Command: cache configure");
                crate::commands::cache::configure(
                    ttl,
                    max_size,
                    compression.as_deref(),
                    auto_refresh.as_deref(),
                    json,
                )?;
            }
            CacheCommands::Optimize { json } => {
                log::debug!("Command: cache optimize | json={}", json);
                crate::commands::cache::optimize(json)?;
            }
            CacheCommands::Export {
                format,
                include_stale,
            } => {
                log::debug!(
                    "Command: cache export | format={} include_stale={}",
                    format,
                    include_stale
                );
                crate::commands::cache::export(&format, include_stale)?;
            }
        },
        // ── Environment variable management (#843) ───────────────────────────
        Commands::Env { action } => match action {
            EnvCommands::Set {
                name,
                value,
                env,
                show_value,
            } => {
                log::debug!(
                    "Command: env set | name={} env={:?} show_value={}",
                    name,
                    env,
                    show_value
                );
                crate::commands::env::set_var(&name, &value, env.as_deref(), show_value)?;
            }
            EnvCommands::Get { name, env, json } => {
                log::debug!(
                    "Command: env get | name={} env={:?} json={}",
                    name,
                    env,
                    json
                );
                crate::commands::env::get_var(&name, env.as_deref(), json)?;
            }
            EnvCommands::List {
                env,
                all,
                merged,
                json,
            } => {
                log::debug!(
                    "Command: env list | env={:?} all={} merged={} json={}",
                    env,
                    all,
                    merged,
                    json
                );
                crate::commands::env::list_vars(env.as_deref(), all, merged, json)?;
            }
            EnvCommands::Copy {
                from,
                to,
                overwrite,
            } => {
                log::debug!(
                    "Command: env copy | from={} to={} overwrite={}",
                    from,
                    to,
                    overwrite
                );
                crate::commands::env::copy_env(&from, &to, overwrite)?;
            }
            EnvCommands::Delete { name, env } => {
                log::debug!("Command: env delete | name={} env={:?}", name, env);
                crate::commands::env::delete_var(&name, env.as_deref())?;
            }
            EnvCommands::Export {
                env,
                format,
                merged,
            } => {
                log::debug!(
                    "Command: env export | env={:?} format={:?} merged={}",
                    env,
                    format,
                    merged
                );
                crate::commands::env::export_env(env.as_deref(), format.as_str(), merged)?;
            }
            EnvCommands::Switch { environment } => {
                log::debug!("Command: env switch | environment={}", environment);
                crate::commands::env::switch_env(&environment)?;
            }
        },
        Commands::Publisher { action } => match action {
            PublisherCommands::Doctor { json } => {
                log::debug!("Command: publisher doctor | json={}", json);
                crate::commands::publisher::doctor(&cli.api_url, json).await?;
            }
        },
        Commands::Snapshot { action } => match action {
            SnapshotCommands::Export { output } => {
                log::debug!("Command: snapshot export | output={}", output);
                crate::commands::snapshot::export(&cli.api_url, &output).await?;
            }
            SnapshotCommands::Sign { snapshot_file, key } => {
                log::debug!(
                    "Command: snapshot sign | file={} key={}",
                    snapshot_file,
                    key
                );
                crate::commands::snapshot::sign(&snapshot_file, &key).await?;
            }
            SnapshotCommands::Verify {
                snapshot_file,
                trust_key,
            } => {
                log::debug!(
                    "Command: snapshot verify | file={} trust_key={}",
                    snapshot_file,
                    trust_key
                );
                crate::commands::snapshot::verify(&snapshot_file, &trust_key).await?;
            }
            SnapshotCommands::Inspect { snapshot_file } => {
                log::debug!("Command: snapshot inspect | file={}", snapshot_file);
                crate::commands::snapshot::inspect(&snapshot_file).await?;
            }
        },
    }

    Ok(())
}
