//! Operational preflight and dry-run planning surfaces.
//!
//! These commands are intentionally bounded. `preflight` reads accepted public
//! Server surfaces. Bootstrap, recovery, and rollout plans never mutate Server,
//! PostgreSQL, providers, deployment state, or local vault data.

use crate::{
    config::CliConfig,
    doctor_live::{self, DoctorReadClient},
    output::{CliExitCode, CliOutput},
    server_api::{self, ReadCommandMode, ServerReadClient},
    worktree_api::{self, WorktreeClient},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationalPlan {
    Bootstrap,
    Recovery,
    Rollout,
}

impl OperationalPlan {
    #[must_use]
    pub const fn command_name(self) -> &'static str {
        match self {
            Self::Bootstrap => "bootstrap plan",
            Self::Recovery => "recovery plan",
            Self::Rollout => "rollout plan",
        }
    }
}

#[must_use]
pub fn render_preflight<C>(config: &CliConfig, client: &C) -> CliOutput
where
    C: DoctorReadClient + ServerReadClient + WorktreeClient,
{
    if config.server_url.is_none() {
        return CliOutput {
            exit_code: CliExitCode::RuntimeError,
            stdout: "preflight: not_run\nreason: server_not_configured".to_owned(),
            stderr: "preflight could not start: server URL is not configured".to_owned(),
        };
    }

    let doctor = doctor_live::render_live_doctor(config, client);
    let adapters =
        server_api::render_adapters_command(config, ReadCommandMode::Auto, client);
    let worktree_status = worktree_api::render_worktree_status(config, client);
    let worktree = if worktree_status.exit_code == CliExitCode::Success
        && worktree_status
            .stdout
            .contains("worktree readiness: ready")
        && worktree_status.stdout.contains("cycles failed: 0")
        && !worktree_status
            .stdout
            .contains("worktree lifecycle: failed")
    {
        worktree_status
    } else if worktree_status.exit_code == CliExitCode::Success {
        CliOutput {
            exit_code: CliExitCode::RuntimeError,
            stdout: worktree_status.stdout,
            stderr: "worktree preflight failed: runtime is not safely ready".to_owned(),
        }
    } else {
        worktree_status
    };

    let ready = [doctor.exit_code, adapters.exit_code, worktree.exit_code]
        .into_iter()
        .all(|code| code == CliExitCode::Success);

    let stdout = format!(
        "preflight: {}\nconfig: {}\n\n[doctor]\n{}\n\n[adapters]\n{}\n\n[worktree]\n{}",
        if ready { "ready" } else { "blocked" },
        config.safe_summary(),
        render_section_stdout(&doctor),
        render_section_stdout(&adapters),
        render_section_stdout(&worktree),
    );

    if ready {
        CliOutput::success(stdout)
    } else {
        let failures = [
            ("doctor", doctor.stderr.as_str()),
            ("adapters", adapters.stderr.as_str()),
            ("worktree", worktree.stderr.as_str()),
        ]
        .into_iter()
        .filter(|(_, error)| !error.is_empty())
        .map(|(name, _)| name)
        .collect::<Vec<_>>();
        CliOutput {
            exit_code: CliExitCode::RuntimeError,
            stdout,
            stderr: format!("preflight failed: {}", failures.join(", ")),
        }
    }
}

fn render_section_stdout(output: &CliOutput) -> &str {
    if output.stdout.is_empty() {
        "not_available"
    } else {
        output.stdout.as_str()
    }
}

#[must_use]
pub fn render_plan(plan: OperationalPlan) -> CliOutput {
    let text = match plan {
        OperationalPlan::Bootstrap => BOOTSTRAP_PLAN,
        OperationalPlan::Recovery => RECOVERY_PLAN,
        OperationalPlan::Rollout => ROLLOUT_PLAN,
    };
    CliOutput::success(text)
}

