use autodarts_core::{
    Availability, BoardState, Config, HttpFailure, HttpResponse, Snapshot, Transport,
    detection::{
        CommandTransport, DetectionCommand, DetectionDispatch, dispatch_detection, poll_state,
    },
    poll,
};
#[cfg(not(test))]
use bindings::niles::integration::speech_stream;
use bindings::niles::integration::types::{self, Device, DeviceClass, DeviceState, Reading};
#[cfg(not(test))]
use bindings::niles::integration::ui;
#[cfg(test)]
use bindings::niles::integration::ui::{ActionResult, ActionValue};
use bindings::niles::integration::ui::{Node, NodeKind, TextBlock, UiTree};
#[cfg(not(test))]
use bindings::niles::integration::websocket;
#[cfg(not(test))]
use bindings::niles::integration::{
    config, devices,
    http::{self, Method, Request},
    log, timers,
};
use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::time::{Duration, Instant};
#[cfg(test)]
use test_host::niles::integration::speech_stream;
#[cfg(test)]
use test_host::niles::integration::ui;
#[cfg(test)]
use test_host::niles::integration::websocket;
#[cfg(test)]
use test_host::niles::integration::{
    config, devices,
    http::{self, Method, Request},
    log, timers,
};
#[cfg(test)]
mod test_host;
#[cfg(test)]
mod tests;
#[allow(clippy::too_many_arguments)]
mod bindings {
    wit_bindgen::generate!({ world: "plugin", path: "wit/niles-integration.wit" });
}
use bindings::*;
struct Plugin;
export!(Plugin);
thread_local! { static RUNTIME: RefCell<Option<Runtime>> = const { RefCell::new(None) }; }
thread_local! { static SPEECH_GENERATION: std::cell::Cell<u64> = const { std::cell::Cell::new(0) }; }
thread_local! { static DETECTION_GENERATION: std::cell::Cell<u64> = const { std::cell::Cell::new(0) }; }

fn detection_now() -> Instant {
    #[cfg(test)]
    return test_host::clock::now();
    #[cfg(not(test))]
    Instant::now()
}
struct Runtime {
    announcements: autodarts_core::AnnouncementSettings,
    announcement_session: autodarts_core::AnnouncementSession,
    policy_event: Option<autodarts_core::AnnouncementEvent>,
    config: Config,
    snapshot: Snapshot,
    observation_at: Option<Instant>,
    command_status: Option<String>,
    detection: Option<DetectionOperation>,
    detection_poll_timer: Option<String>,
    detection_expiry_timer: Option<String>,
    detection_readback: Option<String>,
    speech_session: Option<u32>,
    speech_reopen_pending: bool,
    speech_generation: u64,
    speech_pending: BTreeMap<u64, SpeechRequest>,
    speech_in_flight: Option<u64>,
    speech_queue: VecDeque<SpeechRequest>,
    speech_history: VecDeque<SpeechCompletion>,
    speech_poll_error: Option<String>,
    speech_submission_error: Option<String>,
    speech_enabled: bool,
    deadline_timer: Option<(autodarts_core::RoundToken, String)>,
    speech_progress_timer: Option<String>,
    websocket_id: Option<u32>,
    websocket_status: Option<String>,
}

#[derive(Clone, Debug)]
struct DetectionOperation {
    token: u64,
    desired: bool,
    dispatch: DetectionDispatch,
    started_at: Instant,
    expired: bool,
    fresh_definitive_observation: Option<bool>,
}

#[derive(Clone, Debug)]
struct SpeechRequest {
    generation: u64,
    logical_key: String,
    kind: SpeechRequestKind,
    text: String,
}

#[derive(Clone, Debug)]
struct SpeechCompletion {
    generation: u64,
    host_request: u64,
    logical_key: String,
    kind: SpeechRequestKind,
    text: String,
    outcome: SpeechOutcome,
}

const MAX_PENDING_SPEECH: usize = 32;
const MAX_SPEECH_HISTORY: usize = 32;
const DETECTION_POLL_SECONDS: u32 = 1;
const DETECTION_MONITOR_SECONDS: u32 = 30;

