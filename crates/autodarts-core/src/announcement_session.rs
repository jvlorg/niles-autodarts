use crate::local_score::{JourneyResult, LocalEvent, LocalJourney, parse_json};
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
        let event = match parse_json(json) {
            Ok(event) => event,
            Err(_) => LocalEvent::Invalid,
        };
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
}
