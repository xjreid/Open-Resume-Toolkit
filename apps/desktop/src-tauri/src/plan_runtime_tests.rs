use super::*;

#[test]
fn session_retirement_interrupts_stalled_writes_for_disconnect_and_shutdown() {
    for shutdown in [false, true] {
        let (manager, profile, settings) = signed_in("stall_after_account");
        let manager = std::sync::Arc::new(manager);
        let worker = std::sync::Arc::clone(&manager);
        let (entered, ready) = std::sync::mpsc::channel();
        let handle = std::thread::spawn(move || {
            worker.with_session(
                profile,
                Instant::now() + Duration::from_secs(2),
                SessionAccess {
                    cancel: &|| false,
                    skip_busy: false,
                    permit: &|| Ok(()),
                },
                |_| unreachable!(),
                |session| {
                    entered.send(()).unwrap();
                    session.rpc(
                        "turn/start",
                        json!({"input":"x".repeat(100_000)}),
                        Duration::from_secs(2),
                        &|| false,
                    )
                },
            )
        });
        ready.recv().unwrap();
        std::thread::sleep(Duration::from_millis(20));
        let start = Instant::now();
        if shutdown {
            manager.stop();
        } else {
            manager.disconnect(profile);
        }
        assert_eq!(handle.join().unwrap(), Err("AI_CANCELLED"));
        assert!(start.elapsed() < Duration::from_millis(350));
        assert!(!manager.is_connected(profile, settings.connection_id));
    }
}

#[test]
fn failed_quota_refresh_clears_the_published_snapshot_without_ending_sign_in() {
    for mode in ["quota_then_missing", "quota_then_rejected"] {
        let (manager, profile, settings) = signed_in(mode);
        for succeeds in [true, false] {
            let result = manager.with_session(
                profile,
                Instant::now() + Duration::from_secs(2),
                SessionAccess {
                    cancel: &|| false,
                    skip_busy: false,
                    permit: &|| Ok(()),
                },
                |_| unreachable!(),
                |session| session.refresh_quota(Instant::now() + Duration::from_secs(1), &|| false),
            );
            assert_eq!(result.is_ok(), succeeds);
            let status = manager.status(profile, settings.clone(), None, false);
            assert!(status.connected);
            assert_eq!(status.quota.is_some(), succeeds);
        }
    }
}

fn signed_in(mode: &str) -> (PlanRuntime, Uuid, PlanSettings) {
    let manager = PlanRuntime::default();
    let profile = Uuid::now_v7();
    let settings = PlanSettings {
        enabled: true,
        connection_id: Some(Uuid::now_v7()),
        model: Some("gpt-6.1-sol".into()),
        ..Default::default()
    };
    manager
        .with_session(
            profile,
            Instant::now() + Duration::from_secs(2),
            SessionAccess {
                cancel: &|| false,
                skip_busy: false,
                permit: &|| Ok(()),
            },
            |_| Ok(crate::codex_runtime::test_session(mode)),
            |session| {
                session.rpc(
                    "account/login/start",
                    json!({"type":"chatgpt"}),
                    Duration::from_secs(1),
                    &|| false,
                )?;
                crate::chatgpt_plan::connected(session, &|| false)?;
                session.connection_id = settings.connection_id;
                Ok(())
            },
        )
        .unwrap();
    (manager, profile, settings)
}

#[test]
fn local_controls_and_status_do_not_force_oauth_refresh() {
    let (manager, profile, settings) = signed_in("slow_refresh");
    let start = Instant::now();
    for _ in 0..20 {
        manager.validate_selection(profile, &settings).unwrap();
        assert!(
            manager
                .status(profile, settings.clone(), Some(1), false)
                .connected
        );
    }
    assert!(start.elapsed() < Duration::from_millis(200));
    assert!(!manager.is_connected(Uuid::now_v7(), settings.connection_id));
    assert!(!manager.is_connected(profile, Some(Uuid::now_v7())));
    // Workflow preflight still forces the network authorization refresh.
    let start = Instant::now();
    manager
        .with_session(
            profile,
            start + Duration::from_secs(2),
            SessionAccess {
                cancel: &|| false,
                skip_busy: false,
                permit: &|| Ok(()),
            },
            |_| unreachable!(),
            |session| {
                crate::chatgpt_plan::authorization_ready(session, Duration::from_secs(1), &|| false)
            },
        )
        .unwrap();
    assert!(start.elapsed() >= Duration::from_millis(450));
}

