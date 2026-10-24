use serde::{Deserialize, Serialize};
use url::Url;

pub mod announcement;
pub use announcement_session::AnnouncementSession;
pub mod announcement_session;
pub mod detection;
pub mod local_score;
pub use local_score::{JourneyDecision, JourneyReadiness};

pub const MAX_RESPONSE_BYTES: usize = 256 * 1024;
pub const DEFAULT_TIMEOUT_MS: u32 = 2_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub board_manager_url: Url,
    pub room: String,
    pub poll_interval_secs: u32,
    pub timeout_ms: u32,
}

impl Config {
    pub fn new(
        address: &str,
        room: &str,
        poll_interval_secs: u32,
        timeout_ms: u32,
    ) -> Result<Self, &'static str> {
        if address.len() > 253 || room.len() > 64 || !valid_room(room) {
            return Err("invalid board manager address or room");
        }
        let candidate = if address.contains("://") {
            address.to_owned()
        } else {
            format!("http://{address}")
        };
        let mut url = Url::parse(&candidate).map_err(|_| "invalid board manager address")?;
        if !matches!(url.scheme(), "http" | "https")
            || url.host_str().is_none()
            || url.username() != ""
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || !matches!(url.path(), "" | "/")
        {
            return Err("board manager must be an HTTP(S) origin without credentials or path");
        }
        if !(5..=3600).contains(&poll_interval_secs) {
            return Err("poll interval must be between 5 and 3600 seconds");
        }
        if !(100..=10_000).contains(&timeout_ms) {
            return Err("timeout must be between 100 and 10000 milliseconds");
        }
        url.set_path("");
        Ok(Self {
            board_manager_url: url,
            room: room.to_owned(),
            poll_interval_secs,
            timeout_ms,
        })
    }
    pub fn endpoint(&self, path: &str) -> Url {
        self.board_manager_url
            .join(path.trim_start_matches('/'))
            .expect("fixed relative endpoint")
    }
}
fn valid_room(room: &str) -> bool {
    !room.is_empty()
        && room
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Availability {
    Unknown,
    Available,
    Unavailable,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoardState {
    Unknown,
    Observed {
        connected: Option<bool>,
        running: Option<bool>,
        status: Option<String>,
        event: Option<String>,
        num_throws: Option<u32>,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub availability: Availability,
    pub version: Option<String>,
    pub state: BoardState,
    pub diagnostic: Option<Diagnostic>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub stage: String,
    pub kind: String,
    pub http_status: Option<u16>,
}
impl Snapshot {
    pub fn unknown(stage: String, kind: String, http_status: Option<u16>) -> Self {
        Self {
            availability: Availability::Unavailable,
            version: None,
            state: BoardState::Unknown,
            diagnostic: Some(Diagnostic {
                stage,
                kind,
                http_status,
            }),
        }
    }
}
pub trait Transport {
    fn get(&mut self, url: &Url, timeout_ms: u32) -> Result<HttpResponse, HttpFailure>;
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub body: Vec<u8>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HttpFailure {
    Timeout,
    Connection,
    TooLarge,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DetectionCommand {
    Start,
    Stop,
}

pub trait CommandTransport: Transport {
    fn put(&mut self, url: &Url, timeout_ms: u32) -> Result<u16, HttpFailure>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DetectionResult {
    Confirmed(bool),
    Pending {
        desired: bool,
        observed: Option<bool>,
    },
    Failed(&'static str),
}

/// Request a detection transition, then independently observe state once.
/// A command response is only an acknowledgement, never observed state.
pub fn set_detection<T: CommandTransport>(
    config: &Config,
    transport: &mut T,
    command: DetectionCommand,
) -> DetectionResult {
    let desired = command == DetectionCommand::Start;
    let endpoint = if desired { "api/start" } else { "api/stop" };
    let url = config.endpoint(endpoint);
    match transport.put(&url, config.timeout_ms) {
        Ok(status) if (200..300).contains(&status) => {}
        Ok(_) => return DetectionResult::Failed("command_http_status"),
        Err(HttpFailure::Timeout) => return DetectionResult::Failed("command_timeout"),
        Err(HttpFailure::Connection) => return DetectionResult::Failed("command_connection"),
        Err(HttpFailure::TooLarge) => return DetectionResult::Failed("command_response_too_large"),
    }
    let snapshot = poll(config, transport);
    let observed = match snapshot.state {
        BoardState::Observed { running, .. } => running,
        BoardState::Unknown => None,
    };
    if snapshot.availability == Availability::Available && observed == Some(desired) {
        DetectionResult::Confirmed(desired)
    } else {
        DetectionResult::Pending { desired, observed }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnnouncementMode {
    EachDartThenTotal,
    TotalOnly,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnnouncementSettings {
    pub enabled: bool,
    pub mode: Option<AnnouncementMode>,
    pub room: String,
}

impl AnnouncementSettings {
    pub fn new(enabled: bool, mode: Option<&str>, room: &str) -> Result<Self, &'static str> {
        let mode = match mode {
            Some("each_dart_then_total") => Some(AnnouncementMode::EachDartThenTotal),
            Some("total_only") => Some(AnnouncementMode::TotalOnly),
            Some(_) => return Err("invalid announcement mode"),
            None if enabled => {
                return Err("announcement mode must be explicitly selected when enabled");
            }
            None => None,
        };
        if room.len() > 64 || !valid_room(room) {
            return Err("invalid announcement room");
        }
        Ok(Self {
            enabled,
            mode,
            room: room.to_owned(),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnnouncementRequest {
    pub text: String,
    pub room: String,
    pub deduplication_key: String,
}

/// Why the selected delivery adapter did not accept a submission.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeliveryError {
    Unavailable,
    Failed(String),
}

/// Submission boundary for local announcement policy.
///
/// `Ok(())` means that this adapter accepted the request. It does not prove
/// host transport, synthesis, arbitration, playback, or human audibility.
pub trait AnnouncementDelivery {
    fn deliver(&mut self, request: &AnnouncementRequest) -> Result<(), DeliveryError>;
}

/// Production placeholder that explicitly fails rather than claiming acceptance.
#[derive(Default)]
pub struct UnavailableDelivery;
impl AnnouncementDelivery for UnavailableDelivery {
    fn deliver(&mut self, _: &AnnouncementRequest) -> Result<(), DeliveryError> {
        Err(DeliveryError::Unavailable)
    }
}

/// Deterministic adapter that records accepted submissions for policy tests.
#[derive(Default)]
pub struct RecordingDelivery {
    pub requests: Vec<AnnouncementRequest>,
    pub fail_on_call: Option<usize>,
}

impl AnnouncementDelivery for RecordingDelivery {
    fn deliver(&mut self, request: &AnnouncementRequest) -> Result<(), DeliveryError> {
        let call = self.requests.len() + 1;
        if self.fail_on_call == Some(call) {
            return Err(DeliveryError::Failed("synthetic delivery failure".into()));
        }
        self.requests.push(request.clone());
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnnouncementResult {
    Disabled,
    NoAudience,
    Rejected(&'static str),
    Duplicate,
    /// Number of requests accepted by the selected adapter, not delivered.
    Accepted(usize),
    DeliveryFailed(DeliveryError),
}

pub fn poll<T: Transport>(config: &Config, transport: &mut T) -> Snapshot {
    let version = match get(
        transport,
        &config.endpoint("api/version"),
        config.timeout_ms,
    )
    .and_then(decode_version)
    {
        Ok(value) => value,
        Err(error) => return Snapshot::unknown(error.stage, error.kind, error.http_status),
    };
    let state = match get(transport, &config.endpoint("api/state"), config.timeout_ms)
        .and_then(decode_state)
    {
        Ok(state) => state,
        Err(diagnostic) => {
            return Snapshot {
                availability: Availability::Unavailable,
                version: Some(version),
                state: BoardState::Unknown,
                diagnostic: Some(diagnostic),
            };
        }
    };
    Snapshot {
        availability: Availability::Available,
        version: Some(version),
        state,
        diagnostic: None,
    }
}

fn get<T: Transport>(
    transport: &mut T,
    url: &Url,
    timeout_ms: u32,
) -> Result<HttpResponse, Diagnostic> {
    transport
        .get(url, timeout_ms)
        .map_err(|failure| Diagnostic {
            stage: endpoint_name(url).into(),
            kind: match failure {
                HttpFailure::Timeout => "timeout",
                HttpFailure::Connection => "connection",
                HttpFailure::TooLarge => "too_large",
            }
            .into(),
            http_status: None,
        })
        .and_then(|response| {
            if response.body.len() > MAX_RESPONSE_BYTES {
                Err(Diagnostic {
                    stage: endpoint_name(url).into(),
                    kind: "too_large".into(),
                    http_status: Some(response.status),
                })
            } else if !(200..300).contains(&response.status) {
                Err(Diagnostic {
                    stage: endpoint_name(url).into(),
                    kind: "http_status".into(),
                    http_status: Some(response.status),
                })
            } else {
                Ok(response)
            }
        })
}
fn endpoint_name(url: &Url) -> &'static str {
    if url.path().ends_with("/api/version") {
        "version"
    } else {
        "state"
    }
}

fn decode_version(response: HttpResponse) -> Result<String, Diagnostic> {
    let text = std::str::from_utf8(&response.body).map_err(|_| Diagnostic {
        stage: "version".into(),
        kind: "invalid_utf8".into(),
        http_status: Some(response.status),
    })?;
    let version = text.trim();
    if version.is_empty() || version.len() > 128 || version.chars().any(char::is_control) {
        return Err(Diagnostic {
            stage: "version".into(),
            kind: "invalid_version".into(),
            http_status: Some(response.status),
        });
    }
    Ok(version.to_owned())
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawState {
    connected: Option<bool>,
    running: Option<bool>,
    status: Option<String>,
    event: Option<String>,
    num_throws: Option<u32>,
}
pub fn decode_state(response: HttpResponse) -> Result<BoardState, Diagnostic> {
    let raw: RawState = serde_json::from_slice(&response.body).map_err(|_| Diagnostic {
        stage: "state".into(),
        kind: "invalid_state".into(),
        http_status: Some(response.status),
    })?;
    let status = bounded_label(raw.status)?;
    let event = bounded_label(raw.event)?;
    if raw.connected.is_none()
        && raw.running.is_none()
        && status.is_none()
        && event.is_none()
        && raw.num_throws.is_none()
    {
        return Ok(BoardState::Unknown);
    }
    Ok(BoardState::Observed {
        connected: raw.connected,
        running: raw.running,
        status,
        event,
        num_throws: raw.num_throws,
    })
}
pub fn bounded_label(value: Option<String>) -> Result<Option<String>, Diagnostic> {
    let Some(value) = value else { return Ok(None) };
    if value.len() > 64 || value.chars().any(char::is_control) {
        return Err(Diagnostic {
            stage: "state".into(),
            kind: "invalid_label".into(),
            http_status: None,
        });
    }
    Ok(Some(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    struct FixtureTransport {
        replies: VecDeque<Result<HttpResponse, HttpFailure>>,
        requests: Vec<(String, u32)>,
    }
    impl FixtureTransport {
        fn new(replies: impl IntoIterator<Item = Result<HttpResponse, HttpFailure>>) -> Self {
            Self {
                replies: replies.into_iter().collect(),
                requests: Vec::new(),
            }
        }
    }
    impl Transport for FixtureTransport {
        fn get(&mut self, url: &Url, timeout_ms: u32) -> Result<HttpResponse, HttpFailure> {
            self.requests.push((url.to_string(), timeout_ms));
            self.replies.pop_front().expect("fixture reply")
        }
    }
    fn response(status: u16, body: &str) -> Result<HttpResponse, HttpFailure> {
        Ok(HttpResponse {
            status,
            body: body.as_bytes().to_vec(),
        })
    }
    fn config() -> Config {
        Config::new("board-manager.local:3180", "living_room", 15, 1500).unwrap()
    }

    #[test]
    fn composed_config_poll_and_state_decoding_use_only_expected_gets() {
        let mut transport = FixtureTransport::new([
            response(200, "board-manager 1.2.3\n"),
            response(
                200,
                r#"{"connected":true,"running":false,"status":"Stopped","event":"Stopped","numThrows":0,"throws":[{"segment":{"number":20}}]}"#,
            ),
        ]);
        let snapshot = poll(&config(), &mut transport);
        assert_eq!(snapshot.availability, Availability::Available);
        assert_eq!(snapshot.version.as_deref(), Some("board-manager 1.2.3"));
        assert_eq!(
            snapshot.state,
            BoardState::Observed {
                connected: Some(true),
                running: Some(false),
                status: Some("Stopped".into()),
                event: Some("Stopped".into()),
                num_throws: Some(0)
            }
        );
        assert!(snapshot.diagnostic.is_none());
        assert_eq!(transport.requests.len(), 2);
        assert!(transport.requests[0].0.ends_with("/api/version"));
        assert!(transport.requests[1].0.ends_with("/api/state"));
        assert!(
            transport
                .requests
                .iter()
                .all(|(_, timeout)| *timeout == 1500)
        );
    }
    #[test]
    fn configuration_rejects_credentials_paths_bad_rooms_and_unbounded_intervals() {
        for (address, room, interval) in [
            ("http://user:secret@board.local", "living_room", 15),
            ("http://board.local/path", "living_room", 15),
            ("http://board.local", "Living Room", 15),
            ("http://board.local", "living_room", 4),
        ] {
            assert!(Config::new(address, room, interval, 500).is_err());
        }
    }
    #[test]
    fn http_and_timeout_failures_make_device_unavailable_without_false_state() {
        let mut http = FixtureTransport::new([
            response(200, "1.2.3"),
            response(503, "private diagnostic body must not escape"),
        ]);
        let failed = poll(&config(), &mut http);
        assert_eq!(failed.availability, Availability::Unavailable);
        assert_eq!(failed.state, BoardState::Unknown);
        assert_eq!(failed.diagnostic.unwrap().http_status, Some(503));
        let mut timeout = FixtureTransport::new([Err(HttpFailure::Timeout)]);
        let failed = poll(&config(), &mut timeout);
        assert_eq!(failed.availability, Availability::Unavailable);
        assert_eq!(failed.diagnostic.unwrap().kind, "timeout");
    }
    #[test]
    fn malformed_and_oversized_responses_are_bounded_and_do_not_preserve_stale_state() {
        let mut malformed =
            FixtureTransport::new([response(200, "1.2.3"), response(200, "not json")]);
        let failed = poll(&config(), &mut malformed);
        assert_eq!(failed.state, BoardState::Unknown);
        assert_eq!(failed.availability, Availability::Unavailable);
        let mut oversized = FixtureTransport::new([Ok(HttpResponse {
            status: 200,
            body: vec![b'x'; MAX_RESPONSE_BYTES + 1],
        })]);
        let failed = poll(&config(), &mut oversized);
        assert_eq!(failed.diagnostic.unwrap().kind, "too_large");
    }
    #[test]
    fn unknown_vendor_values_remain_unknown_and_throw_arrays_are_not_scores() {
        let mut transport = FixtureTransport::new([
            response(200, "1.2.3"),
            response(
                200,
                r#"{"status":"FutureState","event":"FutureEvent","throws":[{"segment":{"number":20,"multiplier":3}}]}"#,
            ),
        ]);
        let snapshot = poll(&config(), &mut transport);
        assert_eq!(snapshot.availability, Availability::Available);
        assert_eq!(
            snapshot.state,
            BoardState::Observed {
                connected: None,
                running: None,
                status: Some("FutureState".into()),
                event: Some("FutureEvent".into()),
                num_throws: None
            }
        );
    }
}
