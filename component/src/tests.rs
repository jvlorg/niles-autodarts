use super::*;
use bindings::niles::integration::types::DeviceState as WitDeviceState;
use test_host::niles::integration::{config, devices, http, speech_stream, websocket};

fn response(body: &str) -> Result<http::Response, http::HttpError> {
    Ok(http::Response {
        status: 200,
        headers: vec![],
        body: body.as_bytes().to_vec(),
    })
}

fn initial(running: bool) -> [Result<http::Response, http::HttpError>; 2] {
    [
        response("fixture-version"),
        response(&format!(r#"{{"connected":true,"running":{running}}}"#)),
    ]
}

fn setup(replies: impl IntoIterator<Item = Result<http::Response, http::HttpError>>) {
    Plugin::stop();
    config::reset();
    devices::reset();
    speech_stream::reset();
    websocket::reset();
    test_host::niles::integration::timers::reset();
    test_host::clock::reset();
    config::set(
        "address",
        config::SettingValue::Text("board.local:3180".into()),
    );
    config::set("room", config::SettingValue::Text("living_room".into()));
    http::set_replies(replies);
}

fn desired(on: Option<bool>) -> WitDeviceState {
    WitDeviceState {
        on,
        brightness: None,
        color_temp_kelvin: None,
        rgb: None,
        temperature_celsius: None,
        humidity_percent: None,
        battery_percent: None,
        open: None,
        motion: None,
        illuminance_lux: None,
        position: None,
        readings: vec![],
    }
}

#[test]
fn expired_start_does_not_retry_using_an_observation_before_starting() {
    setup(initial(false));
    Plugin::init().unwrap();
    http::set_replies([response("")]);
    Plugin::set_device("living_room/dartboard".into(), desired(Some(true))).unwrap();
    for body in [
        r#"{"running":false}"#,
        r#"{"status":"Starting","running":false}"#,
    ] {
        http::append_replies([response(body)]);
        Plugin::on_timer(latest_timer("detection-poll:"));
    }
    test_host::clock::advance(std::time::Duration::from_secs(30));
    Plugin::on_timer(latest_timer("detection-expiry:"));
    http::append_replies([response("")]);
    let requests = http::requests().len();
    Plugin::set_device("living_room/dartboard".into(), desired(Some(true))).unwrap();
    assert_eq!(
        http::requests().len(),
        requests,
        "Starting must retain the duplicate latch"
    );
}

#[test]
fn delayed_poll_delivery_cannot_extend_detection_monitoring() {
    setup(initial(false));
    Plugin::init().unwrap();
    http::set_replies([response("")]);
    Plugin::set_device("living_room/dartboard".into(), desired(Some(true))).unwrap();
    let timer = latest_timer("detection-poll:");
    let requests = http::requests().len();
    test_host::clock::advance(std::time::Duration::from_secs(31));
    http::append_replies([response(r#"{"running":false}"#)]);
    Plugin::on_timer(timer);
    assert_eq!(
        http::requests().len(),
        requests,
        "expired polling must not issue HTTP"
    );
    assert!(
        render_text()
            .iter()
            .any(|line| line.contains("monitoring expired"))
    );
}

#[test]
fn disconnected_and_contradictory_readbacks_cannot_confirm_detection() {
    for (initial_on, body) in [
        (false, r#"{"connected":false,"running":true}"#),
        (true, r#"{"status":"Starting","running":true}"#),
    ] {
        setup(initial(initial_on));
        Plugin::init().unwrap();
        http::set_replies([response("")]);
        Plugin::set_device("living_room/dartboard".into(), desired(Some(!initial_on))).unwrap();
        http::append_replies([response(body)]);
        Plugin::on_timer(latest_timer("detection-poll:"));
        assert_eq!(
            devices::calls().upserts.last().unwrap().state.on,
            Some(initial_on)
        );
        assert!(
            !render_text()
                .iter()
                .any(|line| line.contains("Detection command: running confirmed")
                    || line.contains("Detection command: stopped confirmed"))
        );
    }
}

#[test]
fn invalid_event_cannot_publish_running_before_ingress_validation() {
    setup(initial(false));
    enable_announcements("total_only");
    Plugin::init().unwrap();
    send_event(r#"{"type":"state","data":{"running":true,"numThrows":4,"throws":[]}}"#);
    assert_eq!(
        devices::calls().upserts.last().unwrap().state.on,
        Some(false)
    );
}

#[test]
fn timer_failure_does_not_call_uncertain_dispatch_accepted() {
    setup(initial(false));
    Plugin::init().unwrap();
    test_host::niles::integration::timers::fail_after(true);
    http::fail_next(http::HttpError::Timeout);
    Plugin::set_device("living_room/dartboard".into(), desired(Some(true))).unwrap();
    let lines = render_text();
    assert!(lines.iter().any(|line| line.contains("dispatch uncertain")));
    assert!(!lines.iter().any(|line| line.contains("startup accepted")));
}

#[test]
fn complete_large_rejection_response_is_not_hidden_as_dispatch_uncertainty() {
    setup(initial(false));
    Plugin::init().unwrap();
    http::set_replies([Ok(http::Response {
        status: 503,
        headers: vec![],
        body: vec![b'x'; autodarts_core::MAX_RESPONSE_BYTES + 1],
    })]);
    let result = Plugin::set_device("living_room/dartboard".into(), desired(Some(true)));
    assert!(result.unwrap_err().contains("command_http_status"));
    assert_eq!(
        devices::calls().upserts.last().unwrap().state.on,
        Some(false)
    );
}

#[test]
fn readback_crossing_deadline_does_not_rearm_monitoring() {
    setup(initial(false));
    Plugin::init().unwrap();
    http::set_replies([response("")]);
    Plugin::set_device("living_room/dartboard".into(), desired(Some(true))).unwrap();
    let timer = latest_timer("detection-poll:");
    test_host::clock::advance(Duration::from_secs(29));
    http::elapse_next(Duration::from_secs(2));
    http::append_replies([response(r#"{"status":"Starting","running":false}"#)]);
    Plugin::on_timer(timer.clone());
    let requests = http::requests().len();
    Plugin::on_timer(timer);
    Plugin::set_device("living_room/dartboard".into(), desired(Some(true))).unwrap();
    assert_eq!(http::requests().len(), requests);
    assert!(
        render_text()
            .iter()
            .any(|line| line.contains("monitoring expired"))
    );
}

#[test]
fn persistent_false_expires_and_requires_a_new_post_expiry_observation() {
    setup(initial(false));
    Plugin::init().unwrap();
    http::set_replies([response("")]);
    Plugin::set_device("living_room/dartboard".into(), desired(Some(true))).unwrap();
    for _ in 0..29 {
        test_host::clock::advance(Duration::from_secs(1));
        http::append_replies([response(r#"{"running":false}"#)]);
        Plugin::on_timer(latest_timer("detection-poll:"));
    }
    test_host::clock::advance(Duration::from_secs(1));
    Plugin::on_timer(latest_timer("detection-expiry:"));
    let requests = http::requests().len();
    Plugin::set_device("living_room/dartboard".into(), desired(Some(true))).unwrap();
    assert_eq!(http::requests().len(), requests);
    http::append_replies(initial(false));
    Plugin::on_timer("poll".into());
    http::append_replies([response("")]);
    Plugin::set_device("living_room/dartboard".into(), desired(Some(true))).unwrap();
    assert_eq!(
        http::requests()
            .iter()
            .filter(|request| request.method == http::Method::Put)
            .count(),
        2
    );
}

#[test]
fn stop_supersedes_start_and_stale_callbacks_do_not_mutate_new_operations() {
    setup(initial(false));
    Plugin::init().unwrap();
    http::set_replies([response(""), response("")]);
    Plugin::set_device("living_room/dartboard".into(), desired(Some(true))).unwrap();
    let old_poll = latest_timer("detection-poll:");
    let old_expiry = latest_timer("detection-expiry:");
    Plugin::set_device("living_room/dartboard".into(), desired(Some(false))).unwrap();
    let stop_poll = latest_timer("detection-poll:");
    Plugin::on_timer(old_poll.clone());
    Plugin::on_timer(old_expiry);
    Plugin::set_device("living_room/dartboard".into(), desired(Some(false))).unwrap();
    assert_eq!(http::requests().len(), 2);
    assert!(http::requests()[1].url.ends_with("/api/stop"));
    http::append_replies([response(r#"{"running":true}"#)]);
    Plugin::on_timer(stop_poll.clone());
    assert_eq!(
        devices::calls().upserts.last().unwrap().state.on,
        Some(true)
    );
    http::append_replies(initial(false));
    config::set("room", config::SettingValue::Text("office".into()));
    Plugin::init().unwrap();
    let count = http::requests().len();
    Plugin::on_timer(old_poll);
    Plugin::on_timer(stop_poll);
    assert_eq!(http::requests().len(), count);
    assert_eq!(
        devices::calls().upserts.last().unwrap().state.on,
        Some(false)
    );
}

#[test]
fn only_current_available_running_observation_avoids_dispatch() {
    setup(initial(true));
    Plugin::init().unwrap();
    http::set_replies([]);
    Plugin::set_device("living_room/dartboard".into(), desired(Some(true))).unwrap();
    assert!(http::requests().is_empty());
    http::append_replies([response("fixture-version"), Err(http::HttpError::Timeout)]);
    Plugin::on_timer("poll".into());
    http::append_replies([response("")]);
    Plugin::set_device("living_room/dartboard".into(), desired(Some(true))).unwrap();
    assert_eq!(
        http::requests()
            .iter()
            .filter(|request| request.method == http::Method::Put)
            .count(),
        1
    );
}

#[test]
fn composed_device_start_stop_preserve_identity_and_exact_provider_calls() {
    setup(initial(false));
    Plugin::init().unwrap();
    http::set_replies([Ok(http::Response {
        status: 204,
        headers: vec![],
        body: vec![],
    })]);
    assert!(Plugin::set_device("living_room/dartboard".into(), desired(Some(true))).is_ok());
    assert_eq!(
        http::requests()
            .iter()
            .map(|r| (r.method, r.url.as_str()))
            .collect::<Vec<_>>(),
        [(http::Method::Put, "http://board.local:3180/api/start")]
    );
    assert_eq!(
        devices::calls().upserts.last().unwrap().state.on,
        Some(false)
    );

    http::append_replies([response(r#"{"connected":true,"running":true}"#)]);
    let timer = latest_timer("detection-poll:");
    Plugin::on_timer(timer);
    assert_eq!(
        devices::calls().upserts.last().unwrap().state.on,
        Some(true)
    );

    http::set_replies([Ok(http::Response {
        status: 204,
        headers: vec![],
        body: vec![],
    })]);
    assert!(Plugin::set_device("living_room/dartboard".into(), desired(Some(false))).is_ok());
    assert_eq!(http::requests()[0].method, http::Method::Put);
    assert_eq!(http::requests()[0].url, "http://board.local:3180/api/stop");
    http::append_replies([response(r#"{"connected":true,"running":false}"#)]);
    Plugin::on_timer(latest_timer("detection-poll:"));
    assert_eq!(
        devices::calls().upserts.last().unwrap().state.on,
        Some(false)
    );
}

#[test]
fn confirmed_detection_restart_reopens_speech_stream() {
    setup(initial(true));
    enable_announcements("total_only");
    Plugin::init().unwrap();
    send_event(EMPTY_EVENT);
    send_event(ONE_DART);
    send_event(REMOVED);
    assert_eq!(speech_stream::calls().submitted.len(), 1);
    let first_session = speech_stream::calls().submitted[0].0;

    http::set_replies([Ok(http::Response {
        status: 204,
        headers: vec![],
        body: vec![],
    })]);
    assert!(Plugin::set_device("living_room/dartboard".into(), desired(Some(false))).is_ok());
    assert_eq!(speech_stream::calls().closed, [first_session]);

    http::set_replies([Ok(http::Response {
        status: 204,
        headers: vec![],
        body: vec![],
    })]);
    assert!(Plugin::set_device("living_room/dartboard".into(), desired(Some(true))).is_ok());
    http::append_replies([response(r#"{"connected":true,"running":true}"#)]);
    Plugin::on_timer(latest_timer("detection-poll:"));
    let calls = speech_stream::calls();
    assert_eq!(calls.opened_sessions.len(), 2);
    assert_eq!(calls.opened_sessions[1].1, "living_room");

    send_event(EMPTY_EVENT);
    send_event(ONE_DART);
    send_event(REMOVED);
    assert_eq!(speech_stream::calls().submitted.len(), 2);
    assert_eq!(
        speech_stream::calls().submitted[1].0,
        calls.opened_sessions[1].0
    );
}

#[test]
fn composed_acknowledged_but_observed_false_is_pending_and_keeps_actual_state() {
    setup(initial(false));
    Plugin::init().unwrap();
    http::set_replies([Ok(http::Response {
        status: 200,
        headers: vec![],
        body: vec![],
    })]);
    assert!(
        Plugin::set_device("living_room/dartboard".into(), desired(Some(true)).clone()).is_ok()
    );
    http::append_replies([response(r#"{"running":false}"#)]);
    Plugin::on_timer(latest_timer("detection-poll:"));
    assert_eq!(
        devices::calls().upserts.last().unwrap().state.on,
        Some(false)
    );
    assert!(
        render_text()
            .iter()
            .any(|line| line.contains("still stopped"))
    );
}
#[test]
fn composed_timeout_is_uncertain_without_optimistic_state_and_rejection_is_an_error() {
    setup(initial(false));
    Plugin::init().unwrap();
    http::fail_next(http::HttpError::Timeout);
    assert!(Plugin::set_device("living_room/dartboard".into(), desired(Some(true))).is_ok());
    assert_eq!(
        devices::calls().upserts.last().unwrap().state.on,
        Some(false)
    );
    assert_eq!(http::requests().len(), 3);
    assert!(
        render_text()
            .iter()
            .any(|line| line.contains("dispatch uncertain"))
    );

    setup(initial(false));
    Plugin::init().unwrap();
    http::set_replies([Ok(http::Response {
        status: 503,
        headers: vec![],
        body: vec![],
    })]);
    assert!(Plugin::set_device("living_room/dartboard".into(), desired(Some(true))).is_err());
    assert_eq!(
        devices::calls().upserts.last().unwrap().state.on,
        Some(false)
    );
    assert!(render_text().iter().any(|line| line.contains("rejected")));
}
#[test]
fn delayed_startup_schedule_is_truthful_for_ack_and_timeout_dispatch() {
    let schedule: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../crates/autodarts-core/tests/fixtures/board-manager-jvl15-startup-synthetic-scenarios.json"
    ))
    .unwrap();
    let scenarios = schedule["scenarios"].as_array().unwrap();
    for dispatch in ["http_2xx", "response_timeout"] {
        let scenario = scenarios
            .iter()
            .find(|scenario| scenario["dispatch"] == dispatch)
            .unwrap();
        let events = scenario["events"].as_array().unwrap();
        setup(initial(false));
        enable_announcements("total_only");
        Plugin::init().unwrap();
        if dispatch == "http_2xx" {
            http::set_replies([Ok(http::Response {
                status: 204,
                headers: vec![],
                body: vec![],
            })]);
        } else {
            http::set_replies([]);
            http::fail_next(http::HttpError::Timeout);
        }
        assert!(Plugin::set_device("living_room/dartboard".into(), desired(Some(true))).is_ok());
        let requests_after_dispatch = http::requests().len();
        assert!(Plugin::set_device("living_room/dartboard".into(), desired(Some(true))).is_ok());
        assert_eq!(http::requests().len(), requests_after_dispatch);

        let starting = serde_json::to_string(&events[0]).unwrap();
        send_event(&starting);
        let starting_body = serde_json::to_string(&events[0]["data"]).unwrap();
        let started_at = Duration::from_secs_f64(events[1]["at_seconds"].as_f64().unwrap());
        for _ in 0..started_at.as_secs() {
            test_host::clock::advance(Duration::from_secs(1));
            http::append_replies([response(&starting_body)]);
            let before = http::requests().len();
            Plugin::on_timer(latest_timer("detection-poll:"));
            assert_eq!(http::requests().len(), before + 1);
            Plugin::set_device("living_room/dartboard".into(), desired(Some(true))).unwrap();
            assert_eq!(
                http::requests().len(),
                before + 1,
                "duplicate must not dispatch"
            );
        }
        assert!(
            devices::calls()
                .upserts
                .iter()
                .all(|device| device.state.on == Some(false))
        );
        test_host::clock::advance(started_at - Duration::from_secs(started_at.as_secs()));
        send_event(&serde_json::to_string(&events[1]).unwrap());
        assert_eq!(
            devices::calls().upserts.last().unwrap().state.on,
            Some(true)
        );
        assert!(
            render_text()
                .iter()
                .any(|line| line.contains("running confirmed"))
        );
    }
}

#[test]
fn starting_running_true_does_not_confirm_component_detection() {
    setup(initial(false));
    Plugin::init().unwrap();
    http::set_replies([Ok(http::Response {
        status: 204,
        headers: vec![],
        body: vec![],
    })]);
    assert!(Plugin::set_device("living_room/dartboard".into(), desired(Some(true))).is_ok());
    http::append_replies([response(r#"{"status":"Starting","running":true}"#)]);
    Plugin::on_timer(latest_timer("detection-poll:"));
    assert_eq!(
        devices::calls().upserts.last().unwrap().state.on,
        Some(false)
    );
}

#[test]
fn expired_startup_suppresses_duplicates_until_fresh_readback_and_accepts_late_completion() {
    setup(initial(false));
    Plugin::init().unwrap();
    http::set_replies([Ok(http::Response {
        status: 204,
        headers: vec![],
        body: vec![],
    })]);
    assert!(Plugin::set_device("living_room/dartboard".into(), desired(Some(true))).is_ok());
    let before_expiry = http::requests().len();
    let expiry = latest_timer("detection-expiry:");
    test_host::clock::advance(Duration::from_secs(30));
    Plugin::on_timer(expiry);
    assert!(
        render_text()
            .iter()
            .any(|line| line.contains("monitoring expired"))
    );
    assert!(Plugin::set_device("living_room/dartboard".into(), desired(Some(true))).is_ok());
    assert_eq!(http::requests().len(), before_expiry);

    http::append_replies([
        response("fixture-version"),
        response(r#"{"status":"Started","running":true}"#),
    ]);
    Plugin::on_timer("poll".into());
    assert_eq!(
        devices::calls().upserts.last().unwrap().state.on,
        Some(true)
    );
    assert!(
        render_text()
            .iter()
            .any(|line| line.contains("running confirmed"))
    );
}

#[test]
fn unavailable_detection_readback_keeps_polling_until_recovery() {
    setup(initial(false));
    Plugin::init().unwrap();
    http::set_replies([Ok(http::Response {
        status: 204,
        headers: vec![],
        body: vec![],
    })]);
    assert!(Plugin::set_device("living_room/dartboard".into(), desired(Some(true))).is_ok());
    http::fail_next(http::HttpError::Timeout);
    Plugin::on_timer(latest_timer("detection-poll:"));
    assert!(
        render_text()
            .iter()
            .any(|line| line.contains("state timeout"))
    );
    http::append_replies([response(r#"{"status":"Started","running":true}"#)]);
    Plugin::on_timer(latest_timer("detection-poll:"));
    assert_eq!(
        devices::calls().upserts.last().unwrap().state.on,
        Some(true)
    );
}

#[test]
fn ambiguous_connection_reports_unknown_outcome_for_start_and_stop() {
    for (initial_on, desired_on, action) in [(false, true, "start"), (true, false, "stop")] {
        setup(initial(initial_on));
        Plugin::init().unwrap();
        http::fail_next(http::HttpError::Connection(
            "opaque transport detail".into(),
        ));
        let result = Plugin::set_device("living_room/dartboard".into(), desired(Some(desired_on)));
        assert_eq!(
            result.unwrap_err(),
            "Detection command outcome unknown: command_connection"
        );
        assert_eq!(
            devices::calls().upserts.last().unwrap().state.on,
            Some(initial_on)
        );
        let lines = render_text();
        assert!(lines.iter().any(|line| line
            == &format!("Detection command: {action} outcome unknown: command_connection")));
        assert!(!lines.iter().any(|line| line.contains("rejected")));
        assert!(
            lines
                .iter()
                .any(|line| line == "Detection readback: command_connection")
        );
        assert_eq!(
            http::requests()
                .iter()
                .filter(|request| request.method == http::Method::Put)
                .count(),
            1
        );
    }
}

#[test]
fn complete_http_rejection_remains_a_rejection_error() {
    setup(initial(false));
    Plugin::init().unwrap();
    http::set_replies([Ok(http::Response {
        status: 503,
        headers: vec![],
        body: vec![],
    })]);
    let result = Plugin::set_device("living_room/dartboard".into(), desired(Some(true)));
    assert_eq!(
        result.unwrap_err(),
        "Detection command failed: command_http_status"
    );
    assert_eq!(
        devices::calls().upserts.last().unwrap().state.on,
        Some(false)
    );
    let lines = render_text();
    assert!(
        lines
            .iter()
            .any(|line| line == "Detection command: start rejected: command_http_status")
    );
    assert!(!lines.iter().any(|line| line.contains("outcome unknown")));
}

#[test]
fn timer_registration_failure_remains_distinct_from_dispatch_outcome() {
    setup(initial(false));
    Plugin::init().unwrap();
    test_host::niles::integration::timers::fail_after(true);
    http::set_replies([Ok(http::Response {
        status: 204,
        headers: vec![],
        body: vec![],
    })]);
    assert!(Plugin::set_device("living_room/dartboard".into(), desired(Some(true))).is_ok());
    assert!(
        render_text()
            .iter()
            .any(|line| line.contains("monitoring unavailable"))
    );
}
#[test]
fn composed_unsupported_unconfigured_and_recovery_paths() {
    setup([]);
    assert!(Plugin::set_device("living_room/dartboard".into(), desired(Some(true))).is_err());
    assert!(http::requests().is_empty());

    setup(initial(false));
    Plugin::init().unwrap();
    let mut unsupported = desired(Some(true));
    http::set_replies([]);
    unsupported
        .readings
        .push(bindings::niles::integration::types::Reading {
            name: "x".into(),
            value: 1.0,
            unit: None,
        });
    assert!(Plugin::set_device("living_room/dartboard".into(), unsupported).is_err());
    assert!(Plugin::set_device("other_room/dartboard".into(), desired(Some(true))).is_err());
    assert!(Plugin::set_device("living_room/dartboard".into(), desired(None)).is_err());
    assert!(http::requests().is_empty());

    http::set_replies([Ok(http::Response {
        status: 200,
        headers: vec![],
        body: vec![],
    })]);
    assert!(Plugin::set_device("living_room/dartboard".into(), desired(Some(true))).is_ok());
    http::append_replies([response(r#"{"running":true}"#)]);
    Plugin::on_timer(latest_timer("detection-poll:"));
    assert_eq!(
        devices::calls().upserts.last().unwrap().state.on,
        Some(true)
    );
}

#[test]
fn invalid_poll_interval_fails_closed_instead_of_defaulting() {
    setup([]);
    config::set("poll_interval", config::SettingValue::Number(4.5));
    assert!(Plugin::init().is_err());

    setup([]);
    config::set("poll_interval", config::SettingValue::Boolean(true));
    assert!(Plugin::init().is_err());
}

#[test]
fn announcement_settings_room_speech_lifecycle_and_synthetic_delivery() {
    setup(initial(false));
    config::set("announcements_enabled", config::SettingValue::Boolean(true));
    config::set(
        "announcement_mode",
        config::SettingValue::Text("total_only".into()),
    );
    config::set("speech_stream_enabled", config::SettingValue::Boolean(true));
    Plugin::init().unwrap();
    assert_eq!(
        websocket::calls().opened,
        [(
            "board_http".into(),
            "ws://board.local:3180/api/events".into()
        )]
    );
    assert_eq!(speech_stream::calls().opened_rooms, ["living_room"]);
    let websocket_before = websocket::calls();
    let speech_before = speech_stream::calls();
    Plugin::init().unwrap();
    let websocket_after = websocket::calls();
    let speech_after = speech_stream::calls();
    assert_eq!(websocket_after.opened, websocket_before.opened);
    assert_eq!(websocket_after.closed, websocket_before.closed);
    assert_eq!(speech_after.opened_sessions, speech_before.opened_sessions);
    assert_eq!(speech_after.closed, speech_before.closed);
    let lines = render_text();
    assert!(
        lines
            .iter()
            .any(|line| line == "Announcement intent: enabled")
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "Announcement mode: Total only")
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "Announcement room: living_room")
    );
    assert!(lines.iter().any(|line| line == "Speech stream open"));

    const EMPTY: &str = r#"{"type":"state","data":{"running":true,"numThrows":0,"throws":[]}}"#;
    const DART: &str = r#"{"type":"state","data":{"running":true,"numThrows":1,"throws":[{"segment":{"number":20,"multiplier":1}}]}}"#;
    const FULL: &str = r#"{"type":"motion_state","data":{"isTakeoutFull":true}}"#;
    assert_eq!(
        ingest_announcement_json(EMPTY.as_bytes()),
        autodarts_core::local_score::JourneyResult::Baseline
    );
    ingest_announcement_json(DART.as_bytes());
    assert_eq!(
        ingest_announcement_json(FULL.as_bytes()),
        autodarts_core::local_score::JourneyResult::Total(Some(20))
    );
    let request = speech_stream::calls().submitted_ids[0].1;
    speech_stream::set_statuses(vec![
        bindings::niles::integration::speech_stream::RequestStatus {
            request,
            outcome: bindings::niles::integration::speech_stream::Outcome::Pending,
        },
    ]);
    assert!(RUNTIME.with(|slot| {
        slot.borrow()
            .as_ref()
            .is_some_and(|runtime| runtime.speech_pending.len() == 1)
    }));
    http::set_replies(initial(true));
    Plugin::on_timer("poll".into());

    speech_stream::set_statuses(vec![
        bindings::niles::integration::speech_stream::RequestStatus {
            request,
            outcome: bindings::niles::integration::speech_stream::Outcome::Replaced,
        },
    ]);
    http::set_replies(initial(true));
    Plugin::on_timer("poll".into());
    let lines = render_text();
    assert!(lines.iter().any(|line| line.contains("Replaced")));
    assert!(lines.iter().any(|line| line.contains("Speech stream open")));
    assert!(
        lines
            .iter()
            .any(|line| line.contains("epoch:0:collection:"))
    );
    let old_session = speech_stream::calls().opened_sessions[0].0;

    config::set("room", config::SettingValue::Text("office".into()));
    config::set(
        "speech_stream_enabled",
        config::SettingValue::Boolean(false),
    );
    http::set_replies(initial(false));
    Plugin::init().unwrap();
    assert_eq!(speech_stream::calls().closed, [old_session]);
    assert_eq!(speech_stream::calls().opened_rooms, ["living_room"]);
    assert!(
        render_text()
            .iter()
            .any(|line| line == "Announcement room: office")
    );
    let lines = render_text();
    assert!(
        lines
            .iter()
            .any(|line| line.contains("Speech stream disabled"))
    );
    assert!(lines.iter().any(|line| line.contains("Replaced")));
    Plugin::stop();
}

const EMPTY_EVENT: &str = r#"{"type":"state","data":{"running":true,"numThrows":0,"throws":[]}}"#;
const ONE_DART: &str = r#"{"type":"state","data":{"running":true,"numThrows":1,"throws":[{"segment":{"number":20,"multiplier":1}}]}}"#;
const TWO_DARTS: &str = r#"{"type":"state","data":{"running":true,"numThrows":2,"throws":[{"segment":{"number":20,"multiplier":1}},{"segment":{"number":5,"multiplier":1}}]}}"#;
const REMOVED: &str = r#"{"type":"motion_state","data":{"isTakeoutFull":true}}"#;

fn enable_announcements(mode: &str) {
    config::set("announcements_enabled", config::SettingValue::Boolean(true));
    config::set("announcement_mode", config::SettingValue::Text(mode.into()));
    config::set("speech_stream_enabled", config::SettingValue::Boolean(true));
}

#[test]
fn websocket_origin_conversion_uses_only_validated_origins_and_granted_ports() {
    let plain = Config::new("board.local", "living_room", 15, 2_000).unwrap();
    assert_eq!(
        websocket_url(&plain).unwrap(),
        ("ws://board.local/api/events".into(), "board_http_default")
    );
    let tls = Config::new("https://board.local", "living_room", 15, 2_000).unwrap();
    assert_eq!(
        websocket_url(&tls).unwrap(),
        ("wss://board.local/api/events".into(), "board_https")
    );
    let alternate_port = Config::new("board.local:8080", "living_room", 15, 2_000).unwrap();
    assert!(websocket_url(&alternate_port).is_err());
    assert!(Config::new("http://user@board.local:3180", "living_room", 15, 2_000).is_err());
    assert!(Config::new("http://board.local/private", "living_room", 15, 2_000).is_err());
}

#[test]
fn websocket_permission_mismatch_is_reported_without_failing_status_init() {
    setup(initial(false));
    config::set(
        "address",
        config::SettingValue::Text("board.local:8080".into()),
    );
    enable_announcements("total_only");
    Plugin::init().unwrap();
    assert!(websocket::calls().opened.is_empty());
    assert!(render_text().iter().any(|line| {
        line.contains("Board event stream unavailable")
            && line.contains("port is not covered by its host permission")
    }));
}

#[test]
fn websocket_callback_drives_each_dart_and_total_after_clean_baseline() {
    setup(initial(false));
    enable_announcements("each_dart_then_total");
    Plugin::init().unwrap();
    websocket::status(
        41,
        bindings::niles::integration::websocket::ConnectionStatus::Connected,
    );
    websocket::message(
        41,
        bindings::niles::integration::websocket::Message::Text(EMPTY_EVENT.into()),
    );
    websocket::message(
        41,
        bindings::niles::integration::websocket::Message::Text(ONE_DART.into()),
    );
    websocket::message(
        41,
        bindings::niles::integration::websocket::Message::Text(TWO_DARTS.into()),
    );
    websocket::message(
        41,
        bindings::niles::integration::websocket::Message::Text(REMOVED.into()),
    );
    drain_current_speech();
    assert_eq!(
        speech_stream::calls().submitted,
        [(7, "20".into()), (7, "5".into()), (7, "25".into())]
    );
}
#[test]
fn supplied_capture_uses_actual_callback_before_separate_successive_rounds() {
    let capture: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../crates/autodarts-core/tests/fixtures/board-manager-1.0.7-reduced-capture.json"
    ))
    .unwrap();
    let continuation: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../crates/autodarts-core/tests/fixtures/board-manager-1.0.7-synthetic-continuation.json"
    ))
    .unwrap();
    for mode in ["each_dart_then_total", "total_only"] {
        setup(initial(false));
        enable_announcements(mode);
        Plugin::init().unwrap();
        websocket::status(
            41,
            bindings::niles::integration::websocket::ConnectionStatus::Connected,
        );
        for (index, envelope) in capture.as_array().unwrap().iter().enumerate() {
            send_event(&serde_json::to_string(envelope).unwrap());
            assert!(speech_stream::calls().submitted.is_empty());
            if index == 11 {
                let lines = render_text();
                assert!(
                    lines
                        .iter()
                        .any(|line| line == "Policy readiness: collecting (0 darts)")
                );
                assert!(
                    lines
                        .iter()
                        .any(|line| line == "Last policy event: Processed (BaselineEstablished)")
                );
            }
        }
        let lines = render_text();
        assert!(
            lines
                .iter()
                .any(|line| line == "Policy readiness: collecting (0 darts)")
        );
        assert!(!lines.iter().any(|line| line.contains("baseline required")));

        let expected = if mode == "each_dart_then_total" {
            [
                "3", "7", "7", "17", "3", "7", "7", "17", "3", "7", "7", "17",
            ]
            .as_slice()
        } else {
            ["17", "17", "17"].as_slice()
        };
        let mut submitted = 0;
        for (index, envelope) in continuation.as_array().unwrap().iter().enumerate() {
            if matches!(index, 0 | 7 | 12) {
                // Separate synthetic removal episodes; do not alter the real capture.
                send_event(NEUTRAL_MOTION);
            }
            let emits = if mode == "total_only" {
                matches!(index, 3 | 11 | 15)
            } else {
                matches!(index, 0..=3 | 7..=9 | 11..=15)
            };
            send_event(&serde_json::to_string(envelope).unwrap());
            drain_current_speech();
            if emits {
                submitted += 1;
            }
            assert_eq!(
                speech_stream::calls()
                    .submitted
                    .iter()
                    .map(|(_, text)| text.as_str())
                    .collect::<Vec<_>>(),
                &expected[..submitted]
            );
        }
        assert_eq!(submitted, expected.len());
    }
}

#[test]
fn websocket_disconnect_requires_new_baseline_and_ignores_stale_callbacks() {
    setup(initial(false));
    enable_announcements("total_only");
    Plugin::init().unwrap();
    websocket::status(
        41,
        bindings::niles::integration::websocket::ConnectionStatus::Connected,
    );
    websocket::message(
        41,
        bindings::niles::integration::websocket::Message::Text(EMPTY_EVENT.into()),
    );
    websocket::message(
        41,
        bindings::niles::integration::websocket::Message::Text(ONE_DART.into()),
    );
    websocket::status(
        41,
        bindings::niles::integration::websocket::ConnectionStatus::Disconnected("test".into()),
    );
    websocket::status(
        41,
        bindings::niles::integration::websocket::ConnectionStatus::Connected,
    );
    websocket::message(
        41,
        bindings::niles::integration::websocket::Message::Text(REMOVED.into()),
    );
    websocket::message(
        99,
        bindings::niles::integration::websocket::Message::Text(EMPTY_EVENT.into()),
    );
    websocket::message(
        99,
        bindings::niles::integration::websocket::Message::Text(ONE_DART.into()),
    );
    websocket::message(
        41,
        bindings::niles::integration::websocket::Message::Text(REMOVED.into()),
    );
    assert!(speech_stream::calls().submitted.is_empty());
    send_event(NEUTRAL_MOTION);
    websocket::message(
        41,
        bindings::niles::integration::websocket::Message::Text(EMPTY_EVENT.into()),
    );
    websocket::message(
        41,
        bindings::niles::integration::websocket::Message::Text(ONE_DART.into()),
    );
    websocket::message(
        41,
        bindings::niles::integration::websocket::Message::Text(REMOVED.into()),
    );
    assert_eq!(
        speech_stream::calls()
            .submitted
            .iter()
            .map(|(_, text)| text.as_str())
            .collect::<Vec<_>>(),
        ["20"]
    );
    websocket::status(
        41,
        bindings::niles::integration::websocket::ConnectionStatus::Failed(
            bindings::niles::integration::websocket::FailureKind::Policy,
        ),
    );
    assert!(
        render_text()
            .iter()
            .any(|line| line == "Board event stream failed: policy denied")
    );
}

#[test]
fn malformed_websocket_frames_invalidate_without_false_speech() {
    setup(initial(false));
    enable_announcements("total_only");
    Plugin::init().unwrap();
    websocket::message(
        41,
        bindings::niles::integration::websocket::Message::Text(EMPTY_EVENT.into()),
    );
    websocket::message(
        41,
        bindings::niles::integration::websocket::Message::Text(ONE_DART.into()),
    );
    websocket::message(
        41,
        bindings::niles::integration::websocket::Message::Binary(vec![1, 2]),
    );
    websocket::message(
        41,
        bindings::niles::integration::websocket::Message::Text("{malformed".into()),
    );
    websocket::message(
        41,
        bindings::niles::integration::websocket::Message::Text(REMOVED.into()),
    );
    assert!(speech_stream::calls().submitted.is_empty());
}
#[test]
fn invalid_callback_keeps_reason_while_resetting_readiness() {
    setup(initial(false));
    enable_announcements("total_only");
    Plugin::init().unwrap();
    websocket::message(
        41,
        bindings::niles::integration::websocket::Message::Text("{bad".into()),
    );
    let rendered = render_text();
    assert!(
        rendered
            .iter()
            .any(|line| line == "Policy readiness: awaiting baseline")
    );
    assert!(
        rendered
            .iter()
            .any(|line| line.contains("Last policy event: Invalid (InvalidInput)"))
    );
}

#[test]
fn reconfigure_disable_and_stop_close_websocket() {
    setup(initial(false));
    enable_announcements("total_only");
    Plugin::init().unwrap();
    config::set("room", config::SettingValue::Text("office".into()));
    http::set_replies(initial(false));
    Plugin::init().unwrap();
    assert_eq!(websocket::calls().closed, [41]);
    assert_eq!(websocket::calls().opened.len(), 2);
    websocket::message(
        41,
        bindings::niles::integration::websocket::Message::Text(EMPTY_EVENT.into()),
    );
    websocket::message(
        41,
        bindings::niles::integration::websocket::Message::Text(ONE_DART.into()),
    );
    websocket::message(
        41,
        bindings::niles::integration::websocket::Message::Text(REMOVED.into()),
    );
    assert!(speech_stream::calls().submitted.is_empty());
    config::set(
        "announcements_enabled",
        config::SettingValue::Boolean(false),
    );
    http::set_replies(initial(false));
    Plugin::init().unwrap();
    assert_eq!(websocket::calls().closed, [41, 42]);
    assert_eq!(websocket::calls().opened.len(), 2);
    Plugin::stop();
    assert_eq!(websocket::calls().closed, [41, 42]);
}

#[test]
fn speech_outcomes_remain_correlated_after_repeated_polls() {
    let outcomes = [
        bindings::niles::integration::speech_stream::Outcome::Delivered,
        bindings::niles::integration::speech_stream::Outcome::Busy,
        bindings::niles::integration::speech_stream::Outcome::Quiet,
        bindings::niles::integration::speech_stream::Outcome::Expired,
        bindings::niles::integration::speech_stream::Outcome::Failed,
        bindings::niles::integration::speech_stream::Outcome::Cancelled,
        bindings::niles::integration::speech_stream::Outcome::Replaced,
        bindings::niles::integration::speech_stream::Outcome::RateLimited,
        bindings::niles::integration::speech_stream::Outcome::NoSatellite,
    ];
    for outcome in outcomes {
        setup(initial(true));
        enable_announcements("each_dart_then_total");
        Plugin::init().unwrap();
        send_event(EMPTY_EVENT);
        send_event(ONE_DART);
        send_event(TWO_DARTS);
        send_event(REMOVED);
        let request = speech_stream::calls().submitted_ids[0].1;
        speech_stream::set_statuses(vec![
            bindings::niles::integration::speech_stream::RequestStatus { request, outcome },
        ]);
        poll_speech();
        assert_eq!(
            speech_stream::calls()
                .submitted
                .iter()
                .map(|(_, text)| text.as_str())
                .collect::<Vec<_>>(),
            ["20", "5"]
        );
        // Duplicate terminal reports for the first request cannot release its successor.
        poll_speech();
        assert_eq!(speech_stream::calls().submitted.len(), 2);
    }
}

#[test]
fn speech_poll_errors_preserve_pending_requests_and_recover() {
    setup(initial(false));
    enable_announcements("total_only");
    Plugin::init().unwrap();
    ingest_announcement_json(EMPTY_EVENT.as_bytes());
    ingest_announcement_json(ONE_DART.as_bytes());
    ingest_announcement_json(REMOVED.as_bytes());
    let request = speech_stream::calls().submitted_ids[0].1;
    speech_stream::set_status_errors([
        bindings::niles::integration::types::HostError::Unavailable("temporary".into()),
    ]);
    http::set_replies(initial(true));
    Plugin::on_timer("poll".into());
    assert!(
        render_text()
            .iter()
            .any(|line| line.contains("Speech poll failed"))
    );
    assert!(RUNTIME.with(|slot| {
        slot.borrow()
            .as_ref()
            .is_some_and(|runtime| runtime.speech_pending.contains_key(&request))
    }));

    speech_stream::set_statuses(vec![
        bindings::niles::integration::speech_stream::RequestStatus {
            request,
            outcome: bindings::niles::integration::speech_stream::Outcome::Delivered,
        },
    ]);
    http::set_replies(initial(true));
    Plugin::on_timer("poll".into());
    assert!(render_text().iter().any(|line| line.contains("Delivered")));
    assert!(
        !render_text()
            .iter()
            .any(|line| line.contains("Speech poll failed"))
    );
}
#[test]
fn speech_submission_error_does_not_hide_pending_count() {
    setup(initial(false));
    enable_announcements("total_only");
    Plugin::init().unwrap();
    ingest_announcement_json(EMPTY_EVENT.as_bytes());
    ingest_announcement_json(ONE_DART.as_bytes());
    ingest_announcement_json(REMOVED.as_bytes());
    let first = speech_stream::calls().submitted_ids[0].1;

    speech_stream::set_submit_errors([bindings::niles::integration::types::HostError::Limit(
        "submit".into(),
    )]);
    speech_stream::set_statuses(vec![
        bindings::niles::integration::speech_stream::RequestStatus {
            request: first,
            outcome: bindings::niles::integration::speech_stream::Outcome::Delivered,
        },
    ]);
    poll_speech();
    ingest_announcement_json(NEUTRAL_MOTION.as_bytes());
    ingest_announcement_json(EMPTY_EVENT.as_bytes());
    ingest_announcement_json(ONE_DART.as_bytes());
    ingest_announcement_json(REMOVED.as_bytes());
    assert!(
        render_text()
            .iter()
            .any(|line| line.contains("Speech submit failed"))
    );
    assert!(RUNTIME.with(|slot| {
        slot.borrow()
            .as_ref()
            .is_some_and(|runtime| runtime.speech_pending.is_empty())
    }));
}

#[test]
fn open_and_submit_rejections_are_visible_without_fake_terminal_success() {
    setup(initial(false));
    enable_announcements("total_only");
    speech_stream::set_open_error(Some(
        bindings::niles::integration::types::HostError::Unavailable("open".into()),
    ));
    Plugin::init().unwrap();
    assert!(
        render_text()
            .iter()
            .any(|line| line.contains("Speech stream open failed"))
    );
    setup(initial(false));
    enable_announcements("each_dart_then_total");
    Plugin::init().unwrap();
    speech_stream::set_submit_errors([bindings::niles::integration::types::HostError::Limit(
        "submit".into(),
    )]);
    ingest_announcement_json(EMPTY_EVENT.as_bytes());
    let result = ingest_announcement_json(ONE_DART.as_bytes());
    assert!(
        matches!(
            result,
            autodarts_core::local_score::JourneyResult::Dart(Some(20))
        ),
        "{result:?}"
    );
    assert!(
        render_text()
            .iter()
            .any(|line| line.contains("Speech submit failed"))
    );
    assert!(RUNTIME.with(|slot| {
        slot.borrow()
            .as_ref()
            .is_some_and(|runtime| runtime.speech_pending.is_empty())
    }));
}

fn send_event(json: &str) {
    websocket::message(
        41,
        bindings::niles::integration::websocket::Message::Text(json.into()),
    );
}
fn drain_current_speech() {
    loop {
        let request = RUNTIME.with(|slot| {
            slot.borrow()
                .as_ref()
                .and_then(|runtime| runtime.speech_in_flight)
        });
        let Some(request) = request else {
            return;
        };
        speech_stream::set_statuses(vec![
            bindings::niles::integration::speech_stream::RequestStatus {
                request,
                outcome: bindings::niles::integration::speech_stream::Outcome::Delivered,
            },
        ]);
        poll_speech();
    }
}

#[test]
fn stopped_board_discards_pending_speech_and_requires_new_baseline() {
    setup(initial(false));
    enable_announcements("each_dart_then_total");
    Plugin::init().unwrap();
    send_event(EMPTY_EVENT);
    send_event(ONE_DART);
    let session = speech_stream::calls().submitted[0].0;
    send_event(r#"{"type":"state","data":{"running":false,"numThrows":0}}"#);
    assert!(speech_stream::calls().closed.contains(&session));
    assert!(
        render_text()
            .iter()
            .any(|line| line.contains("DiscardedOnReset"))
    );
    send_event(REMOVED);
    send_event(ONE_DART);
    assert_eq!(speech_stream::calls().submitted.len(), 1);
    send_event(EMPTY_EVENT);
    send_event(ONE_DART);
    assert_eq!(speech_stream::calls().submitted.len(), 2);
}

#[test]
fn asynchronous_close_can_delay_reopening_without_replaying_speech() {
    setup(initial(false));
    enable_announcements("each_dart_then_total");
    Plugin::init().unwrap();
    send_event(EMPTY_EVENT);
    send_event(ONE_DART);
    speech_stream::set_open_error(Some(types::HostError::Unavailable(
        "speech output is busy".into(),
    )));
    websocket::status(
        41,
        bindings::niles::integration::websocket::ConnectionStatus::Overflow(1),
    );
    send_event(REMOVED);
    assert_eq!(speech_stream::calls().submitted.len(), 1);
    assert!(
        render_text()
            .iter()
            .any(|line| line.contains("reopen failed"))
    );
    speech_stream::set_open_error(None);
    http::set_replies(initial(false));
    Plugin::on_timer("poll".into());
    assert_eq!(speech_stream::calls().opened_sessions.len(), 2);
    assert_eq!(speech_stream::calls().submitted.len(), 1);
    send_event(EMPTY_EVENT);
    send_event(ONE_DART);
    send_event(REMOVED);
    assert_eq!(speech_stream::calls().submitted.len(), 2);
}

#[test]
fn overflow_closes_pending_speech_and_recovers_without_a_reconnect() {
    setup(initial(false));
    enable_announcements("each_dart_then_total");
    Plugin::init().unwrap();
    send_event(EMPTY_EVENT);
    send_event(ONE_DART);
    let old_session = speech_stream::calls().submitted[0].0;

    websocket::status(
        41,
        bindings::niles::integration::websocket::ConnectionStatus::Overflow(1),
    );
    send_event(REMOVED);
    assert_eq!(speech_stream::calls().submitted.len(), 1);
    assert_eq!(speech_stream::calls().closed, [old_session]);
    assert!(
        render_text()
            .iter()
            .any(|line| line.contains("DiscardedOnReset"))
    );

    // Overflow reports dropped callbacks; the connection remains open.
    send_event(EMPTY_EVENT);
    send_event(ONE_DART);
    send_event(REMOVED);
    let calls = speech_stream::calls();
    assert_eq!(
        calls
            .submitted
            .iter()
            .map(|(_, text)| text.as_str())
            .collect::<Vec<_>>(),
        ["20", "20"]
    );
    assert_ne!(calls.submitted[1].0, old_session);
}

#[test]
fn untrustworthy_frames_close_pending_speech_and_require_a_clean_baseline() {
    use bindings::niles::integration::websocket::Message;
    let oversized = format!(
        "{EMPTY_EVENT}{}",
        " ".repeat(64 * 1024 + 1 - EMPTY_EVENT.len())
    );
    // Character count is below the limit, but UTF-8 byte count is over it.
    let multibyte = format!(
        r#"{{"type":"camera","padding":"{}"}}"#,
        "é".repeat(32 * 1024)
    );
    for frame in [
        Message::Text(oversized),
        Message::Text(multibyte),
        Message::Text("{malformed".into()),
        Message::Binary(vec![1, 2]),
    ] {
        setup(initial(false));
        enable_announcements("each_dart_then_total");
        Plugin::init().unwrap();
        let boundary = format!("{EMPTY_EVENT}{}", " ".repeat(64 * 1024 - EMPTY_EVENT.len()));
        send_event(&boundary);
        send_event(ONE_DART);
        assert_eq!(speech_stream::calls().submitted.len(), 1);
        let old_session = speech_stream::calls().submitted[0].0;
        websocket::message(41, frame);
        assert_eq!(speech_stream::calls().closed, [old_session]);
        send_event(REMOVED);
        assert_eq!(speech_stream::calls().submitted.len(), 1);
        send_event(EMPTY_EVENT);
        send_event(ONE_DART);
        send_event(REMOVED);
        let calls = speech_stream::calls();
        assert_eq!(calls.submitted.len(), 2);
        assert_ne!(calls.submitted[1].0, old_session);
    }
}

#[test]
fn completed_speech_is_collected_during_events_before_the_slow_timer() {
    setup(initial(false));
    enable_announcements("total_only");
    Plugin::init().unwrap();
    for _ in 0..40 {
        send_event(NEUTRAL_MOTION);
        send_event(EMPTY_EVENT);
        send_event(ONE_DART);
        send_event(REMOVED);
        let request = speech_stream::calls().submitted_ids.last().unwrap().1;
        speech_stream::set_statuses(vec![
            bindings::niles::integration::speech_stream::RequestStatus {
                request,
                outcome: bindings::niles::integration::speech_stream::Outcome::Delivered,
            },
        ]);
    }
    send_event(EMPTY_EVENT);
    let lines = render_text();
    assert!(
        !lines.iter().any(|line| line.contains("Speech pending:")),
        "{lines:?}"
    );
    let history = lines
        .iter()
        .find(|line| line.contains("Speech terminal history:"))
        .unwrap();
    assert_eq!(history.matches("Delivered").count(), MAX_SPEECH_HISTORY);
    assert_eq!(speech_stream::calls().submitted.len(), 40);
}

#[test]
fn queue_overflow_discards_pending_work_and_requires_fresh_baseline() {
    setup(initial(true));
    enable_announcements("total_only");
    Plugin::init().unwrap();
    for _ in 0..=MAX_PENDING_SPEECH {
        send_event(NEUTRAL_MOTION);
        send_event(EMPTY_EVENT);
        send_event(ONE_DART);
        send_event(REMOVED);
    }
    let calls = speech_stream::calls();
    assert_eq!(calls.submitted.len(), 1);
    assert_eq!(calls.closed, [calls.submitted[0].0]);
    set_terminal(calls.submitted_ids[0].1);
    poll_speech();
    send_event(ONE_DART);
    send_event(REMOVED);
    assert_eq!(speech_stream::calls().submitted.len(), 1);
    send_event(NEUTRAL_MOTION);
    send_event(EMPTY_EVENT);
    send_event(ONE_DART);
    send_event(REMOVED);
    assert_eq!(speech_stream::calls().submitted.len(), 2);
    assert_ne!(speech_stream::calls().submitted[1].0, calls.submitted[0].0);
}

#[test]
fn room_change_retains_discard_evidence_and_ignores_old_results() {
    setup(initial(false));
    enable_announcements("each_dart_then_total");
    Plugin::init().unwrap();
    send_event(EMPTY_EVENT);
    send_event(ONE_DART);
    let old_request = speech_stream::calls().submitted_ids[0].1;
    config::set("room", config::SettingValue::Text("office".into()));
    http::set_replies(initial(false));
    Plugin::init().unwrap();
    assert!(
        render_text()
            .iter()
            .any(|line| line.contains("DiscardedOnReset"))
    );
    speech_stream::set_statuses(vec![
        bindings::niles::integration::speech_stream::RequestStatus {
            request: old_request,
            outcome: bindings::niles::integration::speech_stream::Outcome::Delivered,
        },
    ]);
    http::set_replies(initial(false));
    Plugin::on_timer("poll".into());
    assert!(!render_text().iter().any(|line| line.contains("Delivered")));
    websocket::message(
        42,
        bindings::niles::integration::websocket::Message::Text(REMOVED.into()),
    );
    assert_eq!(speech_stream::calls().submitted.len(), 1);
}

#[test]
fn speech_enablement_without_announcement_intent_does_not_open_a_session() {
    setup(initial(false));
    config::set("speech_stream_enabled", config::SettingValue::Boolean(true));
    Plugin::init().unwrap();
    assert!(speech_stream::calls().opened_rooms.is_empty());
}

#[test]
fn stale_progress_callback_cannot_disable_current_request_progress() {
    setup(initial(true));
    enable_announcements("each_dart_then_total");
    Plugin::init().unwrap();
    send_event(EMPTY_EVENT);
    send_event(ONE_DART);
    send_event(TWO_DARTS);
    send_event(REMOVED);
    let old_timer = latest_timer("speech-progress:");
    set_terminal(speech_stream::calls().submitted_ids[0].1);
    Plugin::on_timer(old_timer.clone());
    assert_eq!(speech_stream::calls().submitted.len(), 2);
    let current_timer = latest_timer("speech-progress:");
    Plugin::on_timer(old_timer);
    set_terminal(speech_stream::calls().submitted_ids[1].1);
    Plugin::on_timer(current_timer);
    assert_eq!(speech_stream::calls().submitted.len(), 3);
}

#[test]
fn deadline_timer_failure_discards_speech_generation() {
    setup(initial(false));
    enable_announcements("each_dart_then_total");
    Plugin::init().unwrap();
    send_event(EMPTY_EVENT);
    send_event(ONE_DART);
    let session = speech_stream::calls().submitted[0].0;
    test_host::niles::integration::timers::fail_after(true);
    send_event(TWO_DARTS);
    send_event(
        r#"{"type":"state","data":{"running":true,"numThrows":3,"throws":[{"segment":{"number":20,"multiplier":1}},{"segment":{"number":5,"multiplier":1}},{"segment":{"number":1,"multiplier":1}}]}}"#,
    );
    assert_eq!(speech_stream::calls().closed, [session]);
    assert!(
        render_text()
            .iter()
            .any(|line| line.contains("Total deadline timer failed"))
    );
    assert!(RUNTIME.with(|slot| {
        slot.borrow().as_ref().is_some_and(|runtime| {
            runtime.speech_pending.is_empty()
                && runtime.speech_queue.is_empty()
                && runtime.speech_in_flight.is_none()
                && runtime.deadline_timer.is_none()
        })
    }));
    send_event(REMOVED);
    assert_eq!(speech_stream::calls().submitted.len(), 1);
}

#[test]
fn callback_ingress_replays_exact_capture_fifo_and_deadline_timer() {
    let capture: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../crates/autodarts-core/tests/fixtures/board-manager-jvl14-three-round-reduced-capture.json"
    ))
    .unwrap();
    for mode in ["each_dart_then_total", "total_only"] {
        setup(initial(false));
        enable_announcements(mode);
        Plugin::init().unwrap();
        websocket::status(
            41,
            bindings::niles::integration::websocket::ConnectionStatus::Connected,
        );
        for envelope in capture.as_array().unwrap() {
            send_event(&serde_json::to_string(envelope).unwrap());
        }
        assert_eq!(
            speech_stream::calls()
                .submitted
                .iter()
                .map(|(_, text)| text.as_str())
                .collect::<Vec<_>>(),
            [if mode == "total_only" { "87" } else { "60" }]
        );
        drain_current_speech();
        assert_eq!(
            speech_stream::calls()
                .submitted
                .iter()
                .map(|(_, text)| text.as_str())
                .collect::<Vec<_>>(),
            if mode == "total_only" {
                &["87", "60", "30"][..]
            } else {
                &[
                    "60", "20", "7", "87", "40", "20", "0", "60", "12", "18", "0", "30",
                ][..]
            }
        );
    }

    setup(initial(false));
    enable_announcements("total_only");
    Plugin::init().unwrap();
    websocket::status(
        41,
        bindings::niles::integration::websocket::ConnectionStatus::Connected,
    );
    send_event(EMPTY_EVENT);
    send_event(ONE_DART);
    send_event(TWO_DARTS);
    send_event(
        r#"{"type":"state","data":{"running":true,"numThrows":3,"throws":[{"segment":{"number":20,"multiplier":1}},{"segment":{"number":5,"multiplier":1}},{"segment":{"number":1,"multiplier":1}}]}}"#,
    );
    let timer = test_host::niles::integration::timers::after_calls()
        .into_iter()
        .find(|(id, _)| id.starts_with("total-deadline:"))
        .map(|(id, _)| id)
        .expect("registered deadline");
    Plugin::on_timer(timer);
    assert_eq!(speech_stream::calls().submitted, [(7, "26".into())]);
}

#[test]
fn progress_polling_preserves_gate_through_missing_pending_and_error_outcomes() {
    setup(initial(true));
    enable_announcements("each_dart_then_total");
    Plugin::init().unwrap();
    send_event(EMPTY_EVENT);
    send_event(ONE_DART);
    send_event(TWO_DARTS);
    let first = speech_stream::calls().submitted_ids[0].1;
    Plugin::on_timer(latest_timer("speech-progress:"));
    assert_eq!(speech_stream::calls().submitted.len(), 1);
    speech_stream::set_statuses(vec![
        bindings::niles::integration::speech_stream::RequestStatus {
            request: first,
            outcome: bindings::niles::integration::speech_stream::Outcome::Pending,
        },
        bindings::niles::integration::speech_stream::RequestStatus {
            request: first + 1,
            outcome: bindings::niles::integration::speech_stream::Outcome::Delivered,
        },
    ]);
    Plugin::on_timer(latest_timer("speech-progress:"));
    assert_eq!(speech_stream::calls().submitted.len(), 1);
    speech_stream::set_status_errors([types::HostError::Unavailable("temporary".into())]);
    Plugin::on_timer(latest_timer("speech-progress:"));
    assert_eq!(speech_stream::calls().submitted.len(), 1);
    set_terminal(first);
    Plugin::on_timer(latest_timer("speech-progress:"));
    assert_eq!(speech_stream::calls().submitted.len(), 2);
}

#[test]
fn connected_callback_invalidates_queued_work_before_new_baseline() {
    setup(initial(true));
    enable_announcements("each_dart_then_total");
    Plugin::init().unwrap();
    send_event(EMPTY_EVENT);
    send_event(ONE_DART);
    send_event(TWO_DARTS);
    let calls = speech_stream::calls();
    websocket::status(
        41,
        bindings::niles::integration::websocket::ConnectionStatus::Connected,
    );
    set_terminal(calls.submitted_ids[0].1);
    poll_speech();
    assert_eq!(speech_stream::calls().closed, [calls.submitted[0].0]);
    assert_eq!(speech_stream::calls().submitted.len(), 1);
    send_event(TWO_DARTS);
    send_event(EMPTY_EVENT);
    send_event(ONE_DART);
    assert_eq!(speech_stream::calls().submitted.len(), 2);
}

#[test]
fn stopped_status_poll_discards_partial_removal_candidate() {
    setup(initial(true));
    enable_announcements("total_only");
    Plugin::init().unwrap();
    send_event(EMPTY_EVENT);
    send_event(ONE_DART);
    send_event(EMPTY_EVENT);
    http::set_replies(initial(false));
    Plugin::on_timer("poll".into());
    send_event(REMOVED);
    assert!(speech_stream::calls().submitted.is_empty());
    send_event(NEUTRAL_MOTION);
    send_event(EMPTY_EVENT);
    send_event(ONE_DART);
    send_event(REMOVED);
    assert_eq!(speech_stream::calls().submitted.len(), 1);
}

const NEUTRAL_MOTION: &str =
    r#"{"type":"motion_state","data":{"isTakeoutPartial":false,"isTakeoutFull":false}}"#;
const THREE_DARTS: &str = r#"{"type":"state","data":{"running":true,"numThrows":3,"throws":[{"segment":{"number":20,"multiplier":1}},{"segment":{"number":5,"multiplier":1}},{"segment":{"number":1,"multiplier":1}}]}}"#;

fn latest_timer(prefix: &str) -> String {
    test_host::niles::integration::timers::after_calls()
        .into_iter()
        .rev()
        .find(|(id, _)| id.starts_with(prefix))
        .unwrap()
        .0
}

fn set_terminal(request: u64) {
    speech_stream::set_statuses(vec![
        bindings::niles::integration::speech_stream::RequestStatus {
            request,
            outcome: bindings::niles::integration::speech_stream::Outcome::Delivered,
        },
    ]);
}

#[test]
fn invalidating_ingress_does_not_drain_a_just_completed_request() {
    for event in [
        "{malformed",
        r#"{"type":"state","data":{"running":false,"numThrows":0}}"#,
        r#"{"type":"state","data":{"connected":false,"running":true,"numThrows":0}}"#,
    ] {
        setup(initial(true));
        enable_announcements("each_dart_then_total");
        Plugin::init().unwrap();
        send_event(EMPTY_EVENT);
        send_event(ONE_DART);
        send_event(TWO_DARTS);
        let calls = speech_stream::calls();
        set_terminal(calls.submitted_ids[0].1);
        send_event(event);
        assert_eq!(speech_stream::calls().submitted.len(), 1, "{event}");
        assert_eq!(speech_stream::calls().closed, [calls.submitted[0].0]);
    }
}

#[test]
fn stopped_status_poll_cancels_before_draining_speech() {
    for state in [
        r#"{"connected":true,"running":false}"#,
        r#"{"connected":false,"running":true}"#,
    ] {
        setup(initial(true));
        enable_announcements("each_dart_then_total");
        Plugin::init().unwrap();
        send_event(EMPTY_EVENT);
        send_event(ONE_DART);
        send_event(TWO_DARTS);
        send_event(THREE_DARTS);
        let timer = latest_timer("total-deadline:");
        let calls = speech_stream::calls();
        set_terminal(calls.submitted_ids[0].1);
        http::set_replies([response("fixture-version"), response(state)]);
        Plugin::on_timer("poll".into());
        Plugin::on_timer(timer);
        assert_eq!(speech_stream::calls().submitted.len(), 1);
        assert_eq!(speech_stream::calls().closed, [calls.submitted[0].0]);
        send_event(ONE_DART);
        assert_eq!(speech_stream::calls().submitted.len(), 1);
        send_event(EMPTY_EVENT);
        send_event(ONE_DART);
        assert_eq!(speech_stream::calls().submitted.len(), 2);
    }
}

#[test]
fn acknowledged_stop_cancels_even_before_observation_catches_up() {
    for status in [204, 500] {
        setup(initial(true));
        enable_announcements("each_dart_then_total");
        Plugin::init().unwrap();
        send_event(EMPTY_EVENT);
        send_event(ONE_DART);
        send_event(TWO_DARTS);
        http::set_replies([
            Ok(http::Response {
                status,
                headers: vec![],
                body: vec![],
            }),
            response(r#"{"connected":true,"running":true}"#),
        ]);
        assert_eq!(
            Plugin::set_device("living_room/dartboard".into(), desired(Some(false))).is_err(),
            status == 500
        );
        set_terminal(speech_stream::calls().submitted_ids[0].1);
        poll_speech();
        assert_eq!(
            speech_stream::calls().submitted.len(),
            if status == 204 { 1 } else { 2 }
        );
    }
}

#[test]
fn progress_timer_failure_cancels_work_and_recovers_at_new_baseline() {
    setup(initial(true));
    enable_announcements("each_dart_then_total");
    Plugin::init().unwrap();
    test_host::niles::integration::timers::fail_after(true);
    send_event(EMPTY_EVENT);
    send_event(ONE_DART);
    let calls = speech_stream::calls();
    assert_eq!(calls.closed, [calls.submitted[0].0]);
    test_host::niles::integration::timers::fail_after(false);
    send_event(TWO_DARTS);
    send_event(REMOVED);
    assert_eq!(speech_stream::calls().submitted.len(), 1);
    send_event(NEUTRAL_MOTION);
    send_event(EMPTY_EVENT);
    send_event(ONE_DART);
    assert_eq!(speech_stream::calls().submitted.len(), 2);
}

#[test]
fn stopped_plugin_deadline_cannot_complete_a_restarted_round() {
    setup(initial(true));
    enable_announcements("total_only");
    Plugin::init().unwrap();
    for event in [EMPTY_EVENT, ONE_DART, TWO_DARTS, THREE_DARTS] {
        send_event(event);
    }
    let old_timer = latest_timer("total-deadline:");
    Plugin::stop();
    http::set_replies(initial(true));
    Plugin::init().unwrap();
    for event in [EMPTY_EVENT, ONE_DART, TWO_DARTS, THREE_DARTS] {
        websocket::message(
            42,
            bindings::niles::integration::websocket::Message::Text(event.into()),
        );
    }
    let new_timer = latest_timer("total-deadline:");
    Plugin::on_timer(old_timer);
    assert!(speech_stream::calls().submitted.is_empty());
    Plugin::on_timer(new_timer);
    assert_eq!(speech_stream::calls().submitted.len(), 1);
    assert_eq!(speech_stream::calls().submitted[0].1, "26");
}

fn render_text() -> Vec<String> {
    let tree = Plugin::render(
        "board".into(),
        bindings::niles::integration::ui::RenderContext {
            room: None,
            person: None,
        },
    )
    .unwrap();
    tree.nodes
        .iter()
        .filter_map(|node| match &node.kind {
            NodeKind::Text(text) => Some(text.value.clone()),
            _ => None,
        })
        .collect()
}