#[derive(Clone, Debug, PartialEq, Eq)]
enum SpeechRequestKind {
    Dart,
    Total,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum SpeechOutcome {
    Pending,
    Delivered,
    Busy,
    Quiet,
    Expired,
    Failed,
    Cancelled,
    Replaced,
    RateLimited,
    NoSatellite,
    DiscardedOnReset,
}
fn setting_text(key: &str) -> Result<String, String> {
    match config::get(key) {
        Some(config::SettingValue::Text(value)) if !value.trim().is_empty() => Ok(value),
        _ => Err(format!("Required setting {key} is missing.")),
    }
}
fn settings() -> Result<Config, String> {
    let address = setting_text("address")?;
    let room = setting_text("room")?;
    let interval = match config::get("poll_interval") {
        None => 15,
        Some(config::SettingValue::Number(value)) => {
            if !value.is_finite() || value.fract() != 0.0 {
                return Err("Invalid polling interval.".into());
            }
            u32::try_from(value as i64).map_err(|_| "Invalid polling interval.".to_string())?
        }
        Some(_) => return Err("poll_interval must be a number.".into()),
    };
    Config::new(
        &address,
        &room,
        interval,
        autodarts_core::DEFAULT_TIMEOUT_MS,
    )
    .map_err(str::to_owned)
}
fn announcement_settings(room: &str) -> Result<autodarts_core::AnnouncementSettings, String> {
    let enabled = match config::get("announcements_enabled") {
        None => false,
        Some(config::SettingValue::Boolean(value)) => value,
        _ => return Err("announcements_enabled must be boolean".into()),
    };
    let text = |key: &str| -> Result<Option<String>, String> {
        match config::get(key) {
            None => Ok(None),
            Some(config::SettingValue::Text(value)) if value.trim().is_empty() => Ok(None),
            Some(config::SettingValue::Text(value)) => Ok(Some(value)),
            _ => Err(format!("{key} must be text")),
        }
    };
    let mode = text("announcement_mode")?;
    autodarts_core::AnnouncementSettings::new(enabled, mode.as_deref(), room).map_err(str::to_owned)
}
fn speech_enabled() -> Result<bool, String> {
    match config::get("speech_stream_enabled") {
        Some(config::SettingValue::Boolean(value)) => Ok(value),
        None => Ok(false),
        _ => Err("speech_stream_enabled must be boolean".into()),
    }
}
fn device_id(runtime: &Runtime) -> String {
    format!("{}/dartboard", runtime.config.room)
}
fn report(runtime: &Runtime) -> Result<(), String> {
    let id = device_id(runtime);
    let device = Device {
        id: id.clone(),
        class: DeviceClass::Outlet,
        light: types::LightCapabilities {
            color_temp: false,
            rgb: false,
        },
        controllable: true,
        state: DeviceState {
            on: match &runtime.snapshot.state {
                BoardState::Observed { running, .. } => *running,
                BoardState::Unknown => None,
            },
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
            readings: readings(&runtime.snapshot),
        },
    };
    devices::upsert(&device)
        .map_err(|_| "Could not register the Autodarts status device.".to_string())?;
    devices::set_available(
        &id,
        runtime.snapshot.availability == Availability::Available,
    )
    .map_err(|_| "Could not report Autodarts availability.".to_string())
}
fn readings(snapshot: &Snapshot) -> Vec<Reading> {
    let mut values = Vec::new();
    if let BoardState::Observed {
        connected,
        running,
        num_throws,
        ..
    } = &snapshot.state
    {
        for (name, value) in [("connected", *connected), ("running", *running)] {
            if let Some(value) = value {
                values.push(Reading {
                    name: name.into(),
                    value: f64::from(value),
                    unit: None,
                });
            }
        }
        if let Some(count) = num_throws {
            values.push(Reading {
                name: "num_throws".into(),
                value: f64::from(*count),
                unit: Some("count".into()),
            });
        }
    }
    values
}
fn diagnostics_text(
    snapshot: &Snapshot,
    command_status: Option<&str>,
    detection_readback: Option<&str>,
) -> Vec<String> {
    let mut lines = vec![match snapshot.availability {
        Availability::Available => "Board Manager: reachable".into(),
        Availability::Unavailable => "Board Manager: unavailable".into(),
        Availability::Unknown => "Board Manager: unknown".into(),
    }];
    if let Some(version) = &snapshot.version {
        lines.push(format!("Version: {version}"));
    }
    match &snapshot.state {
        BoardState::Unknown => lines.push("Board state: unknown".into()),
        BoardState::Observed {
            connected,
            running,
            status,
            event,
            num_throws,
        } => {
            for (label, value) in [("Connected", connected), ("Running", running)] {
                if let Some(value) = value {
                    lines.push(format!("{label}: {value}"));
                }
            }
            if let Some(value) = status {
                lines.push(format!("Status: {value}"));
            }
            if let Some(value) = event {
                lines.push(format!("Event: {value}"));
            }
            if let Some(value) = num_throws {
                lines.push(format!("Observed throws: {value}"));
            }
        }
    }
    if let Some(status) = command_status {
        lines.push(format!("Detection command: {status}"));
    }
    if let Some(readback) = detection_readback {
        lines.push(format!("Detection readback: {readback}"));
    }
    if let Some(error) = &snapshot.diagnostic {
        lines.push(format!(
            "Diagnostic: {} {}{}",
            error.stage,
            error.kind,
            error
                .http_status
                .map(|code| format!(" (HTTP {code})"))
                .unwrap_or_default()
        ));
    }
    lines
}

fn normalize_snapshot(snapshot: Snapshot) -> Snapshot {
    if matches!(
        &snapshot.state,
        BoardState::Observed {
            connected: Some(false),
            ..
        }
    ) {
        return Snapshot::unknown("state".into(), "disconnected".into(), None);
    }
    if is_starting(&snapshot) && observed_running(&snapshot) != Some(false) {
        return Snapshot::unknown("state".into(), "contradictory_starting".into(), None);
    }
    snapshot
}

fn observed_running(snapshot: &Snapshot) -> Option<bool> {
    match &snapshot.state {
        BoardState::Observed { running, .. } => *running,
        BoardState::Unknown => None,
    }
}

fn is_starting(snapshot: &Snapshot) -> bool {
    matches!(
        &snapshot.state,
        BoardState::Observed {
            status: Some(status),
            ..
        } if status == "Starting"
    ) || matches!(
        &snapshot.state,
        BoardState::Observed {
            event: Some(event),
            ..
        } if event == "Starting"
    )
}

fn confirms(snapshot: &Snapshot, desired: bool) -> bool {
    if snapshot.availability != Availability::Available || is_starting(snapshot) {
        return false;
    }
    match &snapshot.state {
        BoardState::Observed {
            connected: Some(false),
            ..
        } => false,
        BoardState::Observed { running, .. } => running == &Some(desired),
        BoardState::Unknown => false,
    }
}

fn dispatch_status(dispatch: DetectionDispatch) -> String {
    match dispatch {
        DetectionDispatch::Acknowledged => "acknowledged".into(),
        DetectionDispatch::Uncertain(reason) => format!("dispatch uncertain ({reason})"),
    }
}

fn pending_status(operation: &DetectionOperation, snapshot: &Snapshot) -> String {
    let action = if operation.desired { "start" } else { "stop" };
    let dispatch = dispatch_status(operation.dispatch);
    let state = if is_starting(snapshot) {
        "Starting"
    } else {
        match observed_running(snapshot) {
            Some(value) if value == operation.desired => "observed target state",
            Some(true) => "still running",
            Some(false) => "still stopped",
            None => "awaiting observation",
        }
    };
    let elapsed = detection_now()
        .saturating_duration_since(operation.started_at)
        .as_secs();
    if operation.expired {
        format!(
            "{action} monitoring expired; not confirmed ({state}); {dispatch}; operation {} elapsed {elapsed}s",
            operation.token
        )
    } else {
        format!(
            "{action} {dispatch}; {state}; operation {} elapsed {elapsed}s",
            operation.token
        )
    }
}

fn cancel_detection_timers(runtime: &mut Runtime) {
    if let Some(id) = runtime.detection_poll_timer.take() {
        timers::cancel(&id);
    }
    if let Some(id) = runtime.detection_expiry_timer.take() {
        timers::cancel(&id);
    }
}

fn cancel_detection(runtime: &mut Runtime) {
    cancel_detection_timers(runtime);
    runtime.detection = None;
}

fn expire_detection(runtime: &mut Runtime) -> bool {
    let Some(operation) = runtime.detection.as_mut() else {
        return false;
    };
    if !operation.expired
        && detection_now().saturating_duration_since(operation.started_at)
            >= Duration::from_secs(u64::from(DETECTION_MONITOR_SECONDS))
    {
        operation.expired = true;
        // A pre-expiry stopped sample cannot authorize a new attempt later.
        operation.fresh_definitive_observation = None;
        runtime.command_status = Some(pending_status(operation, &runtime.snapshot));
        runtime.detection_readback = Some("monitoring window expired without confirmation".into());
        cancel_detection_timers(runtime);
    }
    runtime
        .detection
        .as_ref()
        .is_some_and(|operation| operation.expired)
}

fn schedule_detection(runtime: &mut Runtime) {
    if expire_detection(runtime) {
        return;
    }
    let Some(operation) = runtime.detection.as_ref() else {
        return;
    };
    let token = operation.token;
    let remaining = Duration::from_secs(u64::from(DETECTION_MONITOR_SECONDS))
        .saturating_sub(detection_now().saturating_duration_since(operation.started_at));
    let expiry_seconds = remaining.as_secs() as u32 + u32::from(remaining.subsec_nanos() != 0);
    if runtime.detection_poll_timer.is_none() {
        let id = format!("detection-poll:{token}");
        match timers::after(&id, DETECTION_POLL_SECONDS) {
            Ok(()) => runtime.detection_poll_timer = Some(id),
            Err(_) => {
                runtime.detection_readback = Some("poll timer registration failed".into());
                runtime.command_status = Some(format!(
                    "{}; monitoring unavailable",
                    pending_status(runtime.detection.as_ref().unwrap(), &runtime.snapshot)
                ));
            }
        }
    }
    if runtime.detection_expiry_timer.is_none() {
        let id = format!("detection-expiry:{token}");
        match timers::after(&id, expiry_seconds) {
            Ok(()) => runtime.detection_expiry_timer = Some(id),
            Err(_) => {
                runtime.detection_readback =
                    Some("monitoring expiry timer registration failed".into());
                runtime.command_status = Some(format!(
                    "{}; monitoring unavailable",
                    pending_status(runtime.detection.as_ref().unwrap(), &runtime.snapshot)
                ));
            }
        }
    }
}

fn finish_detection(runtime: &mut Runtime, desired: bool) {
    cancel_detection(runtime);
    runtime.command_status = Some(if desired {
        "running confirmed".into()
    } else {
        "stopped confirmed".into()
    });
    runtime.detection_readback = None;
    if desired {
        reopen_speech(runtime);
    } else {
        invalidate_speech(runtime, "detection stop observed");
    }
}

fn apply_detection_snapshot(runtime: &mut Runtime, snapshot: Snapshot) -> Result<(), String> {
    expire_detection(runtime);
    if matches!(
        &snapshot.state,
        BoardState::Observed {
            connected: Some(false),
            ..
        }
    ) {
        invalidate_speech(runtime, "observed board disconnect");
    }
    let snapshot = normalize_snapshot(snapshot);
    if snapshot.availability != Availability::Available || observed_running(&snapshot).is_none() {
        runtime.observation_at = None;
        if let Some(operation) = runtime.detection.as_mut() {
            operation.fresh_definitive_observation = None;
        }
        runtime.detection_readback = Some(snapshot.diagnostic.as_ref().map_or_else(
            || "state readback unavailable".into(),
            |error| format!("{} {}", error.stage, error.kind),
        ));
        runtime.snapshot.availability = Availability::Unavailable;
        runtime.snapshot.diagnostic = snapshot.diagnostic;
        if let Some(operation) = runtime.detection.as_mut() {
            runtime.command_status = Some(pending_status(operation, &runtime.snapshot));
        }
        schedule_detection(runtime);
        return report(runtime);
    }
    runtime.observation_at = Some(detection_now());

    runtime.snapshot = snapshot;
    runtime.detection_readback = None;
    let desired = runtime
        .detection
        .as_ref()
        .map(|operation| operation.desired);
    let confirmed = desired.is_some_and(|desired| confirms(&runtime.snapshot, desired));
    if let Some(operation) = runtime.detection.as_mut() {
        operation.fresh_definitive_observation = if operation.expired
            && observed_running(&runtime.snapshot).is_some_and(|on| confirms(&runtime.snapshot, on))
        {
            observed_running(&runtime.snapshot)
        } else {
            None
        };
        if !confirmed {
            let status = pending_status(operation, &runtime.snapshot);
            runtime.command_status = Some(status);
        }
    }
    if confirmed {
        finish_detection(runtime, desired.expect("confirmed operation"));
    } else {
        schedule_detection(runtime);
    }
    report(runtime)
}

fn on_detection_timer(runtime: &mut Runtime, id: &str) {
    let Some(operation) = runtime.detection.as_ref() else {
        return;
    };
    let token = operation.token;
    if id == format!("detection-expiry:{token}") {
        if runtime.detection_expiry_timer.as_deref() != Some(id) {
            return;
        }
        expire_detection(runtime);
        let _ = report(runtime);
        ui::invalidate("board");
        return;
    }
    if id != format!("detection-poll:{token}")
        || runtime.detection_poll_timer.as_deref() != Some(id)
    {
        return;
    }
    runtime.detection_poll_timer = None;
    if expire_detection(runtime) {
        let _ = report(runtime);
        ui::invalidate("board");
        return;
    }
    let snapshot = poll_state(&runtime.config, &mut HostTransport);
    let _ = apply_detection_snapshot(runtime, snapshot);
    ui::invalidate("board");
}

fn poll_now() -> Result<(), String> {
    RUNTIME.with(|slot| {
        let mut slot = slot.borrow_mut();
        let runtime = slot
            .as_mut()
            .ok_or_else(|| "Autodarts is not configured.".to_string())?;
        let snapshot = poll(&runtime.config, &mut HostTransport);
        apply_detection_snapshot(runtime, snapshot)?;
        if matches!(
            runtime.snapshot.state,
            BoardState::Observed {
                running: Some(false),
                ..
            } | BoardState::Observed {
                connected: Some(false),
                ..
            }
        ) && (runtime.speech_in_flight.is_some()
            || runtime
                .announcement_session
                .pending_total_deadline()
                .is_some()
            || matches!(
                runtime.announcement_session.diagnostics().readiness,
                autodarts_core::AnnouncementReadiness::Collecting { .. }
                    | autodarts_core::AnnouncementReadiness::AwaitingRemoval
                    | autodarts_core::AnnouncementReadiness::AwaitingClear
            ))
        {
            invalidate_speech(runtime, "observed detection stop");
            reopen_speech(runtime);
        }
        Ok(())
    })
}
fn begin_detection(runtime: &mut Runtime, desired: bool) -> Result<(), String> {
    let token = DETECTION_GENERATION.with(|generation| {
        let next = generation.get().saturating_add(1);
        generation.set(next);
        next
    });
    runtime.detection = Some(DetectionOperation {
        token,
        desired,
        dispatch: DetectionDispatch::Uncertain("dispatching"),
        started_at: detection_now(),
        expired: false,
        fresh_definitive_observation: None,
    });
    let command = if desired {
        DetectionCommand::Start
    } else {
        DetectionCommand::Stop
    };
    match dispatch_detection(&runtime.config, &mut HostTransport, command) {
        Ok(dispatch) => {
            if let Some(operation) = runtime.detection.as_mut() {
                operation.dispatch = dispatch;
            }
            if !desired && dispatch == DetectionDispatch::Acknowledged {
                invalidate_speech(runtime, "detection stop acknowledged");
            }
            runtime.detection_readback = None;
            if let Some(operation) = runtime.detection.as_ref() {
                runtime.command_status = Some(pending_status(operation, &runtime.snapshot));
            }
            schedule_detection(runtime);
            report(runtime)?;
            ui::invalidate("board");
            Ok(())
        }
        Err(reason) => {
            cancel_detection(runtime);
            let action = if desired { "start" } else { "stop" };
            let ambiguous = reason == "command_connection";
            runtime.command_status = Some(if ambiguous {
                format!("{action} outcome unknown: {reason}")
            } else {
                format!("{action} rejected: {reason}")
            });
            runtime.detection_readback = Some(reason.into());
            report(runtime)?;
            ui::invalidate("board");
            if ambiguous {
                Err(format!("Detection command outcome unknown: {reason}"))
            } else {
                Err(format!("Detection command failed: {reason}"))
            }
        }
    }
}

fn ingest_detection_observation(json: &[u8]) {
    // Validate the bounded full envelope before promoting any device state.
    if json.len() > autodarts_core::local_score::MAX_ENVELOPE_BYTES {
        return;
    }
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(json) else {
        return;
    };
    if !matches!(
        autodarts_core::local_score::parse_envelope(&value),
        Ok(autodarts_core::local_score::LocalEvent::State(_))
    ) {
        return;
    }
    if value.get("type").and_then(serde_json::Value::as_str) != Some("state") {
        return;
    }
    let Some(data) = value.get("data") else {
        return;
    };
    let Ok(body) = serde_json::to_vec(data) else {
        return;
    };
    let Ok(state) = autodarts_core::decode_state(HttpResponse { status: 200, body }) else {
        return;
    };
    RUNTIME.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(runtime) = slot.as_mut() else {
            return;
        };
        let snapshot = Snapshot {
            availability: Availability::Available,
            version: None,
            state,
            diagnostic: None,
        };
        if let Err(error) = apply_detection_snapshot(runtime, snapshot) {
            log::log(log::Level::Warn, &error);
        }
        ui::invalidate("board");
    });
}
fn websocket_url(config: &Config) -> Result<(String, &'static str), String> {
    let mut url = config.board_manager_url.clone();
    let (grant, expected_port) = match url.scheme() {
        "http" => {
            url.set_scheme("ws")
                .map_err(|_| "Invalid WebSocket scheme.")?;
            let port = url.port_or_known_default();
            match port {
                Some(80) => ("board_http_default", 80),
                Some(3180) => ("board_http", 3180),
                _ => {
                    return Err(
                        "Board Manager WebSocket port is not covered by its host permission."
                            .into(),
                    );
                }
            }
        }
        "https" => {
            url.set_scheme("wss")
                .map_err(|_| "Invalid WebSocket scheme.")?;
            ("board_https", 443)
        }
        _ => return Err("Board Manager origin must use HTTP or HTTPS.".into()),
    };
    if url.port_or_known_default() != Some(expected_port) {
        return Err("Board Manager WebSocket port is not covered by its host permission.".into());
    }
    url.set_path("/api/events");
    url.set_query(None);
    url.set_fragment(None);
    Ok((url.to_string(), grant))
}

