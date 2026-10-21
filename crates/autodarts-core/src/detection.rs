use crate::{Availability, BoardState, Config, HttpFailure, Snapshot, Transport};
use url::Url;

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

/// Send one detection command and make one bounded independent state reread.
/// The old snapshot is intentionally not an input: only fresh provider state
/// can confirm the requested transition.
pub fn set_detection<T: CommandTransport>(
    config: &Config,
    transport: &mut T,
    command: DetectionCommand,
) -> (DetectionResult, Snapshot) {
    let desired = command == DetectionCommand::Start;
    let endpoint = if desired { "api/start" } else { "api/stop" };
    match transport.put(&config.endpoint(endpoint), config.timeout_ms) {
        Ok(status) if (200..300).contains(&status) => {}
        Ok(_) => {
            return (
                DetectionResult::Failed("command_http_status"),
                Snapshot::unknown("command".into(), "http_status".into(), None),
            );
        }
        Err(HttpFailure::Timeout) => {
            return (
                DetectionResult::Failed("command_timeout"),
                Snapshot::unknown("command".into(), "timeout".into(), None),
            );
        }
        Err(HttpFailure::Connection) => {
            return (
                DetectionResult::Failed("command_connection"),
                Snapshot::unknown("command".into(), "connection".into(), None),
            );
        }
        Err(HttpFailure::TooLarge) => {
            return (
                DetectionResult::Failed("command_response_too_large"),
                Snapshot::unknown("command".into(), "too_large".into(), None),
            );
        }
    }
    let snapshot = poll_state(config, transport);
    let observed = match snapshot.state {
        BoardState::Observed { running, .. } => running,
        BoardState::Unknown => None,
    };
    let result = if snapshot.availability == Availability::Available && observed == Some(desired) {
        DetectionResult::Confirmed(desired)
    } else {
        DetectionResult::Pending { desired, observed }
    };
    (result, snapshot)
}

