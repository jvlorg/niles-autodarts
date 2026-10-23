use autodarts_core::{
    Availability, BoardState, Config, HttpFailure, HttpResponse, Snapshot, Transport,
    detection::{CommandTransport, DetectionCommand, DetectionResult, set_detection},
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
struct Runtime {
    announcements: autodarts_core::AnnouncementSettings,
    announcement_session: autodarts_core::AnnouncementSession,
    config: Config,
    snapshot: Snapshot,
    command_status: Option<String>,
    speech_session: Option<u32>,
    speech_reopen_pending: bool,
    speech_generation: u64,
    speech_pending: BTreeMap<u64, SpeechRequest>,
    speech_history: VecDeque<SpeechCompletion>,
    speech_poll_error: Option<String>,
    speech_submission_error: Option<String>,
    speech_enabled: bool,
    websocket_id: Option<u32>,
    websocket_status: Option<String>,
}

const MAX_PENDING_SPEECH: usize = 128;
const MAX_SPEECH_HISTORY: usize = 32;

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
fn diagnostics_text(snapshot: &Snapshot, command_status: Option<&str>) -> Vec<String> {
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
fn poll_now() -> Result<(), String> {
    RUNTIME.with(|slot| {
        let mut slot = slot.borrow_mut();
        let runtime = slot
            .as_mut()
            .ok_or_else(|| "Autodarts is not configured.".to_string())?;
        runtime.snapshot = poll(&runtime.config, &mut HostTransport);
        report(runtime)
    })
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
    let has_pending = RUNTIME.with(|slot| {
        slot.borrow().as_ref().is_some_and(|runtime| {
            runtime.speech_reopen_pending || !runtime.speech_pending.is_empty()
        })
    });
    // The released host retains only 32 uncollected results. Reclaim them
    // during event ingress, not only at our larger bookkeeping bound.
    if has_pending {
        poll_speech();
    }
    RUNTIME.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(runtime) = slot.as_mut() else {
            return autodarts_core::local_score::JourneyResult::Suppressed;
        };
        let mut delivery = SpeechDelivery {
            session: runtime.speech_session,
            generation: runtime.speech_generation,
            pending: &mut runtime.speech_pending,
            submission_error: &mut runtime.speech_submission_error,
        };
        let result = runtime
            .announcement_session
            .ingest_json(json, &mut delivery);
        if result == autodarts_core::local_score::JourneyResult::Invalid {
            invalidate_speech(runtime, "invalid Board event");
            reopen_speech(runtime);
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
    pending: &'a mut BTreeMap<u64, SpeechRequest>,
    submission_error: &'a mut Option<String>,
}
impl autodarts_core::AnnouncementDelivery for SpeechDelivery<'_> {
    fn deliver(
        &mut self,
        request: &autodarts_core::AnnouncementRequest,
    ) -> Result<(), autodarts_core::DeliveryError> {
        let Some(session) = self.session else {
            *self.submission_error = Some("Speech stream unavailable".into());
            return Err(autodarts_core::DeliveryError::Unavailable);
        };
        if self.pending.len() >= MAX_PENDING_SPEECH {
            *self.submission_error =
                Some("Speech request tracking full; submission rejected".into());
            return Err(autodarts_core::DeliveryError::Failed(
                "speech request tracking full".into(),
            ));
        }
        match speech_stream::submit(session, &request.text) {
            Ok(id) => {
                self.pending.insert(
                    id,
                    SpeechRequest {
                        generation: self.generation,
                        logical_key: request.deduplication_key.clone(),
                        kind: request_kind(&request.deduplication_key),
                        text: request.text.clone(),
                    },
                );
                *self.submission_error = None;
                Ok(())
            }
            Err(error) => {
                let message = format!("Speech submit failed: {error:?}");
                *self.submission_error = Some(message.clone());
                Err(autodarts_core::DeliveryError::Failed(message))
            }
        }
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

fn poll_speech() {
    RUNTIME.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(runtime) = slot.as_mut() else { return };
        if runtime.speech_reopen_pending {
            reopen_speech(runtime);
        }
        let Some(session) = runtime.speech_session else {
            return;
        };
        match speech_stream::poll(session) {
            Ok(statuses) => {
                runtime.speech_poll_error = None;
                for item in statuses {
                    let outcome = map_speech_outcome(&item.outcome);
                    if outcome == SpeechOutcome::Pending {
                        continue;
                    }
                    if let Some(request) = runtime.speech_pending.remove(&item.request) {
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
            }
            Err(error) => {
                runtime.speech_poll_error = Some(format!("Speech poll failed: {error:?}"));
            }
        }
    });
}
fn speech_status_line(runtime: &Runtime) -> String {
    let mut parts = Vec::new();
    if let Some(error) = &runtime.speech_submission_error {
        parts.push(error.clone());
    }
    if let Some(error) = &runtime.speech_poll_error {
        parts.push(error.clone());
    }
    if !runtime.speech_pending.is_empty() {
        parts.push(format!("Speech pending: {}", runtime.speech_pending.len()));
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
    if !parts.is_empty() {
        return parts.join("; ");
    }
    if runtime.speech_enabled {
        if runtime.speech_session.is_some() {
            "Speech stream open".into()
        } else {
            "Speech stream unavailable".into()
        }
    } else {
        "Speech stream disabled".into()
    }
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
        if response.body.len() > autodarts_core::MAX_RESPONSE_BYTES {
            return Err(HttpFailure::TooLarge);
        }
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

fn render_tree(snapshot: &Snapshot, command_status: Option<&str>) -> UiTree {
    let mut nodes = vec![Node {
        kind: NodeKind::Stack,
        children: Vec::new(),
    }];
    append_text_nodes(&mut nodes, diagnostics_text(snapshot, command_status));
    UiTree { nodes }
}
fn invalidate_speech(runtime: &mut Runtime, reason: &str) {
    runtime.announcement_session.connection_reset();
    runtime.speech_generation = runtime.speech_generation.saturating_add(1);
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
    let session = runtime.speech_session.take();
    runtime.speech_reopen_pending = false;
    runtime.speech_poll_error = None;
    runtime.speech_submission_error = Some(format!("Speech requests discarded on {reason}"));
    if let Some(session) = session {
        speech_stream::close(session);
    }
}

fn reopen_speech(runtime: &mut Runtime) {
    runtime.speech_reopen_pending = false;
    if runtime.announcements.enabled && runtime.speech_enabled && runtime.speech_session.is_none() {
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
}

fn stop_runtime() -> Option<Runtime> {
    let mut runtime = RUNTIME.with(|slot| slot.borrow_mut().take())?;
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
                announcements,
                config,
                snapshot: Snapshot {
                    availability: Availability::Unknown,
                    version: None,
                    state: BoardState::Unknown,
                    diagnostic: None,
                },
                command_status: None,
                speech_session,
                speech_reopen_pending: false,
                speech_generation: previous
                    .as_ref()
                    .map_or(0, |runtime| runtime.speech_generation),
                speech_pending: BTreeMap::new(),
                speech_history: previous
                    .map_or_else(VecDeque::new, |runtime| runtime.speech_history),
                speech_poll_error: None,
                speech_submission_error: speech_open_error,
                speech_enabled,
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
            let command = if on {
                DetectionCommand::Start
            } else {
                DetectionCommand::Stop
            };
            let (outcome, observation) =
                set_detection(&runtime.config, &mut HostTransport, command);
            let command_status = match &outcome {
                DetectionResult::Confirmed(true) => "running confirmed".into(),
                DetectionResult::Confirmed(false) => "stopped confirmed".into(),
                DetectionResult::Pending {
                    desired: true,
                    observed: Some(false),
                } => "start acknowledged; still stopped".into(),
                DetectionResult::Pending {
                    desired: false,
                    observed: Some(true),
                } => "stop acknowledged; still running".into(),
                DetectionResult::Pending {
                    desired: true,
                    observed: None,
                } => "start acknowledged; observed state unknown".into(),
                DetectionResult::Pending {
                    desired: false,
                    observed: None,
                } => "stop acknowledged; observed state unknown".into(),
                DetectionResult::Pending {
                    desired: true,
                    observed: Some(true),
                } => "start acknowledged; observed running".into(),
                DetectionResult::Pending {
                    desired: false,
                    observed: Some(false),
                } => "stop acknowledged; observed stopped".into(),
                DetectionResult::Failed(reason) => {
                    return Err(format!("Detection command failed: {reason}"));
                }
            };
            runtime.snapshot = observation;
            runtime.command_status = Some(command_status);
            report(runtime)?;
            ui::invalidate("board");
            if !matches!(outcome, DetectionResult::Confirmed(_)) {
                return Err(runtime.command_status.clone().unwrap_or_default());
            }
            Ok(())
        })
    }
    fn on_timer(id: String) {
        if id == "poll" {
            poll_speech();
            if let Err(error) = poll_now() {
                log::log(log::Level::Warn, &error);
            }
            ui::invalidate("board");
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
        let (snapshot, status, announcements, speech_status, speech_enabled, websocket_status) =
            RUNTIME.with(|slot| {
                slot.borrow()
                    .as_ref()
                    .map(|runtime| {
                        (
                            runtime.snapshot.clone(),
                            runtime.command_status.clone(),
                            runtime.announcements.clone(),
                            speech_status_line(runtime),
                            runtime.speech_enabled,
                            runtime.websocket_status.clone(),
                        )
                    })
                    .ok_or_else(|| "Autodarts is not configured.".to_string())
            })?;
        let mut tree = render_tree(&snapshot, status.as_deref());
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
            match status {
                bindings::niles::integration::websocket::ConnectionStatus::Connected => {
                    runtime.announcement_session.connection_reset();
                    reopen_speech(runtime);
                    runtime.websocket_status =
                        Some("Board event stream connected; baseline required".into());
                }
                bindings::niles::integration::websocket::ConnectionStatus::Disconnected(reason) => {
                    invalidate_speech(runtime, "Board event disconnect");
                    runtime.websocket_status =
                        Some(format!("Board event stream reconnecting: {reason}"));
                }
                bindings::niles::integration::websocket::ConnectionStatus::Failed(kind) => {
                    invalidate_speech(runtime, "Board event failure");
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
                    reopen_speech(runtime);
                    runtime.websocket_status = Some(format!(
                        "Board event stream overflow ({count}); baseline required"
                    ));
                }
            }
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
            .map(|runtime| runtime.config.poll_interval_secs)
            .unwrap_or(15)
    })
}