#[test]
fn busy_protocol_does_not_queue_polls_or_block_settings_and_disconnect_interrupts_refresh() {
    let (manager, profile, settings) = signed_in("slow_quota");
    let manager = std::sync::Arc::new(manager);
    let (entered, ready) = std::sync::mpsc::channel();
    let worker = std::sync::Arc::clone(&manager);
    let handle = std::thread::spawn(move || {
        worker.with_session(
            profile,
            Instant::now() + Duration::from_secs(2),
            SessionAccess {
                cancel: &|| false,
                skip_busy: false,
                permit: &|| Ok(()),
            },
            |_| unreachable!(),
            |session| {
                entered.send(()).unwrap();
                session.rpc(
                    "account/rateLimits/read",
                    json!({}),
                    Duration::from_secs(2),
                    &|| worker.interrupted(),
                )
            },
        )
    });
    ready.recv().unwrap();
    let start = Instant::now();
    manager.validate_selection(profile, &settings).unwrap();
    let skipped = manager
        .with_session(
            profile,
            start + Duration::from_secs(2),
            SessionAccess {
                cancel: &|| false,
                skip_busy: true,
                permit: &|| Ok(()),
            },
            |_| unreachable!(),
            |_| -> Result<(), &'static str> { unreachable!() },
        )
        .unwrap();
    assert!(skipped.is_none());
    assert!(
        manager
            .status(profile, settings.clone(), None, false)
            .connected
    );
    assert!(start.elapsed() < Duration::from_millis(200));
    manager.disconnect(profile);
    assert_eq!(handle.join().unwrap(), Err("AI_CANCELLED"));
    assert!(start.elapsed() < Duration::from_millis(350));
    assert!(!manager.is_connected(profile, settings.connection_id));
}

#[test]
fn disconnect_and_restart_destroy_authorization_and_failure_never_reuses_it() {
    let (manager, profile, settings) = signed_in("success");
    manager.disconnect(Uuid::now_v7());
    assert!(!manager.interrupted());
    assert!(manager.is_connected(profile, settings.connection_id));
    manager.disconnect(profile);
    manager.disconnect(profile);
    assert!(!manager.is_connected(profile, settings.connection_id));
    manager
        .with_session(
            profile,
            Instant::now() + Duration::from_secs(2),
            SessionAccess {
                cancel: &|| false,
                skip_busy: false,
                permit: &|| Ok(()),
            },
            |_| Ok(crate::codex_runtime::test_session("success")),
            |session| {
                assert_eq!(
                    crate::chatgpt_plan::authorization_ready(
                        session,
                        Duration::from_secs(1),
                        &|| false
                    ),
                    Err("PLAN_AUTH_REQUIRED")
                );
                Ok(())
            },
        )
        .unwrap();
    assert!(!manager.is_connected(profile, settings.connection_id));
}

#[test]
fn disabled_permission_never_starts_runtime_and_enabled_signed_out_is_valid() {
    let manager = PlanRuntime::default();
    let settings = PlanSettings::default();
    assert!(settings.valid());
    let result = manager.with_session(
        Uuid::now_v7(),
        Instant::now() + Duration::from_secs(1),
        SessionAccess {
            cancel: &|| false,
            skip_busy: false,
            permit: &|| settings.runtime_permission(),
        },
        |_| panic!("disabled runtime started"),
        |_| -> Result<(), &'static str> { panic!("disabled runtime used") },
    );
    assert_eq!(result, Err("PLAN_DISABLED"));
    let enabled = PlanSettings {
        enabled: true,
        ..settings
    };
    assert!(enabled.valid());
    assert_eq!(enabled.runtime_permission(), Ok(()));
}

#[test]
fn a_queued_status_check_rechecks_permission_after_disable_and_cannot_restart() {
    let (manager, profile, settings) = signed_in("success");
    let manager = std::sync::Arc::new(manager);
    let (entered, ready) = std::sync::mpsc::channel();
    let worker = std::sync::Arc::clone(&manager);
    let handle = std::thread::spawn(move || {
        worker.with_session(
            profile,
            Instant::now() + Duration::from_secs(2),
            SessionAccess {
                cancel: &|| false,
                skip_busy: false,
                permit: &|| Ok(()),
            },
            |_| unreachable!(),
            |_| {
                entered.send(()).unwrap();
                std::thread::sleep(Duration::from_millis(75));
                Ok(())
            },
        )
    });
    ready.recv().unwrap();
    let disabled = PlanSettings {
        enabled: false,
        ..settings.clone()
    };
    let result = manager.with_session(
        profile,
        Instant::now() + Duration::from_secs(2),
        SessionAccess {
            cancel: &|| false,
            skip_busy: false,
            permit: &|| disabled.runtime_permission(),
        },
        |_| panic!("runtime restarted after disable"),
        |_| -> Result<(), &'static str> { panic!("runtime reused after disable") },
    );
    assert_eq!(result, Err("PLAN_DISABLED"));
    assert_eq!(handle.join().unwrap(), Ok(Some(())));
    assert!(!manager.is_connected(profile, settings.connection_id));
    assert!(
        manager
            .status(profile, disabled, None, false)
            .runtime_version
            .is_none()
    );
}

#[test]
fn app_shutdown_destroys_sign_in_without_changing_the_enabled_preference() {
    let (manager, profile, settings) = signed_in("success");
    manager.stop();
    let status = manager.status(profile, settings, None, false);
    assert!(status.settings.enabled);
    assert!(!status.connected);
    assert!(status.runtime_version.is_none());
}
