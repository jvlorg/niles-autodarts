use crate::local_score::{
    AnnouncementDiagnostics, AnnouncementReadiness, JourneyResult, LocalEvent, LocalJourney,
    RoundToken, parse_json,
};
use crate::{AnnouncementDelivery, AnnouncementSettings};

/// Transport-independent owner for one configured Board Manager connection.
/// It does not provide event ingress; callers pass each received JSON envelope.
pub struct AnnouncementSession {
    settings: AnnouncementSettings,
    journey: LocalJourney,
}

impl AnnouncementSession {
    pub fn new(settings: AnnouncementSettings) -> Self {
        Self {
            settings,
            journey: LocalJourney::default(),
        }
    }
    pub fn settings(&self) -> &AnnouncementSettings {
        &self.settings
    }
    pub fn diagnostics(&self) -> AnnouncementDiagnostics {
        let mut diagnostics = self.journey.diagnostics();
        if !self.settings.enabled || self.settings.mode.is_none() {
            diagnostics.readiness = AnnouncementReadiness::Disabled;
        } else if diagnostics.readiness == AnnouncementReadiness::Disabled {
            diagnostics.readiness = AnnouncementReadiness::AwaitingBaseline;
        }
        diagnostics
    }
    /// Apply validated settings. Any change invalidates collection state and requires a clean baseline.
    pub fn reconfigure(&mut self, settings: AnnouncementSettings) {
        if self.settings != settings {
            self.journey.reconnect();
            self.settings = settings;
        }
    }
    /// Mark a transport reconnect and require a new baseline.
    pub fn connection_reset(&mut self) {
        self.journey.reconnect();
    }
    /// Parse and ingest a JSON envelope. Parse errors taint state, preventing stale candidates from announcing.
    pub fn ingest_json<D: AnnouncementDelivery>(
        &mut self,
        json: &[u8],
        delivery: &mut D,
    ) -> JourneyResult {
        let event = parse_json(json).unwrap_or(LocalEvent::Invalid);
        let Some(mode) = self.settings.mode else {
            self.journey.reconnect();
            return JourneyResult::Suppressed;
        };
        self.journey.ingest(
            self.settings.enabled,
            mode,
            &self.settings.room,
            event,
            delivery,
        )
    }
    pub fn pending_total_deadline(&self) -> Option<RoundToken> {
        self.journey.pending_total_deadline()
    }

    /// Deliver the one-shot total deadline through the same reducer path as takeout.
    pub fn on_total_deadline<D: AnnouncementDelivery>(
        &mut self,
        token: RoundToken,
        delivery: &mut D,
    ) -> JourneyResult {
        let Some(mode) = self.settings.mode else {
            return JourneyResult::Suppressed;
        };
        if !self.settings.enabled {
            return JourneyResult::Suppressed;
        }
        self.journey
            .on_total_deadline(token, mode, &self.settings.room, delivery)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AnnouncementMode, DeliveryError, RecordingDelivery, UnavailableDelivery};

    fn settings(enabled: bool, mode: &str, room: &str) -> AnnouncementSettings {
        AnnouncementSettings::new(enabled, Some(mode), room).unwrap()
    }
    const EMPTY: &str = r#"{"type":"state","data":{"running":true,"numThrows":0,"throws":[]}}"#;
    const DART: &str = r#"{"type":"state","data":{"running":true,"numThrows":1,"throws":[{"segment":{"number":20,"multiplier":1}}]}}"#;
    const FULL: &str = r#"{"type":"motion_state","data":{"isTakeoutFull":true}}"#;
    const NEUTRAL: &str =
        r#"{"type":"motion_state","data":{"isTakeoutPartial":false,"isTakeoutFull":false}}"#;

    fn snapshot(scores: &[u8]) -> serde_json::Value {
        serde_json::json!({
            "type": "state",
            "data": {
                "running": true,
                "numThrows": scores.len(),
                "throws": scores.iter().map(|score| serde_json::json!({
                    "segment": {"number": score, "multiplier": 1}
                })).collect::<Vec<_>>()
            }
        })
    }

