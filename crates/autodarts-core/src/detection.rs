use crate::{Availability, BoardState, Config, HttpFailure, Snapshot, Transport};
use url::Url;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DetectionCommand {
    Start,
    Stop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DetectionDispatch {
    /// The provider returned a successful HTTP status. This is acknowledgement only.
    Acknowledged,
    /// The provider response deadline elapsed before the host had a complete answer.
    Uncertain(&'static str),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DetectionResult {
    Confirmed(bool),
    Pending {
        desired: bool,
        observed: Option<bool>,
        dispatch: DetectionDispatch,
    },
}

pub trait CommandTransport: Transport {
    fn put(&mut self, url: &Url, timeout_ms: u32) -> Result<u16, HttpFailure>;
}

/// Submit one bounded command. A timeout is not evidence that the provider rejected it:
/// the request may have been accepted before the whole response deadline elapsed.
pub fn dispatch_detection<T: CommandTransport>(
    config: &Config,
    transport: &mut T,
    command: DetectionCommand,
) -> Result<DetectionDispatch, &'static str> {
    let endpoint = if command == DetectionCommand::Start {
        "api/start"
    } else {
        "api/stop"
    };
    match transport.put(&config.endpoint(endpoint), config.timeout_ms) {
        Ok(status) if (200..300).contains(&status) => Ok(DetectionDispatch::Acknowledged),
        Ok(_) => Err("command_http_status"),
        Err(HttpFailure::Timeout) => Ok(DetectionDispatch::Uncertain("command_timeout")),
        Err(HttpFailure::Connection) => Err("command_connection"),
        Err(HttpFailure::TooLarge) => {
            Ok(DetectionDispatch::Uncertain("command_response_too_large"))
        }
    }
}

pub fn detection_result(
    desired: bool,
    dispatch: DetectionDispatch,
    snapshot: &Snapshot,
) -> DetectionResult {
    let (observed, starting) = match &snapshot.state {
        BoardState::Observed {
            running,
            status,
            event,
            connected,
            ..
        } => (
            *running,
            connected == &Some(false)
                || status.as_deref() == Some("Starting")
                || event.as_deref() == Some("Starting"),
        ),
        BoardState::Unknown => (None, false),
    };
    if snapshot.availability == Availability::Available && !starting && observed == Some(desired) {
        DetectionResult::Confirmed(desired)
    } else {
        DetectionResult::Pending {
            desired,
            observed,
            dispatch,
        }
    }
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
    fn dispatch_and_reconciliation_keep_acknowledgement_separate_from_state() {
        let mut fixture = Fixture::new(Ok(204), state(true));
        let dispatch = dispatch_detection(&config(), &mut fixture, DetectionCommand::Start);
        assert_eq!(dispatch, Ok(DetectionDispatch::Acknowledged));
        assert_eq!(fixture.gets.len(), 0);
        let snapshot = poll_state(&config(), &mut fixture);
        assert_eq!(
            detection_result(true, dispatch.unwrap(), &snapshot),
            DetectionResult::Confirmed(true)
        );
        assert_eq!(
            fixture.puts,
            [("http://board.local:3180/api/start".into(), 1500)]
        );
        assert_eq!(
            fixture.gets,
            [("http://board.local:3180/api/state".into(), 1500)]
        );
    }

    #[test]
    fn acknowledged_start_with_observed_false_is_pending_not_success() {
        let mut fixture = Fixture::new(Ok(200), state(false));
        let dispatch =
            dispatch_detection(&config(), &mut fixture, DetectionCommand::Start).unwrap();
        let snapshot = poll_state(&config(), &mut fixture);
        assert_eq!(
            detection_result(true, dispatch, &snapshot),
            DetectionResult::Pending {
                desired: true,
                observed: Some(false),
                dispatch: DetectionDispatch::Acknowledged,
            }
        );
    }
    #[test]
    fn starting_running_true_is_not_confirmation() {
        let mut fixture = Fixture::new(
            Ok(200),
            [response(200, r#"{"status":"Starting","running":true}"#)],
        );
        let dispatch =
            dispatch_detection(&config(), &mut fixture, DetectionCommand::Start).unwrap();
        let snapshot = poll_state(&config(), &mut fixture);
        assert!(matches!(
            detection_result(true, dispatch, &snapshot),
            DetectionResult::Pending {
                observed: Some(true),
                ..
            }
        ));
    }

    #[test]
    fn response_timeout_is_uncertain_without_a_false_failure() {
        for error in [HttpFailure::Timeout, HttpFailure::TooLarge] {
            let mut fixture = Fixture::new(Err(error), []);
            let dispatch = dispatch_detection(&config(), &mut fixture, DetectionCommand::Start);
            assert_eq!(
                dispatch,
                Ok(DetectionDispatch::Uncertain(match error {
                    HttpFailure::Timeout => "command_timeout",
                    HttpFailure::TooLarge => "command_response_too_large",
                    HttpFailure::Connection => unreachable!(),
                }))
            );
            assert!(fixture.gets.is_empty());
        }
    }

    #[test]
    fn connection_error_is_ambiguous_without_dispatch_phase() {
        let mut fixture = Fixture::new(Err(HttpFailure::Connection), []);
        assert_eq!(
            dispatch_detection(&config(), &mut fixture, DetectionCommand::Start),
            Err("command_connection")
        );
        assert!(fixture.gets.is_empty());
    }

    #[test]
    fn complete_http_rejection_is_definite() {
        let mut fixture = Fixture::new(Ok(503), []);
        assert_eq!(
            dispatch_detection(&config(), &mut fixture, DetectionCommand::Start),
            Err("command_http_status")
        );
        assert!(fixture.gets.is_empty());
    }

    #[test]
    fn state_timeout_and_unknown_observation_remain_pending() {
        let mut timed_out = Fixture::new(Ok(204), [Err(HttpFailure::Timeout)]);
        let dispatch =
            dispatch_detection(&config(), &mut timed_out, DetectionCommand::Start).unwrap();
        let snapshot = poll_state(&config(), &mut timed_out);
        assert_eq!(
            detection_result(true, dispatch, &snapshot),
            DetectionResult::Pending {
                desired: true,
                observed: None,
                dispatch: DetectionDispatch::Acknowledged,
            }
        );
        assert_eq!(snapshot.availability, Availability::Unavailable);
        let mut missing = Fixture::new(Ok(204), [response(200, "{}")]);
        let dispatch = dispatch_detection(&config(), &mut missing, DetectionCommand::Stop).unwrap();
        let snapshot = poll_state(&config(), &mut missing);
        assert_eq!(
            detection_result(false, dispatch, &snapshot),
            DetectionResult::Pending {
                desired: false,
                observed: None,
                dispatch: DetectionDispatch::Acknowledged,
            }
        );
        assert_eq!(snapshot.availability, Availability::Available);
        assert_eq!(snapshot.state, BoardState::Unknown);
    }
}