fn connect_websocket(config: &Config) -> Result<u32, String> {
    let (url, grant) = websocket_url(config)?;
    websocket::connect(grant, &url, &[])
        .map_err(|error| format!("Could not open Board Manager event stream: {error:?}"))
}

/// The bounded event ingress shared by host callbacks and synthetic tests.
fn ingest_announcement_json(json: &[u8]) -> autodarts_core::local_score::JourneyResult {
    RUNTIME.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(runtime) = slot.as_mut() else {
            return autodarts_core::local_score::JourneyResult::Suppressed;
        };
        let before = (
            runtime.announcement_session.diagnostics(),
            runtime.policy_event,
            runtime.speech_generation,
            runtime.speech_in_flight,
            runtime.speech_queue.len(),
        );
        if runtime.speech_reopen_pending {
            reopen_speech(runtime);
        }
        let mut overflowed = false;
        let mut delivery = SpeechDelivery {
            session: runtime.speech_session,
            generation: runtime.speech_generation,
            pending: &runtime.speech_pending,
            queue: &mut runtime.speech_queue,
            submission_error: &mut runtime.speech_submission_error,
            overflowed: &mut overflowed,
        };
        let result = runtime
            .announcement_session
            .ingest_json(json, &mut delivery);
        runtime.policy_event = runtime.announcement_session.diagnostics().last_event;
        if overflowed {
            invalidate_speech(runtime, "speech queue full");
            runtime.speech_reopen_pending = true;
        } else if result == autodarts_core::local_score::JourneyResult::Invalid {
            invalidate_speech(runtime, "invalid Board event");
            reopen_speech(runtime);
        } else {
            if result == autodarts_core::local_score::JourneyResult::Baseline {
                reopen_speech(runtime);
            }
            collect_speech(runtime);
            reconcile_deadline_timer(runtime);
        }
        let after = (
            runtime.announcement_session.diagnostics(),
            runtime.policy_event,
            runtime.speech_generation,
            runtime.speech_in_flight,
            runtime.speech_queue.len(),
        );
        if before != after {
            ui::invalidate("board");
        }
        result
    })
}