    fn ingest_value(
        session: &mut AnnouncementSession,
        delivery: &mut RecordingDelivery,
        value: &serde_json::Value,
    ) -> JourneyResult {
        session.ingest_json(&serde_json::to_vec(value).unwrap(), delivery)
    }

    fn texts(delivery: &RecordingDelivery) -> Vec<&str> {
        delivery
            .requests
            .iter()
            .map(|request| request.text.as_str())
            .collect()
    }

    #[test]
    fn settings_json_and_recording_delivery_are_composed() {
        let mut session =
            AnnouncementSession::new(settings(true, "each_dart_then_total", "living_room"));
        let mut delivery = RecordingDelivery::default();
        assert_eq!(
            session.ingest_json(EMPTY.as_bytes(), &mut delivery),
            JourneyResult::Baseline
        );
        assert_eq!(
            session.ingest_json(DART.as_bytes(), &mut delivery),
            JourneyResult::Dart(Some(20))
        );
        assert_eq!(
            session.ingest_json(FULL.as_bytes(), &mut delivery),
            JourneyResult::Total(Some(20))
        );
        assert_eq!(
            delivery
                .requests
                .iter()
                .map(|r| r.text.as_str())
                .collect::<Vec<_>>(),
            ["20", "20"]
        );
        assert_eq!(delivery.requests[0].room, "living_room");
    }

    #[test]
    fn total_only_clear_before_marker_remains_eligible() {
        let mut session = AnnouncementSession::new(settings(true, "total_only", "living_room"));
        let mut delivery = RecordingDelivery::default();
        session.ingest_json(EMPTY.as_bytes(), &mut delivery);
        session.ingest_json(DART.as_bytes(), &mut delivery);
        assert!(delivery.requests.is_empty());
        assert_eq!(
            session.ingest_json(EMPTY.as_bytes(), &mut delivery),
            JourneyResult::Suppressed
        );
        assert_eq!(
            session.ingest_json(FULL.as_bytes(), &mut delivery),
            JourneyResult::Total(Some(20))
        );
        assert_eq!(delivery.requests.len(), 1);
        assert_eq!(delivery.requests[0].text, "20");
    }

    #[test]
    fn parse_error_invalidates_candidate() {
        let mut session = AnnouncementSession::new(settings(true, "total_only", "living_room"));
        let mut delivery = RecordingDelivery::default();
        session.ingest_json(EMPTY.as_bytes(), &mut delivery);
        session.ingest_json(DART.as_bytes(), &mut delivery);
        assert_eq!(
            session.ingest_json(b"{bad", &mut delivery),
            JourneyResult::Invalid
        );
        assert_eq!(
            session.ingest_json(FULL.as_bytes(), &mut delivery),
            JourneyResult::Suppressed
        );
        assert!(delivery.requests.is_empty());
    }

    #[test]
    fn reconfiguration_and_connection_reset_require_baseline() {
        let mut session = AnnouncementSession::new(settings(true, "total_only", "living_room"));
        let mut delivery = RecordingDelivery::default();
        session.ingest_json(EMPTY.as_bytes(), &mut delivery);
        session.ingest_json(DART.as_bytes(), &mut delivery);
        session.reconfigure(settings(true, "total_only", "office"));
        assert_eq!(
            session.ingest_json(FULL.as_bytes(), &mut delivery),
            JourneyResult::Suppressed
        );
        session.ingest_json(EMPTY.as_bytes(), &mut delivery);
        session.ingest_json(DART.as_bytes(), &mut delivery);
        session.connection_reset();
        assert_eq!(
            session.ingest_json(FULL.as_bytes(), &mut delivery),
            JourneyResult::Suppressed
        );
        assert!(delivery.requests.is_empty());
        assert_eq!(session.settings().mode, Some(AnnouncementMode::TotalOnly));
    }
    #[test]
    fn disabling_settings_reports_disabled_readiness_after_prior_ingress() {
        let mut session = AnnouncementSession::new(settings(true, "total_only", "living_room"));
        let mut delivery = RecordingDelivery::default();
        session.ingest_json(EMPTY.as_bytes(), &mut delivery);
        session.reconfigure(AnnouncementSettings::new(false, None, "living_room").unwrap());
        assert_eq!(
            session.diagnostics(),
            AnnouncementDiagnostics {
                readiness: AnnouncementReadiness::Disabled,
                last_event: None,
            }
        );
    }