const BOOTSTRAP_PLAN: &str = "bootstrap plan: dry_run\nwrites: none\n1. select exactly one trusted source\n2. verify coordinated backup and restore evidence\n3. run haze-sync preflight\n4. review duplicate, unsupported, shortcut, and invalid-path reports\n5. keep GDrive disabled until provider sandbox acceptance exists\n6. enable import-only before any outbound mode\n7. enable Worktree export-only before bidirectional mode\n8. enable Obsidian pull-only before local push\n9. enable one write-capable adapter at a time\n10. record rollback and emergency-stop checkpoints";

const RECOVERY_PLAN: &str = "recovery plan: dry_run\nwrites: none\ndestructive restore: requires explicit operator approval\n1. stop or quiesce every writer\n2. preserve incident evidence and exact code revision\n3. verify PostgreSQL, object-store, and Worktree artifacts share one recovery window\n4. restore PostgreSQL metadata into an approved empty or replaceable target\n5. restore matching object-store data\n6. restore Worktree data only when it belongs to the recovery set\n7. start Server with adapters disabled\n8. run haze-sync preflight\n9. re-enable adapters one at a time only after reconciliation\n10. do not use automatic database reset, volume deletion, or hard cleanup";

const ROLLOUT_PLAN: &str = "rollout plan: dry_run\nwrites: none\n1. GDrive dry-run with zero provider writes\n2. review duplicate and unsupported-file reports\n3. GDrive import-only validation\n4. Worktree export-only validation\n5. Obsidian desktop and iOS foreground pull-only validation\n6. enable bidirectional mode for one adapter only\n7. run haze-sync preflight and reconcile counts/hashes\n8. record rollback point before enabling the next adapter\nemergency stop: disable adapters or stop the Server using the approved deployment runbook";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{OutputFormat, ProfileName, ServerUrl, TokenSource},
        doctor_live::{
            HealthSummary, ReadinessComponentState, ReadinessOverallStatus,
            ReadinessSummary,
        },
        server_api::{
            AdapterList, DependencyReadinessState, PauseStatusSummary,
            ServerReadError, ServerStatus, StatusSummary,
        },
        worktree_api::{
            WorktreeClientError, WorktreeStatusRequest, WorktreeSyncRequest,
        },
    };
    use haze_sync_api::dto::worktree::{
        WorktreeConfiguredMode, WorktreeHostLifecycle,
        WorktreeManualAvailability, WorktreeReadiness, WorktreeReadinessReason,
        WorktreeStatusResponse, WorktreeStatusSafeParts,
        WorktreeSyncOnceResponse,
    };

    #[derive(Clone)]
    struct FakeClient {
        fail_doctor: bool,
        worktree_ready: bool,
    }

    impl DoctorReadClient for FakeClient {
        fn fetch_health(
            &self,
            _server_url: &ServerUrl,
        ) -> Result<HealthSummary, ServerReadError> {
            if self.fail_doctor {
                Err(ServerReadError::ServerUnavailable)
            } else {
                Ok(HealthSummary {
                    process_alive: true,
                })
            }
        }

        fn fetch_readiness(
            &self,
            _server_url: &ServerUrl,
        ) -> Result<ReadinessSummary, ServerReadError> {
            Ok(ReadinessSummary {
                overall: ReadinessOverallStatus::Ready,
                database: ReadinessComponentState::Ready,
                object_store: ReadinessComponentState::Ready,
            })
        }

        fn fetch_status(
            &self,
            server_url: &ServerUrl,
        ) -> Result<StatusSummary, ServerReadError> {
            ServerReadClient::fetch_status(self, server_url)
        }
    }

    impl ServerReadClient for FakeClient {
        fn fetch_status(
            &self,
            _server_url: &ServerUrl,
        ) -> Result<StatusSummary, ServerReadError> {
            Ok(StatusSummary {
                server_status: ServerStatus::Ready,
                db_readiness_state: DependencyReadinessState::Ready,
                object_store_readiness_state: DependencyReadinessState::Ready,
                last_operation_sequence: Some(7),
                adapter_count: Some(1),
                pause: PauseStatusSummary::unsupported(),
            })
        }

        fn fetch_adapters(
            &self,
            _server_url: &ServerUrl,
        ) -> Result<AdapterList, ServerReadError> {
            Ok(AdapterList::empty())
        }
    }

    impl WorktreeClient for FakeClient {
        fn fetch_worktree_status(
            &self,
            _server_url: &ServerUrl,
            _request: WorktreeStatusRequest,
        ) -> Result<(u16, WorktreeStatusResponse), WorktreeClientError> {
            Ok((
                200,
                WorktreeStatusResponse::from_safe_parts(
                    WorktreeStatusSafeParts {
                        configured_mode: WorktreeConfiguredMode::Disabled,
                        host_lifecycle: if self.worktree_ready {
                            WorktreeHostLifecycle::Disabled
                        } else {
                            WorktreeHostLifecycle::Failed
                        },
                        readiness: if self.worktree_ready {
                            WorktreeReadiness::Ready
                        } else {
                            WorktreeReadiness::NotReady
                        },
                        readiness_reason: if self.worktree_ready {
                            WorktreeReadinessReason::DisabledInert
                        } else {
                            WorktreeReadinessReason::Failed
                        },
                        cycles_completed: 0,
                        cycles_failed: u64::from(!self.worktree_ready),
                        cycle_in_progress: false,
                        pending_watcher_hints: 0,
                        manual_availability: if self.worktree_ready {
                            WorktreeManualAvailability::Unavailable
                        } else {
                            WorktreeManualAvailability::Failed
                        },
                    },
                ),
            ))
        }

        fn submit_worktree_sync_once(
            &self,
            _server_url: &ServerUrl,
            _request: WorktreeSyncRequest,
        ) -> Result<(u16, WorktreeSyncOnceResponse), WorktreeClientError> {
            Err(WorktreeClientError::ServerUnavailable)
        }
    }

    fn configured() -> CliConfig {
        CliConfig::new(
            ProfileName::parse("ops").unwrap(),
            Some(ServerUrl::parse("https://sync.example.test").unwrap()),
            OutputFormat::Human,
            TokenSource::None,
        )
    }

    #[test]
    fn ready_preflight_aggregates_live_surfaces() {
        let output = render_preflight(
            &configured(),
            &FakeClient {
                fail_doctor: false,
                worktree_ready: true,
            },
        );
        assert_eq!(output.exit_code, CliExitCode::Success);
        assert!(output.stdout.contains("preflight: ready"));
        assert!(output.stdout.contains("config: profile=ops"));
        assert!(output.stdout.contains("[doctor]"));
        assert!(output.stdout.contains("[worktree]"));
        assert!(output.stderr.is_empty());
    }

    #[test]
    fn failed_preflight_is_non_zero_and_safe() {
        let output = render_preflight(
            &configured(),
            &FakeClient {
                fail_doctor: true,
                worktree_ready: true,
            },
        );
        assert_eq!(output.exit_code, CliExitCode::RuntimeError);
        assert!(output.stdout.contains("preflight: blocked"));
        assert_eq!(output.stderr, "preflight failed: doctor");
    }

    #[test]
    fn failed_worktree_status_blocks_preflight() {
        let output = render_preflight(
            &configured(),
            &FakeClient {
                fail_doctor: false,
                worktree_ready: false,
            },
        );
        assert_eq!(output.exit_code, CliExitCode::RuntimeError);
        assert!(output.stdout.contains("worktree lifecycle: failed"));
        assert_eq!(output.stderr, "preflight failed: worktree");
    }

    #[test]
    fn operational_plans_are_explicit_dry_runs() {
        for plan in [
            OperationalPlan::Bootstrap,
            OperationalPlan::Recovery,
            OperationalPlan::Rollout,
        ] {
            let output = render_plan(plan);
            assert_eq!(output.exit_code, CliExitCode::Success);
            assert!(output.stdout.contains("dry_run"));
            assert!(output.stdout.contains("writes: none"));
            assert!(output.stderr.is_empty());
        }
    }
}