fn request_kind(key: &str) -> SpeechRequestKind {
    if key.contains(":dart:") {
        SpeechRequestKind::Dart
    } else if key.contains(":total:") {
        SpeechRequestKind::Total
    } else {
        SpeechRequestKind::Unknown
    }
}

fn remember_completion(runtime: &mut Runtime, completion: SpeechCompletion) {
    runtime.speech_history.push_back(completion);
    while runtime.speech_history.len() > MAX_SPEECH_HISTORY {
        runtime.speech_history.pop_front();
    }
}

struct SpeechDelivery<'a> {
    session: Option<u32>,
    generation: u64,
    pending: &'a BTreeMap<u64, SpeechRequest>,
    queue: &'a mut VecDeque<SpeechRequest>,
    submission_error: &'a mut Option<String>,
    overflowed: &'a mut bool,
}
impl autodarts_core::AnnouncementDelivery for SpeechDelivery<'_> {
    fn deliver(
        &mut self,
        request: &autodarts_core::AnnouncementRequest,
    ) -> Result<(), autodarts_core::DeliveryError> {
        if self.session.is_none() {
            *self.submission_error = Some("Speech stream unavailable".into());
            return Err(autodarts_core::DeliveryError::Unavailable);
        }
        if self.pending.len() + self.queue.len() >= MAX_PENDING_SPEECH {
            *self.overflowed = true;
            *self.submission_error = Some("Speech request queue full; submission rejected".into());
            return Err(autodarts_core::DeliveryError::Failed(
                "speech request queue full".into(),
            ));
        }
        self.queue.push_back(SpeechRequest {
            generation: self.generation,
            logical_key: request.deduplication_key.clone(),
            kind: request_kind(&request.deduplication_key),
            text: request.text.clone(),
        });
        *self.submission_error = None;
        Ok(())
    }
}