    #[test]
    fn unavailable_attempt_is_not_retried() {
        let mut session = AnnouncementSession::new(settings(true, "total_only", "living_room"));
        let mut delivery = UnavailableDelivery;
        session.ingest_json(EMPTY.as_bytes(), &mut delivery);
        session.ingest_json(DART.as_bytes(), &mut delivery);
        assert_eq!(
            session.ingest_json(FULL.as_bytes(), &mut delivery),
            JourneyResult::DeliveryFailed(DeliveryError::Unavailable)
        );
        assert_eq!(
            session.ingest_json(FULL.as_bytes(), &mut delivery),
            JourneyResult::Suppressed
        );
    }
    #[test]
    fn supplied_capture_stays_silent_then_continuation_announces_three_rounds() {
        let capture: serde_json::Value = serde_json::from_slice(include_bytes!(
            "../tests/fixtures/board-manager-1.0.7-reduced-capture.json"
        ))
        .unwrap();
        let envelopes = capture.as_array().unwrap();
        assert_eq!(
            capture,
            serde_json::json!([
                {
                    "type": "motion_state",
                    "data": {
                        "isTakeoutPartial": false,
                        "isTakeoutFull": false
                    }
                },
                {
                    "type": "state",
                    "data": {
                        "connected": true,
                        "running": true,
                        "status": "Throw",
                        "event": "Throw detected",
                        "numThrows": 1,
                        "throws": [{
                            "segment": {
                                "name": "S3",
                                "number": 3,
                                "bed": "SingleOuter",
                                "multiplier": 1
                            }
                        }]
                    }
                },
                {
                    "type": "motion_state",
                    "data": {
                        "isTakeoutPartial": false,
                        "isTakeoutFull": false
                    }
                },
                {
                    "type": "state",
                    "data": {
                        "connected": true,
                        "running": true,
                        "status": "Throw",
                        "event": "Throw detected",
                        "numThrows": 2,
                        "throws": [
                            {
                                "segment": {
                                    "name": "S3",
                                    "number": 3,
                                    "bed": "SingleOuter",
                                    "multiplier": 1
                                }
                            },
                            {
                                "segment": {
                                    "name": "S7",
                                    "number": 7,
                                    "bed": "SingleOuter",
                                    "multiplier": 1
                                }
                            }
                        ]
                    }
                },
                {
                    "type": "motion_state",
                    "data": {
                        "isTakeoutPartial": false,
                        "isTakeoutFull": false
                    }
                },
                {
                    "type": "state",
                    "data": {
                        "connected": true,
                        "running": true,
                        "status": "Takeout",
                        "event": "Throw detected",
                        "numThrows": 3,
                        "throws": [
                            {
                                "segment": {
                                    "name": "S3",
                                    "number": 3,
                                    "bed": "SingleOuter",
                                    "multiplier": 1
                                }
                            },
                            {
                                "segment": {
                                    "name": "S7",
                                    "number": 7,
                                    "bed": "SingleOuter",
                                    "multiplier": 1
                                }
                            },
                            {
                                "segment": {
                                    "name": "S7",
                                    "number": 7,
                                    "bed": "SingleOuter",
                                    "multiplier": 1
                                }
                            }
                        ]
                    }
                },
                {
                    "type": "motion_state",
                    "data": {
                        "isTakeoutPartial": false,
                        "isTakeoutFull": false
                    }
                },
                {
                    "type": "state",
                    "data": {
                        "connected": true,
                        "running": true,
                        "status": "Takeout in progress",
                        "event": "Takeout started",
                        "numThrows": 3,
                        "throws": [
                            {
                                "segment": {
                                    "name": "S3",
                                    "number": 3,
                                    "bed": "SingleOuter",
                                    "multiplier": 1
                                }
                            },
                            {
                                "segment": {
                                    "name": "S7",
                                    "number": 7,
                                    "bed": "SingleOuter",
                                    "multiplier": 1
                                }
                            },
                            {
                                "segment": {
                                    "name": "S7",
                                    "number": 7,
                                    "bed": "SingleOuter",
                                    "multiplier": 1
                                }
                            }
                        ]
                    }
                },
                {
                    "type": "motion_state",
                    "data": {
                        "isTakeoutPartial": false,
                        "isTakeoutFull": false
                    }
                },
                {
                    "type": "motion_state",
                    "data": {
                        "isTakeoutPartial": true,
                        "isTakeoutFull": true
                    }
                },
                {
                    "type": "motion_state",
                    "data": {
                        "isTakeoutPartial": true,
                        "isTakeoutFull": false
                    }
                },
                {
                    "type": "state",
                    "data": {
                        "connected": true,
                        "running": true,
                        "status": "Throw",
                        "event": "Takeout finished",
                        "numThrows": 0
                    }
                },
                {
                    "type": "motion_state",
                    "data": {
                        "isTakeoutPartial": true,
                        "isTakeoutFull": true
                    }
                },
                {
                    "type": "motion_state",
                    "data": {
                        "isTakeoutPartial": false,
                        "isTakeoutFull": false
                    }
                }
            ])
        );
        assert_eq!(envelopes.len(), 14);
        assert_eq!(
            envelopes
                .iter()
                .map(|envelope| envelope["type"].as_str().unwrap())
                .collect::<Vec<_>>(),
            [
                "motion_state",
                "state",
                "motion_state",
                "state",
                "motion_state",
                "state",
                "motion_state",
                "state",
                "motion_state",
                "motion_state",
                "motion_state",
                "state",
                "motion_state",
                "motion_state",
            ]
        );
        assert_eq!(
            envelopes[1]["data"]["throws"][0]["segment"],
            serde_json::json!({
                "name": "S3",
                "number": 3,
                "bed": "SingleOuter",
                "multiplier": 1
            })
        );
        assert_eq!(
            envelopes[5]["data"]["throws"],
            serde_json::json!([
                {
                    "segment": {
                        "name": "S3",
                        "number": 3,
                        "bed": "SingleOuter",
                        "multiplier": 1
                    }
                },
                {
                    "segment": {
                        "name": "S7",
                        "number": 7,
                        "bed": "SingleOuter",
                        "multiplier": 1
                    }
                },
                {
                    "segment": {
                        "name": "S7",
                        "number": 7,
                        "bed": "SingleOuter",
                        "multiplier": 1
                    }
                }
            ])
        );
        assert_eq!(
            envelopes[11]["data"],
            serde_json::json!({
                "connected": true,
                "running": true,
                "status": "Throw",
                "event": "Takeout finished",
                "numThrows": 0
            })
        );
        assert!(
            !envelopes[11]["data"]
                .as_object()
                .unwrap()
                .contains_key("throws")
        );
        assert_eq!(
            envelopes[12]["data"],
            serde_json::json!({
                "isTakeoutPartial": true,
                "isTakeoutFull": true
            })
        );
        assert_eq!(
            envelopes[13]["data"],
            serde_json::json!({
                "isTakeoutPartial": false,
                "isTakeoutFull": false
            })
        );

        let continuation: serde_json::Value = serde_json::from_slice(include_bytes!(
            "../tests/fixtures/board-manager-1.0.7-synthetic-continuation.json"
        ))
        .unwrap();
        for (mode, expected) in [
            (
                AnnouncementMode::EachDartThenTotal,
                [
                    "3", "7", "7", "17", "3", "7", "7", "17", "3", "7", "7", "17",
                ]
                .as_slice(),
            ),
            (AnnouncementMode::TotalOnly, ["17", "17", "17"].as_slice()),
        ] {
            let mode_name = match mode {
                AnnouncementMode::EachDartThenTotal => "each_dart_then_total",
                AnnouncementMode::TotalOnly => "total_only",
            };
            let mut session = AnnouncementSession::new(settings(true, mode_name, "living_room"));
            let mut delivery = RecordingDelivery::default();
            for (index, envelope) in envelopes.iter().enumerate() {
                session.ingest_json(&serde_json::to_vec(envelope).unwrap(), &mut delivery);
                assert!(delivery.requests.is_empty());
                if index == 11 {
                    assert_eq!(
                        session.diagnostics().readiness,
                        AnnouncementReadiness::Collecting { dart_count: 0 }
                    );
                }
            }
            assert_eq!(
                session.diagnostics().readiness,
                AnnouncementReadiness::Collecting { dart_count: 0 }
            );

            for (index, envelope) in continuation.as_array().unwrap().iter().enumerate() {
                // Separate synthetic motion schedule; the stored fixture is unchanged.
                if matches!(index, 0 | 7 | 12) {
                    session.ingest_json(NEUTRAL.as_bytes(), &mut delivery);
                }
                session.ingest_json(&serde_json::to_vec(envelope).unwrap(), &mut delivery);
            }
            assert_eq!(
                delivery
                    .requests
                    .iter()
                    .map(|request| request.text.as_str())
                    .collect::<Vec<_>>(),
                expected
            );
        }
    }
    #[test]
    fn deadline_token_is_exposed_and_expiry_shares_total_path() {
        let mut session = AnnouncementSession::new(settings(true, "total_only", "living_room"));
        let mut delivery = RecordingDelivery::default();
        session.ingest_json(EMPTY.as_bytes(), &mut delivery);
        session.ingest_json(DART.as_bytes(), &mut delivery);
        let dart_two = r#"{"type":"state","data":{"running":true,"numThrows":2,"throws":[{"segment":{"number":20,"multiplier":1}},{"segment":{"number":5,"multiplier":1}}]}}"#;
        let dart_three = r#"{"type":"state","data":{"running":true,"numThrows":3,"throws":[{"segment":{"number":20,"multiplier":1}},{"segment":{"number":5,"multiplier":1}},{"segment":{"number":1,"multiplier":1}}]}}"#;
        session.ingest_json(dart_two.as_bytes(), &mut delivery);
        session.ingest_json(dart_three.as_bytes(), &mut delivery);
        let token = session.pending_total_deadline().expect("deadline");
        assert_eq!(
            session.on_total_deadline(token, &mut delivery),
            JourneyResult::Total(Some(26))
        );
        assert_eq!(session.pending_total_deadline(), None);
        assert_eq!(
            delivery
                .requests
                .iter()
                .map(|r| r.text.as_str())
                .collect::<Vec<_>>(),
            ["26"]
        );
    }