pub fn poll_state<T: Transport>(config: &Config, transport: &mut T) -> Snapshot {
    match transport.get(&config.endpoint("api/state"), config.timeout_ms) {
        Ok(response)
            if (200..300).contains(&response.status)
                && response.body.len() <= crate::MAX_RESPONSE_BYTES =>
        {
            match crate::decode_state(response) {
                Ok(state) => Snapshot {
                    availability: Availability::Available,
                    version: None,
                    state,
                    diagnostic: None,
                },
                Err(error) => Snapshot::unknown(error.stage, error.kind, error.http_status),
            }
        }
        Ok(response) => {
            Snapshot::unknown("state".into(), "http_status".into(), Some(response.status))
        }
        Err(HttpFailure::Timeout) => Snapshot::unknown("state".into(), "timeout".into(), None),
        Err(HttpFailure::Connection) => {
            Snapshot::unknown("state".into(), "connection".into(), None)
        }
        Err(HttpFailure::TooLarge) => Snapshot::unknown("state".into(), "too_large".into(), None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HttpResponse;
    use std::collections::VecDeque;

    struct Fixture {
        puts: Vec<(String, u32)>,
        gets: Vec<(String, u32)>,
        put_result: Result<u16, HttpFailure>,
        replies: VecDeque<Result<HttpResponse, HttpFailure>>,
    }

    impl Fixture {
        fn new(
            put_result: Result<u16, HttpFailure>,
            replies: impl IntoIterator<Item = Result<HttpResponse, HttpFailure>>,
        ) -> Self {
            Self {
                puts: vec![],
                gets: vec![],
                put_result,
                replies: replies.into_iter().collect(),
            }
        }
    }

    impl Transport for Fixture {
        fn get(&mut self, url: &Url, timeout_ms: u32) -> Result<HttpResponse, HttpFailure> {
            self.gets.push((url.to_string(), timeout_ms));
            self.replies.pop_front().expect("fixture reply")
        }
    }

    impl CommandTransport for Fixture {
        fn put(&mut self, url: &Url, timeout_ms: u32) -> Result<u16, HttpFailure> {
            self.puts.push((url.to_string(), timeout_ms));
            self.put_result
        }
    }

    fn response(status: u16, body: &str) -> Result<HttpResponse, HttpFailure> {
        Ok(HttpResponse {
            status,
            body: body.as_bytes().to_vec(),
        })
    }

    fn config() -> Config {
        Config::new("board.local:3180", "living_room", 15, 1500).unwrap()
    }

    fn state(running: bool) -> [Result<HttpResponse, HttpFailure>; 1] {
        [response(200, &format!(r#"{{"running":{running}}}"#))]
    }

    #[test]
    fn start_and_stop_use_exact_put_paths_then_fresh_state_reads() {
        for (command, path, running) in [
            (DetectionCommand::Start, "api/start", true),
            (DetectionCommand::Stop, "api/stop", false),
        ] {
            let mut fixture = Fixture::new(Ok(204), state(running));
            let (result, snapshot) = set_detection(&config(), &mut fixture, command);
            assert_eq!(result, DetectionResult::Confirmed(running));
            assert!(
                matches!(snapshot.state, BoardState::Observed { running: Some(value), .. } if value == running)
            );
            assert_eq!(
                fixture.puts,
                [(format!("http://board.local:3180/{path}"), 1500)]
            );
            assert_eq!(
                fixture.gets,
                [("http://board.local:3180/api/state".into(), 1500)]
            );
        }
    }

    #[test]
    fn acknowledged_start_with_observed_false_is_pending_not_success() {
        let mut fixture = Fixture::new(Ok(200), state(false));
        let (result, snapshot) = set_detection(&config(), &mut fixture, DetectionCommand::Start);
        assert_eq!(
            result,
            DetectionResult::Pending {
                desired: true,
                observed: Some(false)
            }
        );
        assert!(matches!(
            snapshot.state,
            BoardState::Observed {
                running: Some(false),
                ..
            }
        ));
    }

    #[test]
    fn command_timeout_or_http_failure_does_not_reread_or_claim_success() {
        for (result, expected) in [
            (
                Err(HttpFailure::Timeout),
                DetectionResult::Failed("command_timeout"),
            ),
            (Ok(503), DetectionResult::Failed("command_http_status")),
        ] {
            let mut fixture = Fixture::new(result, []);
            let (result, snapshot) =
                set_detection(&config(), &mut fixture, DetectionCommand::Start);
            assert_eq!(result, expected);
            assert_eq!(snapshot.availability, Availability::Unavailable);
            assert!(fixture.gets.is_empty());
        }
    }

    #[test]
    fn state_timeout_and_unknown_observation_remain_pending() {
        let mut timed_out = Fixture::new(Ok(204), [Err(HttpFailure::Timeout)]);
        let (result, snapshot) = set_detection(&config(), &mut timed_out, DetectionCommand::Start);
        assert_eq!(
            result,
            DetectionResult::Pending {
                desired: true,
                observed: None
            }
        );
        assert_eq!(snapshot.availability, Availability::Unavailable);
        assert_eq!(timed_out.gets.len(), 1);
        let mut missing = Fixture::new(Ok(204), [response(200, "{}")]);
        let (result, snapshot) = set_detection(&config(), &mut missing, DetectionCommand::Stop);
        assert_eq!(
            result,
            DetectionResult::Pending {
                desired: false,
                observed: None
            }
        );
        assert_eq!(snapshot.availability, Availability::Available);
        assert_eq!(snapshot.state, BoardState::Unknown);
        assert_eq!(missing.gets.len(), 1);
    }

    #[test]
    fn legacy_snapshot_cannot_be_used_to_claim_a_transition() {
        let stale = Snapshot {
            availability: Availability::Available,
            version: Some("1".into()),
            state: BoardState::Observed {
                connected: Some(true),
                running: Some(true),
                status: None,
                event: None,
                num_throws: None,
            },
            diagnostic: None,
        };
        let mut fixture = Fixture::new(Ok(200), state(false));
        assert_ne!(stale.state, poll_state(&config(), &mut fixture).state);
    }
}