fn map_speech_outcome(
    outcome: &bindings::niles::integration::speech_stream::Outcome,
) -> SpeechOutcome {
    use bindings::niles::integration::speech_stream::Outcome;
    match outcome {
        Outcome::Pending => SpeechOutcome::Pending,
        Outcome::Delivered => SpeechOutcome::Delivered,
        Outcome::Busy => SpeechOutcome::Busy,
        Outcome::Quiet => SpeechOutcome::Quiet,
        Outcome::Expired => SpeechOutcome::Expired,
        Outcome::Failed => SpeechOutcome::Failed,
        Outcome::Cancelled => SpeechOutcome::Cancelled,
        Outcome::Replaced => SpeechOutcome::Replaced,
        Outcome::RateLimited => SpeechOutcome::RateLimited,
        Outcome::NoSatellite => SpeechOutcome::NoSatellite,
    }
}

fn cancel_speech_progress(runtime: &mut Runtime) {
    if let Some(id) = runtime.speech_progress_timer.take() {
        timers::cancel(&id);
    }
}

fn schedule_speech_progress(runtime: &mut Runtime) {
    let Some(request) = runtime.speech_in_flight else {
        cancel_speech_progress(runtime);
        return;
    };
    let id = format!("speech-progress:g{}:r{request}", runtime.speech_generation);
    if runtime.speech_progress_timer.as_deref() == Some(id.as_str()) {
        return;
    }
    if let Some(previous) = runtime.speech_progress_timer.take() {
        timers::cancel(&previous);
    }
    match timers::after(&id, 1) {
        Ok(()) => runtime.speech_progress_timer = Some(id),
        Err(error) => {
            invalidate_speech(runtime, "speech progress timer failure");
            runtime.speech_submission_error = Some(format!(
                "Speech progress timer failed: {error:?}; speech requests discarded"
            ));
            runtime.speech_reopen_pending = true;
        }
    }
}

fn pump_speech(runtime: &mut Runtime) {
    if runtime.speech_in_flight.is_some() {
        return;
    }
    let Some(session) = runtime.speech_session else {
        return;
    };
    loop {
        let Some(request) = runtime.speech_queue.pop_front() else {
            cancel_speech_progress(runtime);
            return;
        };
        match speech_stream::submit(session, &request.text) {
            Ok(id) => {
                runtime.speech_pending.insert(id, request);
                runtime.speech_in_flight = Some(id);
                runtime.speech_submission_error = None;
                schedule_speech_progress(runtime);
                return;
            }
            Err(error) => {
                runtime.speech_submission_error = Some(format!("Speech submit failed: {error:?}"));
                remember_completion(
                    runtime,
                    SpeechCompletion {
                        generation: request.generation,
                        host_request: 0,
                        logical_key: request.logical_key,
                        kind: request.kind,
                        text: request.text,
                        outcome: SpeechOutcome::Failed,
                    },
                );
            }
        }
    }
}

fn poll_speech() {
    RUNTIME.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(runtime) = slot.as_mut() else { return };
        collect_speech(runtime);
    });
}

fn collect_speech(runtime: &mut Runtime) {
    if runtime.speech_reopen_pending {
        reopen_speech(runtime);
    }
    let Some(session) = runtime.speech_session else {
        return;
    };
    if runtime.speech_in_flight.is_none() {
        pump_speech(runtime);
        return;
    }
    let current = runtime.speech_in_flight;
    match speech_stream::poll(session) {
        Ok(statuses) => {
            runtime.speech_poll_error = None;
            for item in statuses {
                let outcome = map_speech_outcome(&item.outcome);
                if outcome == SpeechOutcome::Pending || Some(item.request) != current {
                    continue;
                }
                if let Some(request) = runtime.speech_pending.remove(&item.request) {
                    runtime.speech_in_flight = None;
                    remember_completion(
                        runtime,
                        SpeechCompletion {
                            generation: request.generation,
                            host_request: item.request,
                            logical_key: request.logical_key,
                            kind: request.kind,
                            text: request.text,
                            outcome,
                        },
                    );
                }
            }
            pump_speech(runtime);
            schedule_speech_progress(runtime);
        }
        Err(error) => {
            runtime.speech_poll_error = Some(format!("Speech poll failed: {error:?}"));
            schedule_speech_progress(runtime);
        }
    }
}
fn cancel_deadline_timer(runtime: &mut Runtime) {
    if let Some((_, id)) = runtime.deadline_timer.take() {
        timers::cancel(&id);
    }
}
fn reconcile_deadline_timer(runtime: &mut Runtime) {
    let desired = runtime.announcement_session.pending_total_deadline();
    if runtime
        .deadline_timer
        .as_ref()
        .is_some_and(|(token, _)| Some(*token) != desired)
    {
        cancel_deadline_timer(runtime);
    }
    let Some(token) = desired else {
        return;
    };
    if runtime
        .deadline_timer
        .as_ref()
        .is_some_and(|(registered, _)| *registered == token)
    {
        return;
    }
    let id = format!(
        "total-deadline:g{}:e{}:c{}",
        runtime.speech_generation, token.epoch, token.collection
    );
    match timers::after(&id, 1) {
        Ok(()) => runtime.deadline_timer = Some((token, id)),
        Err(error) => {
            let message =
                format!("Total deadline timer failed: {error:?}; speech requests discarded");
            invalidate_speech(runtime, "total deadline timer failure");
            runtime.speech_submission_error = Some(message);
            runtime.speech_reopen_pending = true;
        }
    }
}