    #[test]
    fn explicit_state_boundaries_validate_and_preserve_clear_for_next_round() {
        for mode in ["each_dart_then_total", "total_only"] {
            for count in 1..=3 {
                for evidence in [
                    serde_json::json!({"event": "Takeout started"}),
                    serde_json::json!({"status": "Takeout in progress"}),
                    serde_json::json!({"event": "Takeout finished"}),
                ] {
                    let mut session = AnnouncementSession::new(settings(true, mode, "living_room"));
                    let mut delivery = RecordingDelivery::default();
                    session.ingest_json(EMPTY.as_bytes(), &mut delivery);
                    for n in 1..=count {
                        ingest_value(&mut session, &mut delivery, &snapshot(&[20, 5, 1][..n]));
                    }
                    let token = session.pending_total_deadline();
                    let mut boundary = if evidence["event"] == "Takeout finished" {
                        snapshot(&[])
                    } else {
                        snapshot(&[20, 5, 1][..count])
                    };
                    boundary["data"]
                        .as_object_mut()
                        .unwrap()
                        .extend(evidence.as_object().unwrap().clone());
                    let total = [20, 25, 26][count - 1];
                    assert_eq!(
                        ingest_value(&mut session, &mut delivery, &boundary),
                        JourneyResult::Total(Some(total))
                    );
                    let mut finish = snapshot(&[]);
                    finish["data"]["event"] = "Takeout finished".into();
                    ingest_value(&mut session, &mut delivery, &finish);
                    ingest_value(&mut session, &mut delivery, &finish);
                    if let Some(token) = token {
                        assert_eq!(
                            session.on_total_deadline(token, &mut delivery),
                            JourneyResult::Suppressed
                        );
                    }
                    assert_eq!(
                        session.diagnostics().readiness,
                        AnnouncementReadiness::Collecting { dart_count: 0 }
                    );
                    session.ingest_json(NEUTRAL.as_bytes(), &mut delivery);
                    ingest_value(&mut session, &mut delivery, &snapshot(&[7]));
                    assert_eq!(
                        session.ingest_json(FULL.as_bytes(), &mut delivery),
                        JourneyResult::Total(Some(7))
                    );
                    let mut expected: Vec<String> = if mode == "each_dart_then_total" {
                        [20, 5, 1][..count]
                            .iter()
                            .map(ToString::to_string)
                            .collect()
                    } else {
                        Vec::new()
                    };
                    expected.push(total.to_string());
                    if mode == "each_dart_then_total" {
                        expected.push("7".into());
                    }
                    expected.push("7".into());
                    assert_eq!(texts(&delivery), expected);
                }
            }
        }
    }

