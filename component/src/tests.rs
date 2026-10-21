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
fn composed_device_start_stop_preserve_identity_and_exact_provider_calls() {
    setup(initial(false));
    Plugin::init().unwrap();
    http::set_replies([
        Ok(http::Response {
            status: 204,
            headers: vec![],
            body: vec![],
        }),
        response(r#"{"connected":true,"running":true}"#),
    ]);
    assert!(Plugin::set_device("living_room/dartboard".into(), desired(Some(true))).is_ok());
    let recorded = http::requests();
    assert_eq!(
        recorded
            .iter()
            .map(|r| (r.method, r.url.as_str()))
            .collect::<Vec<_>>(),
        [
            (http::Method::Put, "http://board.local:3180/api/start"),
            (http::Method::Get, "http://board.local:3180/api/state"),
        ]
    );
    let calls = devices::calls();
    assert_eq!(calls.upserts.last().unwrap().id, "living_room/dartboard");
    assert_eq!(calls.upserts.last().unwrap().state.on, Some(true));

    http::set_replies([
        Ok(http::Response {
            status: 204,
            headers: vec![],
            body: vec![],
        }),
        response(r#"{"connected":true,"running":false}"#),
    ]);
    assert!(Plugin::set_device("living_room/dartboard".into(), desired(Some(false))).is_ok());
    assert_eq!(http::requests()[0].method, http::Method::Put);
    assert_eq!(http::requests()[0].url, "http://board.local:3180/api/stop");
    assert_eq!(
        devices::calls().upserts.last().unwrap().state.on,
        Some(false)
    );
}

#[test]
fn composed_acknowledged_but_observed_false_returns_pending_and_keeps_actual_state() {
    setup(initial(false));
    Plugin::init().unwrap();
    http::set_replies([
        Ok(http::Response {
            status: 200,
            headers: vec![],
            body: vec![],
        }),
        response(r#"{"running":false}"#),
    ]);
    assert!(
        Plugin::set_device("living_room/dartboard".into(), desired(Some(true)).clone()).is_err()
    );
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
fn composed_timeout_and_failure_are_errors_not_optimistic_state() {
    setup(initial(false));
    Plugin::init().unwrap();
    http::fail_next(http::HttpError::Timeout);
    assert!(Plugin::set_device("living_room/dartboard".into(), desired(Some(true))).is_err());
    assert_eq!(
        devices::calls().upserts.last().unwrap().state.on,
        Some(false)
    );
    assert_eq!(http::requests().len(), 3);

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

    http::set_replies([
        Ok(http::Response {
            status: 200,
            headers: vec![],
            body: vec![],
        }),
        response(r#"{"running":true}"#),
    ]);
    assert!(Plugin::set_device("living_room/dartboard".into(), desired(Some(true))).is_ok());
    assert_eq!(
        devices::calls().upserts.last().unwrap().state.on,
        Some(true)
    );
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
    assert_eq!(speech_stream::calls().submitted, [(7, "20".into())]);
    speech_stream::set_statuses(vec![
        bindings::niles::integration::speech_stream::RequestStatus {
            request: 11,
            outcome: bindings::niles::integration::speech_stream::Outcome::Pending,
        },
    ]);
    http::set_replies(initial(false));
    Plugin::on_timer("poll".into());
    assert!(render_text().iter().any(|line| {
        line.contains("request 11")
            && line.contains("Pending")
            && line.contains("epoch:0:collection:")
    }));

    speech_stream::set_statuses(vec![
        bindings::niles::integration::speech_stream::RequestStatus {
            request: 11,
            outcome: bindings::niles::integration::speech_stream::Outcome::Replaced,
        },
    ]);
    http::set_replies(initial(false));
    Plugin::on_timer("poll".into());
    let lines = render_text();
    assert!(
        lines
            .iter()
            .any(|line| line.contains("request 11") && line.contains("Replaced"))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("epoch:0:collection:"))
    );

    config::set("room", config::SettingValue::Text("office".into()));
    config::set(
        "speech_stream_enabled",
        config::SettingValue::Boolean(false),
    );
    http::set_replies(initial(false));
    Plugin::init().unwrap();
    assert_eq!(speech_stream::calls().closed, [7]);
    assert_eq!(speech_stream::calls().opened_rooms, ["living_room"]);
    assert!(
        render_text()
            .iter()
            .any(|line| line == "Announcement room: office")
    );
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
    assert_eq!(
        speech_stream::calls().submitted,
        [(7, "20".into()), (7, "5".into()), (7, "25".into())]
    );
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
    assert_eq!(speech_stream::calls().submitted, [(7, "20".into())]);
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