fn on_deadline_timer(runtime: &mut Runtime, id: &str) {
    let Some((token, registered)) = runtime.deadline_timer.take() else {
        return;
    };
    if registered != id {
        runtime.deadline_timer = Some((token, registered));
        return;
    }
    let mut overflowed = false;
    let mut delivery = SpeechDelivery {
        session: runtime.speech_session,
        generation: runtime.speech_generation,
        pending: &runtime.speech_pending,
        queue: &mut runtime.speech_queue,
        submission_error: &mut runtime.speech_submission_error,
        overflowed: &mut overflowed,
    };
    runtime
        .announcement_session
        .on_total_deadline(token, &mut delivery);
    runtime.policy_event = runtime.announcement_session.diagnostics().last_event;
    if overflowed {
        invalidate_speech(runtime, "speech queue full");
        runtime.speech_reopen_pending = true;
    } else {
        collect_speech(runtime);
        reconcile_deadline_timer(runtime);
    }
    ui::invalidate("board");
}
fn speech_status_line(runtime: &Runtime) -> String {
    let mut parts = vec![if !runtime.speech_enabled {
        "Speech stream disabled".to_string()
    } else if runtime.speech_session.is_some() {
        "Speech stream open".to_string()
    } else {
        "Speech stream unavailable".to_string()
    }];
    if let Some(error) = &runtime.speech_submission_error {
        parts.push(error.clone());
    }
    if let Some(error) = &runtime.speech_poll_error {
        parts.push(error.clone());
    }
    if !runtime.speech_pending.is_empty() {
        parts.push(format!("Speech pending: {}", runtime.speech_pending.len()));
    }
    if !runtime.speech_queue.is_empty() {
        parts.push(format!("Speech queued: {}", runtime.speech_queue.len()));
    }
    if !runtime.speech_history.is_empty() {
        let recent = runtime
            .speech_history
            .iter()
            .map(|record| {
                format!(
                    "{} gen {} {:?} {} ({}) : {:?}",
                    record.host_request,
                    record.generation,
                    record.kind,
                    record.text,
                    record.logical_key,
                    record.outcome
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        parts.push(format!("Speech terminal history: {recent}"));
    }
    parts.join("; ")
}
fn readiness_text(readiness: autodarts_core::AnnouncementReadiness) -> String {
    match readiness {
        autodarts_core::AnnouncementReadiness::Disabled => "disabled".into(),
        autodarts_core::AnnouncementReadiness::AwaitingBaseline => "awaiting baseline".into(),
        autodarts_core::AnnouncementReadiness::Collecting { dart_count } => {
            format!("collecting ({dart_count} darts)")
        }
        autodarts_core::AnnouncementReadiness::AwaitingRemoval => "awaiting removal".into(),
        autodarts_core::AnnouncementReadiness::AwaitingClear => "awaiting clear".into(),
    }
}

fn policy_event_text(event: Option<autodarts_core::AnnouncementEvent>) -> String {
    let Some(event) = event else {
        return "none".into();
    };
    format!("{:?} ({:?})", event.disposition, event.reason)
}
struct HostTransport;
impl Transport for HostTransport {
    fn get(&mut self, url: &url::Url, timeout_ms: u32) -> Result<HttpResponse, HttpFailure> {
        let response = http::send(&Request {
            method: Method::Get,
            url: url.to_string(),
            headers: Vec::new(),
            body: None,
            timeout_ms: Some(timeout_ms),
        })
        .map_err(map_http)?;
        if response.body.len() > autodarts_core::MAX_RESPONSE_BYTES {
            return Err(HttpFailure::TooLarge);
        }
        Ok(HttpResponse {
            status: response.status,
            body: response.body,
        })
    }
}
impl CommandTransport for HostTransport {
    fn put(&mut self, url: &url::Url, timeout_ms: u32) -> Result<u16, HttpFailure> {
        let response = http::send(&Request {
            method: Method::Put,
            url: url.to_string(),
            headers: Vec::new(),
            body: None,
            timeout_ms: Some(timeout_ms),
        })
        .map_err(map_http)?;
        // The complete response status is authoritative; the unused body cannot
        // turn a definite HTTP rejection into submission uncertainty.
        Ok(response.status)
    }
}
fn map_http(error: http::HttpError) -> HttpFailure {
    match error {
        http::HttpError::Timeout => HttpFailure::Timeout,
        http::HttpError::TooLarge => HttpFailure::TooLarge,
        _ => HttpFailure::Connection,
    }
}
fn append_text_nodes(nodes: &mut Vec<Node>, lines: impl IntoIterator<Item = String>) {
    for line in lines {
        let index = nodes.len() as u32;
        nodes[0].children.push(index);
        nodes.push(Node {
            kind: NodeKind::Text(TextBlock {
                value: line,
                muted: false,
            }),
            children: Vec::new(),
        });
    }
}

fn render_tree(
    snapshot: &Snapshot,
    command_status: Option<&str>,
    detection_readback: Option<&str>,
) -> UiTree {
    let mut nodes = vec![Node {
        kind: NodeKind::Stack,
        children: Vec::new(),
    }];
    append_text_nodes(
        &mut nodes,
        diagnostics_text(snapshot, command_status, detection_readback),
    );
    UiTree { nodes }
}
fn invalidate_speech(runtime: &mut Runtime, reason: &str) {
    cancel_deadline_timer(runtime);
    cancel_speech_progress(runtime);
    runtime.announcement_session.connection_reset();
    runtime.speech_generation = SPEECH_GENERATION.with(|generation| {
        let next = generation.get().saturating_add(1);
        generation.set(next);
        next
    });
    let pending = std::mem::take(&mut runtime.speech_pending);
    for (host_request, request) in pending {
        remember_completion(
            runtime,
            SpeechCompletion {
                generation: request.generation,
                host_request,
                logical_key: request.logical_key,
                kind: request.kind,
                text: request.text,
                outcome: SpeechOutcome::DiscardedOnReset,
            },
        );
    }
    let queued = std::mem::take(&mut runtime.speech_queue);
    for request in queued {
        remember_completion(
            runtime,
            SpeechCompletion {
                generation: request.generation,
                host_request: 0,
                logical_key: request.logical_key,
                kind: request.kind,
                text: request.text,
                outcome: SpeechOutcome::DiscardedOnReset,
            },
        );
    }
    runtime.speech_in_flight = None;
    let session = runtime.speech_session.take();
    runtime.speech_reopen_pending = false;
    runtime.speech_poll_error = None;
    runtime.speech_submission_error = Some(format!("Speech requests discarded on {reason}"));
    if let Some(session) = session {
        speech_stream::close(session);
    }
}

fn reopen_speech(runtime: &mut Runtime) {
    if !(runtime.announcements.enabled
        && runtime.speech_enabled
        && runtime.speech_session.is_none())
    {
        return;
    }
    runtime.speech_reopen_pending = false;
    match speech_stream::open(&runtime.config.room) {
        Ok(session) => {
            runtime.speech_session = Some(session);
            runtime.speech_submission_error = None;
        }
        Err(error) => {
            // close is asynchronous in the released host; Busy maps to
            // Unavailable until its old owner finishes. Retry opening on
            // later ingress/timer callbacks, never replay an utterance.
            runtime.speech_reopen_pending = matches!(error, types::HostError::Unavailable(_));
            runtime.speech_submission_error =
                Some(format!("Speech stream reopen failed: {error:?}"));
        }
    }
}

fn stop_runtime() -> Option<Runtime> {
    let mut runtime = RUNTIME.with(|slot| slot.borrow_mut().take())?;
    cancel_detection(&mut runtime);
    let conn = runtime.websocket_id.take();
    invalidate_speech(&mut runtime, "stop");
    if let Some(conn) = conn {
        websocket::close(conn);
    }
    Some(runtime)
}
impl Guest for Plugin {
    fn init() -> Result<(), String> {
        let config = match settings() {
            Ok(config) => config,
            Err(error) => {
                Self::stop();
                return Err(error);
            }
        };
        let announcements = match announcement_settings(&config.room) {
            Ok(announcements) => announcements,
            Err(error) => {
                Self::stop();
                return Err(error);
            }
        };
        let speech_enabled = match speech_enabled() {
            Ok(enabled) => enabled,
            Err(error) => {
                Self::stop();
                return Err(error);
            }
        };
        let unchanged = RUNTIME.with(|slot| {
            slot.borrow().as_ref().is_some_and(|runtime| {
                runtime.config == config
                    && runtime.announcements == announcements
                    && runtime.speech_enabled == speech_enabled
            })
        });
        if unchanged {
            return Ok(());
        }
        let previous = stop_runtime();
        let announcements_enabled = announcements.enabled;
        let (websocket_id, websocket_status) = if announcements_enabled {
            match connect_websocket(&config) {
                Ok(connection) => (
                    Some(connection),
                    Some("Board event stream connecting".into()),
                ),
                Err(error) => {
                    log::log(log::Level::Warn, &error);
                    (
                        None,
                        Some(format!("Board event stream unavailable: {error}")),
                    )
                }
            }
        } else {
            (None, Some("Board event stream disabled".into()))
        };
        let mut speech_open_error = None;
        let speech_session = if announcements_enabled && speech_enabled {
            match speech_stream::open(&config.room) {
                Ok(session) => Some(session),
                Err(error) => {
                    let message = format!("Speech stream open failed: {error:?}");
                    log::log(log::Level::Warn, &message);
                    speech_open_error = Some(message);
                    None
                }
            }
        } else {
            None
        };
        RUNTIME.with(|slot| {
            *slot.borrow_mut() = Some(Runtime {
                announcement_session: autodarts_core::AnnouncementSession::new(
                    announcements.clone(),
                ),
                policy_event: None,
                announcements,
                config,
                snapshot: Snapshot {
                    availability: Availability::Unknown,
                    version: None,
                    state: BoardState::Unknown,
                    diagnostic: None,
                },
                observation_at: None,
                command_status: None,
                detection: None,
                detection_poll_timer: None,
                detection_expiry_timer: None,
                detection_readback: None,
                speech_session,
                speech_reopen_pending: false,
                speech_generation: SPEECH_GENERATION.with(std::cell::Cell::get),
                speech_pending: BTreeMap::new(),
                speech_in_flight: None,
                speech_queue: VecDeque::new(),
                speech_history: previous
                    .map_or_else(VecDeque::new, |runtime| runtime.speech_history),
                speech_poll_error: None,
                speech_submission_error: speech_open_error,
                speech_enabled,
                deadline_timer: None,
                speech_progress_timer: None,
                websocket_id,
                websocket_status,
            })
        });
        if let Err(error) = poll_now() {
            log::log(log::Level::Warn, &error);
        }
        timers::every("poll", settings_interval())
            .map_err(|_| "Could not schedule the Autodarts poll timer.".to_string())
    }
    fn tools() -> Vec<ToolDef> {
        Vec::new()
    }
    fn call_tool(_: String, _: String) -> Result<String, String> {
        Err("Autodarts has no tools in this foundation.".into())
    }
    fn set_device(
        id: String,
        desired: bindings::niles::integration::types::DeviceState,
    ) -> Result<(), String> {
        RUNTIME.with(|slot| {
            let mut slot = slot.borrow_mut();
            let runtime = slot
                .as_mut()
                .ok_or_else(|| "Autodarts is not configured.".to_string())?;
            if id != device_id(runtime) {
                return Err("Unknown Autodarts device.".into());
            }
            let on = desired
                .on
                .ok_or_else(|| "Detection control requires an on/off value.".to_string())?;
            if desired.brightness.is_some()
                || desired.color_temp_kelvin.is_some()
                || desired.rgb.is_some()
                || desired.temperature_celsius.is_some()
                || desired.humidity_percent.is_some()
                || desired.battery_percent.is_some()
                || desired.open.is_some()
                || desired.motion.is_some()
                || desired.illuminance_lux.is_some()
                || desired.position.is_some()
                || !desired.readings.is_empty()
            {
                return Err("Only detection on/off is supported.".into());
            }

            expire_detection(runtime);
            let mut superseded = false;
            if let Some(operation) = runtime.detection.clone() {
                let unresolved_duplicate = operation.desired == on
                    // Only reconciliation can complete an operation, never cached state.
                    && !(operation.expired
                        && operation.fresh_definitive_observation.is_some()
                        && runtime.observation_at.is_some_and(|at|
                            detection_now().saturating_duration_since(at)
                                <= Duration::from_secs(u64::from(runtime.config.poll_interval_secs))));
                if unresolved_duplicate {
                    runtime.command_status = Some(pending_status(&operation, &runtime.snapshot));
                    report(runtime)?;
                    ui::invalidate("board");
                    return Ok(());
                }
                superseded = operation.desired != on;
                cancel_detection(runtime);
            }
            if !superseded
                && runtime.observation_at.is_some_and(|at|
                    detection_now().saturating_duration_since(at)
                        <= Duration::from_secs(u64::from(runtime.config.poll_interval_secs)))
                && confirms(&runtime.snapshot, on)
            {
                runtime.command_status = Some(if on {
                    "running already confirmed".into()
                } else {
                    "stopped already confirmed".into()
                });
                runtime.detection_readback = None;
                report(runtime)?;
                ui::invalidate("board");
                return Ok(());
            }
            begin_detection(runtime, on)
        })
    }
    fn on_timer(id: String) {
        if id == "poll" {
            if let Err(error) = poll_now() {
                log::log(log::Level::Warn, &error);
            }
            poll_speech();
            ui::invalidate("board");
        } else if id.starts_with("detection-") {
            RUNTIME.with(|slot| {
                if let Some(runtime) = slot.borrow_mut().as_mut() {
                    on_detection_timer(runtime, &id);
                }
            });
        } else if id.starts_with("total-deadline:") {
            RUNTIME.with(|slot| {
                if let Some(runtime) = slot.borrow_mut().as_mut() {
                    on_deadline_timer(runtime, &id);
                }
            });
        } else if id.starts_with("speech-progress:") {
            let current = RUNTIME.with(|slot| {
                slot.borrow_mut().as_mut().is_some_and(|runtime| {
                    if runtime.speech_progress_timer.as_deref() != Some(id.as_str()) {
                        return false;
                    }
                    runtime.speech_progress_timer = None;
                    true
                })
            });
            if current {
                poll_speech();
                ui::invalidate("board");
            }
        }
    }
    fn on_event(_: Event) {}
    fn render(
        surface: String,
        _: bindings::niles::integration::ui::RenderContext,
    ) -> Result<bindings::niles::integration::ui::UiTree, String> {
        if surface != "board" {
            return Err("Unknown Autodarts surface.".into());
        }
        let (
            snapshot,
            status,
            detection_readback,
            announcements,
            diagnostics,
            policy_event,
            speech_status,
            speech_enabled,
            websocket_status,
        ) = RUNTIME.with(|slot| {
            slot.borrow()
                .as_ref()
                .map(|runtime| {
                    (
                        runtime.snapshot.clone(),
                        runtime.command_status.clone(),
                        runtime.detection_readback.clone(),
                        runtime.announcements.clone(),
                        runtime.announcement_session.diagnostics(),
                        runtime.policy_event,
                        speech_status_line(runtime),
                        runtime.speech_enabled,
                        runtime.websocket_status.clone(),
                    )
                })
                .ok_or_else(|| "Autodarts is not configured.".to_string())
        })?;
        let mut tree = render_tree(&snapshot, status.as_deref(), detection_readback.as_deref());
        let lines = vec![
            format!(
                "Announcement intent: {}",
                if announcements.enabled {
                    "enabled"
                } else {
                    "disabled"
                }
            ),
            format!(
                "Announcement mode: {}",
                match announcements.mode {
                    Some(autodarts_core::AnnouncementMode::EachDartThenTotal) =>
                        "Each dart → Total",
                    Some(autodarts_core::AnnouncementMode::TotalOnly) => "Total only",
                    None => "not selected",
                }
            ),
            format!("Announcement room: {}", announcements.room),
            format!(
                "Policy readiness: {}",
                readiness_text(diagnostics.readiness)
            ),
            format!(
                "Last policy event: {}",
                policy_event_text(policy_event.or(diagnostics.last_event))
            ),
            format!(
                "Speech stream setting: {}",
                if speech_enabled {
                    "enabled"
                } else {
                    "disabled"
                }
            ),
            speech_status,
            websocket_status.unwrap_or_else(|| "Board event stream status unavailable".into()),
        ];
        append_text_nodes(&mut tree.nodes, lines);
        Ok(tree)
    }
    fn on_action(
        _: String,
        _: String,
        _: ActionValue,
        _: bindings::niles::integration::ui::RenderContext,
    ) -> Result<ActionResult, String> {
        Err("Unsupported Autodarts action.".into())
    }
    fn on_mqtt_message(_: u32, _: String, _: Vec<u8>) {}
    fn on_mqtt_status(_: u32, _: bindings::niles::integration::mqtt::ConnectionStatus) {}
    fn on_websocket_message(conn: u32, message: bindings::niles::integration::websocket::Message) {
        let current = RUNTIME.with(|slot| {
            slot.borrow()
                .as_ref()
                .is_some_and(|runtime| runtime.websocket_id == Some(conn))
        });
        if !current {
            return;
        }
        match message {
            bindings::niles::integration::websocket::Message::Text(text) => {
                ingest_detection_observation(text.as_bytes());
                // The core parser checks byte length before parsing, without
                // allocating a second oversized buffer.
                let _ = ingest_announcement_json(text.as_bytes());
            }
            bindings::niles::integration::websocket::Message::Binary(_) => {
                let _ = ingest_announcement_json(b"invalid non-text WebSocket event");
            }
        }
    }
    fn on_websocket_status(
        conn: u32,
        status: bindings::niles::integration::websocket::ConnectionStatus,
    ) {
        RUNTIME.with(|slot| {
            let mut slot = slot.borrow_mut();
            let Some(runtime) = slot.as_mut() else { return };
            if runtime.websocket_id != Some(conn) {
                return;
            }
            runtime.observation_at = None;
            if let Some(operation) = runtime.detection.as_mut() {
                operation.fresh_definitive_observation = None;
            }
            match status {
                bindings::niles::integration::websocket::ConnectionStatus::Connected => {
                    if runtime.speech_in_flight.is_some()
                        || !runtime.speech_queue.is_empty()
                        || runtime.deadline_timer.is_some()
                    {
                        invalidate_speech(runtime, "Board event reconnect");
                    } else {
                        runtime.announcement_session.connection_reset();
                    }
                    runtime.policy_event = None;
                    reopen_speech(runtime);
                    runtime.websocket_status = Some("Board event stream connected".into());
                }
                bindings::niles::integration::websocket::ConnectionStatus::Disconnected(reason) => {
                    invalidate_speech(runtime, "Board event disconnect");
                    runtime.policy_event = None;
                    runtime.websocket_status =
                        Some(format!("Board event stream reconnecting: {reason}"));
                }
                bindings::niles::integration::websocket::ConnectionStatus::Failed(kind) => {
                    invalidate_speech(runtime, "Board event failure");
                    runtime.policy_event = None;
                    let reason = match kind {
                        bindings::niles::integration::websocket::FailureKind::Policy => {
                            "policy denied"
                        }
                        bindings::niles::integration::websocket::FailureKind::Authentication => {
                            "authentication rejected"
                        }
                        bindings::niles::integration::websocket::FailureKind::Tls => "TLS failed",
                        bindings::niles::integration::websocket::FailureKind::Protocol => {
                            "protocol failed"
                        }
                    };
                    runtime.websocket_status = Some(format!("Board event stream failed: {reason}"));
                }
                bindings::niles::integration::websocket::ConnectionStatus::Overflow(count) => {
                    invalidate_speech(runtime, "Board event overflow");
                    runtime.policy_event = None;
                    reopen_speech(runtime);
                    runtime.websocket_status =
                        Some(format!("Board event stream overflow ({count})"));
                }
            }
            ui::invalidate("board");
        });
    }
    fn authorize_url(_: String, _: String) -> Result<String, String> {
        Err("Autodarts does not use account authorization.".into())
    }
    fn on_callback(
        _: String,
        _: Vec<(String, String)>,
        _: Option<String>,
    ) -> Result<String, String> {
        Err("Autodarts does not use account callbacks.".into())
    }
    fn on_webhook(
        _: bindings::niles::integration::inbound::InboundRequest,
    ) -> bindings::niles::integration::inbound::InboundResponse {
        bindings::niles::integration::inbound::InboundResponse {
            status: 404,
            content_type: None,
            body: Vec::new(),
        }
    }
    fn stop() {
        stop_runtime();
    }
}
fn settings_interval() -> u32 {
    RUNTIME.with(|slot| {
        slot.borrow()
            .as_ref()
            .map_or(15, |runtime| runtime.config.poll_interval_secs)
    })
}