    #[test]
    fn state_boundary_cannot_salvage_invalid_or_stopped_collection() {
        for mode in ["each_dart_then_total", "total_only"] {
            let mut stopped = snapshot(&[]);
            stopped["data"]["running"] = false.into();
            let mut disconnected = snapshot(&[]);
            disconnected["data"]["connected"] = false.into();
            let mut malformed = snapshot(&[20, 5, 1]);
            malformed["data"]["throws"][2]["segment"]["multiplier"] = 0.into();
            for mut boundary in [stopped, disconnected, malformed, snapshot(&[20, 5, 2])] {
                boundary["data"]["event"] = "Takeout finished".into();
                let mut session = AnnouncementSession::new(settings(true, mode, "living_room"));
                let mut delivery = RecordingDelivery::default();
                session.ingest_json(EMPTY.as_bytes(), &mut delivery);
                for n in 1..=3 {
                    ingest_value(&mut session, &mut delivery, &snapshot(&[20, 5, 1][..n]));
                }
                let token = session.pending_total_deadline().unwrap();
                assert_eq!(
                    ingest_value(&mut session, &mut delivery, &boundary),
                    JourneyResult::Invalid
                );
                assert_eq!(session.pending_total_deadline(), None);
                assert_eq!(
                    session.on_total_deadline(token, &mut delivery),
                    JourneyResult::Suppressed
                );
                session.ingest_json(FULL.as_bytes(), &mut delivery);
                assert_eq!(
                    texts(&delivery),
                    if mode == "each_dart_then_total" {
                        vec!["20", "5", "1"]
                    } else {
                        vec![]
                    }
                );
            }
        }
    }

    #[test]
    fn starting_does_not_establish_a_running_baseline() {
        for mode in ["each_dart_then_total", "total_only"] {
            let mut session = AnnouncementSession::new(settings(true, mode, "living_room"));
            let mut delivery = RecordingDelivery::default();
            let mut starting = snapshot(&[]);
            starting["data"]["status"] = "Starting".into();
            ingest_value(&mut session, &mut delivery, &starting);
            session.ingest_json(DART.as_bytes(), &mut delivery);
            session.ingest_json(FULL.as_bytes(), &mut delivery);
            assert!(delivery.requests.is_empty());
            starting["data"]["status"] = "Started".into();
            assert_eq!(
                ingest_value(&mut session, &mut delivery, &starting),
                JourneyResult::Baseline
            );
            session.ingest_json(NEUTRAL.as_bytes(), &mut delivery);
            session.ingest_json(DART.as_bytes(), &mut delivery);
            assert_eq!(
                session.ingest_json(FULL.as_bytes(), &mut delivery),
                JourneyResult::Total(Some(20))
            );
        }
    }

    #[test]
    fn deadline_completion_does_not_consume_motion_episode_and_old_flags_stay_latched() {
        for mode in ["each_dart_then_total", "total_only"] {
            for clear_before_expiry in [false, true] {
                let mut session = AnnouncementSession::new(settings(true, mode, "living_room"));
                let mut delivery = RecordingDelivery::default();
                session.ingest_json(EMPTY.as_bytes(), &mut delivery);
                for n in 1..=3 {
                    ingest_value(&mut session, &mut delivery, &snapshot(&[20, 5, 1][..n]));
                }
                let token = session.pending_total_deadline().unwrap();
                if clear_before_expiry {
                    session.ingest_json(EMPTY.as_bytes(), &mut delivery);
                }
                assert_eq!(
                    session.on_total_deadline(token, &mut delivery),
                    JourneyResult::Total(Some(26))
                );
                if !clear_before_expiry {
                    session.ingest_json(EMPTY.as_bytes(), &mut delivery);
                }
                // No motion rise occurred in the first round: its timer must not latch one.
                ingest_value(&mut session, &mut delivery, &snapshot(&[7]));
                assert_eq!(
                    session.ingest_json(FULL.as_bytes(), &mut delivery),
                    JourneyResult::Total(Some(7))
                );
                session.ingest_json(EMPTY.as_bytes(), &mut delivery);
                ingest_value(&mut session, &mut delivery, &snapshot(&[5]));
                assert_eq!(
                    session.ingest_json(FULL.as_bytes(), &mut delivery),
                    JourneyResult::Suppressed
                );
                session.ingest_json(br#"{"type":"motion_state","data":{}}"#, &mut delivery);
                assert_eq!(
                    session.ingest_json(FULL.as_bytes(), &mut delivery),
                    JourneyResult::Suppressed
                );
                session.ingest_json(NEUTRAL.as_bytes(), &mut delivery);
                assert_eq!(
                    session.ingest_json(FULL.as_bytes(), &mut delivery),
                    JourneyResult::Total(Some(5))
                );
                assert_eq!(
                    session.on_total_deadline(token, &mut delivery),
                    JourneyResult::Suppressed
                );
                assert_eq!(
                    texts(&delivery),
                    if mode == "each_dart_then_total" {
                        vec!["20", "5", "1", "26", "7", "7", "5", "5"]
                    } else {
                        vec!["26", "7", "5"]
                    }
                );
            }
        }
    }

    #[test]
    fn resets_cancel_deadlines_without_reusing_tokens() {
        for mode in ["each_dart_then_total", "total_only"] {
            for reset in 0..5 {
                let mut session = AnnouncementSession::new(settings(true, mode, "living_room"));
                let mut delivery = RecordingDelivery::default();
                session.ingest_json(EMPTY.as_bytes(), &mut delivery);
                for n in 1..=3 {
                    ingest_value(&mut session, &mut delivery, &snapshot(&[20, 5, 1][..n]));
                }
                let old = session.pending_total_deadline().unwrap();
                match reset {
                    0 => session.connection_reset(),
                    1 => session.reconfigure(settings(true, mode, "office")),
                    2 => {
                        session.reconfigure(settings(false, mode, "living_room"));
                        session.reconfigure(settings(true, mode, "living_room"));
                    }
                    3 => {
                        session.ingest_json(b"{bad", &mut delivery);
                    }
                    _ => {
                        let mut stopped = snapshot(&[]);
                        stopped["data"]["running"] = false.into();
                        ingest_value(&mut session, &mut delivery, &stopped);
                    }
                }
                assert_eq!(session.pending_total_deadline(), None);
                assert_eq!(
                    session.on_total_deadline(old, &mut delivery),
                    JourneyResult::Suppressed
                );
                session.ingest_json(EMPTY.as_bytes(), &mut delivery);
                for n in 1..=3 {
                    ingest_value(&mut session, &mut delivery, &snapshot(&[7, 2, 1][..n]));
                }
                let new = session.pending_total_deadline().unwrap();
                assert_ne!(old, new);
                assert_eq!(
                    session.on_total_deadline(old, &mut delivery),
                    JourneyResult::Suppressed
                );
                assert_eq!(
                    session.on_total_deadline(new, &mut delivery),
                    JourneyResult::Total(Some(10))
                );
                assert_eq!(
                    texts(&delivery),
                    if mode == "each_dart_then_total" {
                        vec!["20", "5", "1", "7", "2", "1", "10"]
                    } else {
                        vec!["10"]
                    }
                );
            }
        }
    }

    #[test]
    fn detected_zero_collections_complete_but_empty_takeout_stays_silent() {
        for mode in ["each_dart_then_total", "total_only"] {
            for count in 0..=3 {
                let mut session = AnnouncementSession::new(settings(true, mode, "living_room"));
                let mut delivery = RecordingDelivery::default();
                session.ingest_json(EMPTY.as_bytes(), &mut delivery);
                for n in 1..=count {
                    let mut detected = snapshot(&vec![0; n]);
                    for dart in detected["data"]["throws"].as_array_mut().unwrap() {
                        dart["segment"] = serde_json::json!({
                            "name": "M19", "number": 19, "bed": "Outside", "multiplier": 0
                        });
                    }
                    ingest_value(&mut session, &mut delivery, &detected);
                }
                if let Some(token) = session.pending_total_deadline() {
                    assert_eq!(
                        session.on_total_deadline(token, &mut delivery),
                        JourneyResult::Total(Some(0))
                    );
                } else {
                    assert_eq!(
                        session.ingest_json(FULL.as_bytes(), &mut delivery),
                        if count == 0 {
                            JourneyResult::Suppressed
                        } else {
                            JourneyResult::Total(Some(0))
                        }
                    );
                }
                session.ingest_json(FULL.as_bytes(), &mut delivery);
                let expected_count = usize::from(count > 0)
                    + if mode == "each_dart_then_total" {
                        count
                    } else {
                        0
                    };
                assert_eq!(texts(&delivery), vec!["0"; expected_count]);
            }
        }
    }

    #[test]
    fn boundary_append_is_validated_before_completion_and_jump_cannot_salvage_prefix() {
        for mode in ["each_dart_then_total", "total_only"] {
            for jump in [false, true] {
                let mut session = AnnouncementSession::new(settings(true, mode, "living_room"));
                let mut delivery = RecordingDelivery::default();
                session.ingest_json(EMPTY.as_bytes(), &mut delivery);
                ingest_value(&mut session, &mut delivery, &snapshot(&[20]));
                if !jump {
                    ingest_value(&mut session, &mut delivery, &snapshot(&[20, 5]));
                }
                let mut boundary = snapshot(&[20, 5, 1]);
                boundary["data"]["event"] = "Takeout started".into();
                assert_eq!(
                    ingest_value(&mut session, &mut delivery, &boundary),
                    if jump {
                        JourneyResult::Invalid
                    } else {
                        JourneyResult::Total(Some(26))
                    }
                );
                assert_eq!(session.pending_total_deadline(), None);
                assert_eq!(
                    texts(&delivery),
                    match (mode, jump) {
                        ("each_dart_then_total", false) => vec!["20", "5", "1", "26"],
                        ("each_dart_then_total", true) => vec!["20"],
                        (_, false) => vec!["26"],
                        (_, true) => vec![],
                    }
                );
            }
        }
    }

    #[test]
    fn failed_total_is_consumed_for_both_trigger_orders() {
        for mode in ["each_dart_then_total", "total_only"] {
            for deadline_first in [false, true] {
                let mut session = AnnouncementSession::new(settings(true, mode, "living_room"));
                let mut delivery = RecordingDelivery::default();
                session.ingest_json(EMPTY.as_bytes(), &mut delivery);
                for n in 1..=3 {
                    ingest_value(&mut session, &mut delivery, &snapshot(&[20, 5, 1][..n]));
                }
                let token = session.pending_total_deadline().unwrap();
                let failed = if deadline_first {
                    session.on_total_deadline(token, &mut UnavailableDelivery)
                } else {
                    session.ingest_json(FULL.as_bytes(), &mut UnavailableDelivery)
                };
                assert_eq!(
                    failed,
                    JourneyResult::DeliveryFailed(DeliveryError::Unavailable)
                );
                session.ingest_json(NEUTRAL.as_bytes(), &mut delivery);
                assert_eq!(
                    session.ingest_json(FULL.as_bytes(), &mut delivery),
                    JourneyResult::Suppressed
                );
                assert_eq!(
                    session.on_total_deadline(token, &mut delivery),
                    JourneyResult::Suppressed
                );
                assert_eq!(
                    texts(&delivery),
                    if mode == "each_dart_then_total" {
                        vec!["20", "5", "1"]
                    } else {
                        vec![]
                    }
                );
            }
        }
    }
}
