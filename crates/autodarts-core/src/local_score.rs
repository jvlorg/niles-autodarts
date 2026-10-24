use std::collections::BTreeSet;

use serde_json::Value;

use crate::{AnnouncementDelivery, AnnouncementMode, AnnouncementRequest, DeliveryError};

const MAX_DARTS: usize = 3;
const MAX_ENVELOPE_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalState {
    pub running: Option<bool>,
    pub num_throws: Option<u32>,
    pub throws: Option<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LocalEvent {
    State(LocalState),
    Motion {
        takeout_partial: Option<bool>,
        takeout_full: Option<bool>,
    },
    Unknown,
    Invalid,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JourneyResult {
    Baseline,
    Dart(Option<u8>),
    Total(Option<u16>),
    Suppressed,
    Invalid,
    DeliveryFailed(DeliveryError),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnnouncementReadiness {
    Disabled,
    AwaitingBaseline,
    Collecting { dart_count: usize },
    AwaitingRemoval,
    AwaitingClear,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnnouncementDisposition {
    Ignored,
    Suppressed,
    Invalid,
    Processed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnnouncementReason {
    UnrelatedTopic,
    InactiveMotion,
    InvalidInput,
    BaselineEstablished,
    DartAccepted,
    TotalAccepted,
    StateSuppressed,
    NoEligibleCollection,
    DeliveryFailure,
    Disabled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnnouncementEvent {
    pub disposition: AnnouncementDisposition,
    pub reason: AnnouncementReason,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnnouncementDiagnostics {
    pub readiness: AnnouncementReadiness,
    pub last_event: Option<AnnouncementEvent>,
}

pub fn parse_json(input: &[u8]) -> Result<LocalEvent, &'static str> {
    if input.len() > MAX_ENVELOPE_BYTES {
        return Err("event envelope too large");
    }
    let value: Value = serde_json::from_slice(input).map_err(|_| "invalid JSON")?;
    parse_envelope(&value)
}

pub fn parse_state(value: &Value) -> Result<LocalState, &'static str> {
    if !value.is_object() {
        return Err("state must be an object");
    }
    let running = optional_bool(value, "running")?;
    let num_throws = match value.get("numThrows") {
        Some(Value::Number(n)) => {
            let count = n
                .as_u64()
                .and_then(|n| u32::try_from(n).ok())
                .ok_or("invalid throw count")?;
            if count as usize > MAX_DARTS {
                return Err("too many darts");
            }
            Some(count)
        }
        None | Some(Value::Null) => return Err("missing throw count"),
        _ => return Err("invalid throw count"),
    };
    let throws = match value.get("throws") {
        None if num_throws == Some(0) => Some(Vec::new()),
        None => return Err("missing throws collection"),
        Some(Value::Array(items)) if items.len() <= MAX_DARTS => {
            let mut scores = Vec::with_capacity(items.len());
            for item in items {
                let segment = item.get("segment").ok_or("missing segment")?;
                let number = segment
                    .get("number")
                    .and_then(Value::as_u64)
                    .ok_or("invalid segment number")?;
                let multiplier = segment
                    .get("multiplier")
                    .and_then(Value::as_u64)
                    .ok_or("invalid multiplier")?;
                if !matches!(number, 0..=20 | 25)
                    || !matches!(multiplier, 1..=3)
                    || (number == 25 && multiplier == 3)
                {
                    return Err("out-of-range dart");
                }
                scores.push(u8::try_from(number * multiplier).map_err(|_| "out-of-range dart")?);
            }
            Some(scores)
        }
        Some(Value::Array(_)) => return Err("too many darts"),
        Some(Value::Null) => return Err("invalid throws collection"),
        Some(_) => return Err("invalid throws collection"),
    };
    if usize::try_from(num_throws.expect("validated throw count")).ok()
        != Some(throws.as_ref().expect("validated throws collection").len())
    {
        return Err("throw count mismatch");
    }
    Ok(LocalState {
        running,
        num_throws,
        throws,
    })
}

fn optional_bool(value: &Value, key: &str) -> Result<Option<bool>, &'static str> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(v)) => Ok(Some(*v)),
        _ => Err("invalid boolean state"),
    }
}

pub fn parse_envelope(value: &Value) -> Result<LocalEvent, &'static str> {
    if !value.is_object() {
        return Err("event envelope must be an object");
    }
    let kind = value
        .get("type")
        .and_then(Value::as_str)
        .ok_or("missing event type")?;
    if !matches!(kind, "state" | "motion_state") {
        return Ok(LocalEvent::Unknown);
    }
    let data = value.get("data").ok_or("missing event data")?;
    match kind {
        "state" => parse_state(data).map(LocalEvent::State),
        "motion_state" => {
            if !data.is_object() {
                return Err("motion data must be an object");
            }
            Ok(LocalEvent::Motion {
                takeout_partial: optional_bool(data, "isTakeoutPartial")?,
                takeout_full: optional_bool(data, "isTakeoutFull")?,
            })
        }
        _ => unreachable!("known event type checked above"),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    AwaitingBaseline,
    Collecting,
    AwaitingClear,
}

#[derive(Default)]
pub struct LocalJourney {
    epoch: u64,
    serial: u64,
    phase: Option<Phase>,
    previous: Option<LocalState>,
    darts: Vec<u8>,
    announced: usize,
    tainted: bool,
    removal_seen: bool,
    removal_candidate: Option<Vec<u8>>,
    in_flight: BTreeSet<String>,
    settings: Option<(AnnouncementMode, String)>,
    last_event: Option<AnnouncementEvent>,
}

impl LocalJourney {
    pub fn reconnect(&mut self) {
        self.epoch = self.epoch.saturating_add(1);
        self.reset();
    }
    fn reset(&mut self) {
        self.in_flight.clear();
        self.serial = self.serial.saturating_add(1);
        self.phase = Some(Phase::AwaitingBaseline);
        self.previous = None;
        self.darts.clear();
        self.announced = 0;
        self.tainted = false;
        self.removal_seen = false;
        self.removal_candidate = None;
        self.last_event = None;
    }
    pub fn diagnostics(&self) -> AnnouncementDiagnostics {
        let readiness = if self.settings.is_none() {
            AnnouncementReadiness::Disabled
        } else {
            match self.phase {
                Some(Phase::Collecting) => AnnouncementReadiness::Collecting {
                    dart_count: self.darts.len(),
                },
                Some(Phase::AwaitingClear) if self.removal_candidate.is_some() => {
                    AnnouncementReadiness::AwaitingRemoval
                }
                Some(Phase::AwaitingClear) => AnnouncementReadiness::AwaitingClear,
                Some(Phase::AwaitingBaseline) | None => AnnouncementReadiness::AwaitingBaseline,
            }
        };
        AnnouncementDiagnostics {
            readiness,
            last_event: self.last_event,
        }
    }
    pub fn reject_input(&mut self) -> JourneyResult {
        self.tainted = true;
        self.previous = None;
        self.removal_candidate = None;
        self.phase = Some(Phase::AwaitingBaseline);
        JourneyResult::Invalid
    }
    pub fn ingest<D: AnnouncementDelivery>(
        &mut self,
        enabled: bool,
        mode: AnnouncementMode,
        room: &str,
        event: LocalEvent,
        delivery: &mut D,
    ) -> JourneyResult {
        let current_settings = (mode, room.to_owned());
        if self
            .settings
            .as_ref()
            .is_some_and(|old| old != &current_settings)
        {
            self.reset();
        }
        self.settings = Some(current_settings);
        let initial_reason = match &event {
            LocalEvent::Unknown => AnnouncementReason::UnrelatedTopic,
            LocalEvent::Invalid => AnnouncementReason::InvalidInput,
            LocalEvent::Motion {
                takeout_partial: Some(true),
                ..
            }
            | LocalEvent::Motion {
                takeout_full: Some(true),
                ..
            } => AnnouncementReason::NoEligibleCollection,
            LocalEvent::Motion { .. } => AnnouncementReason::InactiveMotion,
            LocalEvent::State(_) => AnnouncementReason::StateSuppressed,
        };
        if !enabled {
            self.settings = None;
            self.reset();
            self.last_event = Some(AnnouncementEvent {
                disposition: AnnouncementDisposition::Suppressed,
                reason: AnnouncementReason::Disabled,
            });
            return JourneyResult::Suppressed;
        }
        let result = match event {
            LocalEvent::Invalid => self.reject_input(),
            LocalEvent::Unknown => JourneyResult::Suppressed,
            LocalEvent::Motion {
                takeout_partial: Some(true),
                ..
            }
            | LocalEvent::Motion {
                takeout_full: Some(true),
                ..
            } => self.removal(mode, room, delivery),
            LocalEvent::Motion { .. } => JourneyResult::Suppressed,
            LocalEvent::State(state) => self.state(state, mode, room, delivery),
        };
        let (reason, disposition) = match result {
            JourneyResult::Baseline
                if matches!(
                    self.diagnostics().readiness,
                    AnnouncementReadiness::Collecting { dart_count: 0 }
                ) =>
            {
                (
                    AnnouncementReason::BaselineEstablished,
                    AnnouncementDisposition::Processed,
                )
            }
            JourneyResult::Baseline => (
                AnnouncementReason::StateSuppressed,
                AnnouncementDisposition::Suppressed,
            ),
            JourneyResult::Dart(_) => (
                AnnouncementReason::DartAccepted,
                AnnouncementDisposition::Processed,
            ),
            JourneyResult::Total(_) => (
                AnnouncementReason::TotalAccepted,
                AnnouncementDisposition::Processed,
            ),
            JourneyResult::Invalid => (
                AnnouncementReason::InvalidInput,
                AnnouncementDisposition::Invalid,
            ),
            JourneyResult::DeliveryFailed(_) => (
                AnnouncementReason::DeliveryFailure,
                AnnouncementDisposition::Processed,
            ),
            JourneyResult::Suppressed if initial_reason == AnnouncementReason::UnrelatedTopic => (
                AnnouncementReason::UnrelatedTopic,
                AnnouncementDisposition::Ignored,
            ),
            JourneyResult::Suppressed => (initial_reason, AnnouncementDisposition::Suppressed),
        };
        self.last_event = Some(AnnouncementEvent {
            disposition,
            reason,
        });
        result
    }

    fn state<D: AnnouncementDelivery>(
        &mut self,
        state: LocalState,
        mode: AnnouncementMode,
        room: &str,
        delivery: &mut D,
    ) -> JourneyResult {
        if state.running == Some(false) {
            self.reset();
            return JourneyResult::Suppressed;
        }
        if mode == AnnouncementMode::TotalOnly
            && self.phase == Some(Phase::Collecting)
            && !self.darts.is_empty()
            && state.throws.as_ref().is_some_and(Vec::is_empty)
            && state.num_throws == Some(0)
        {
            self.previous = Some(state);
            self.removal_candidate = Some(self.darts.clone());
            self.phase = Some(Phase::AwaitingClear);
            self.darts.clear();
            return JourneyResult::Suppressed;
        }
        if self.phase == Some(Phase::AwaitingClear)
            && state.throws.as_ref().is_some_and(Vec::is_empty)
            && state.num_throws == Some(0)
        {
            if self.removal_candidate.is_some() && !self.removal_seen {
                self.previous = Some(state);
                return JourneyResult::Suppressed;
            }
            self.previous = Some(state.clone());
            self.phase = Some(Phase::Collecting);
            self.darts.clear();
            self.announced = 0;
            self.tainted = false;
            self.removal_seen = false;
            self.removal_candidate = None;
            self.serial = self.serial.saturating_add(1);
            self.in_flight.clear();
            return JourneyResult::Baseline;
        }
        if self.phase != Some(Phase::Collecting) {
            if self.phase == Some(Phase::AwaitingClear) {
                let old = self.previous.as_ref().and_then(|s| s.throws.as_ref());
                if let (Some(old), Some(new)) = (old, state.throws.as_ref())
                    && new.len() < old.len()
                {
                    self.removal_candidate = None;
                    self.tainted = true;
                    self.previous = Some(state);
                    return JourneyResult::Suppressed;
                }
            }
            self.previous = Some(state.clone());
            if state.throws.as_ref().is_some_and(Vec::is_empty) && state.num_throws == Some(0) {
                self.serial = self.serial.saturating_add(1);
                self.in_flight.clear();
                self.phase = Some(Phase::Collecting);
                self.darts.clear();
                self.announced = 0;
                self.tainted = false;
                self.removal_seen = false;
                self.removal_candidate = None;
                return JourneyResult::Baseline;
            }
            self.phase = Some(Phase::AwaitingBaseline);
            self.tainted = true;
            return JourneyResult::Baseline;
        }
        if state.throws.is_none() {
            self.reject_input();
            return JourneyResult::Suppressed;
        }
        let Some(previous) = self.previous.clone() else {
            self.reject_input();
            return JourneyResult::Suppressed;
        };
        let old = previous.throws.unwrap_or_default();
        let new = state.throws.clone().unwrap_or_default();
        if self.removal_seen && new.is_empty() {
            self.previous = Some(state);
            self.phase = Some(Phase::AwaitingClear);
            return JourneyResult::Suppressed;
        }
        if self.removal_seen {
            self.tainted = true;
            self.previous = Some(state);
            self.removal_candidate = None;
            return JourneyResult::Invalid;
        }
        if new == old {
            self.previous = Some(state);
            return JourneyResult::Suppressed;
        }
        if new.len() < old.len() {
            let eligible_snapshot = !self.tainted
                && !old.is_empty()
                && (new.is_empty() || (self.darts.len() == old.len() && new == self.darts[1..]))
                && (mode == AnnouncementMode::TotalOnly || self.announced == self.darts.len());
            if eligible_snapshot {
                self.removal_candidate = Some(self.darts.clone());
                self.phase = Some(Phase::AwaitingClear);
                self.darts.clear();
                self.announced = 0;
            } else {
                self.removal_candidate = None;
                self.tainted = true;
            }
            self.previous = Some(state);
            if new.is_empty() && self.removal_candidate.is_none() {
                self.phase = Some(Phase::AwaitingBaseline);
            }
            return JourneyResult::Suppressed;
        }
        if new.len() != old.len() + 1
            || !new.starts_with(&old)
            || new.len() > MAX_DARTS
            || self.tainted
        {
            self.tainted = true;
            self.removal_candidate = None;
            self.previous = Some(state);
            return JourneyResult::Invalid;
        }
        self.previous = Some(state);
        self.darts = new.clone();
        self.removal_candidate = None;
        if mode == AnnouncementMode::TotalOnly {
            return JourneyResult::Dart(None);
        }
        let score = *new.last().expect("appended dart");
        let key = self.key("dart", new.len());
        if !self.in_flight.insert(key.clone()) {
            return JourneyResult::Suppressed;
        }
        match deliver(delivery, room, &score.to_string(), key) {
            Ok(()) => {
                self.announced = new.len();
                JourneyResult::Dart(Some(score))
            }
            Err(e) => {
                self.tainted = true;
                JourneyResult::DeliveryFailed(e)
            }
        }
    }
    fn removal<D: AnnouncementDelivery>(
        &mut self,
        mode: AnnouncementMode,
        room: &str,
        delivery: &mut D,
    ) -> JourneyResult {
        if self.removal_seen {
            return JourneyResult::Suppressed;
        }
        let retained_candidate = self.removal_candidate.is_some();
        let candidate = self
            .removal_candidate
            .take()
            .or_else(|| (!self.darts.is_empty()).then(|| self.darts.clone()));
        let Some(candidate) = candidate else {
            return JourneyResult::Suppressed;
        };
        let candidate_len = candidate.len();
        if self.tainted
            || candidate.is_empty()
            || candidate.len() > MAX_DARTS
            || (mode == AnnouncementMode::EachDartThenTotal
                && !retained_candidate
                && self.announced != candidate.len())
        {
            return JourneyResult::Suppressed;
        }
        let clear_before_marker = self
            .previous
            .as_ref()
            .and_then(|state| state.throws.as_ref())
            .is_some_and(Vec::is_empty);
        self.darts = candidate.clone();
        let total: u16 = candidate.iter().map(|v| u16::from(*v)).sum();
        let key = self.key("total", candidate_len);
        if !self.in_flight.insert(key.clone()) {
            return JourneyResult::Suppressed;
        }
        self.removal_seen = true;
        match deliver(delivery, room, &total.to_string(), key) {
            Ok(()) => {
                self.serial = self.serial.saturating_add(1);
                if clear_before_marker {
                    self.phase = Some(Phase::Collecting);
                    self.darts.clear();
                    self.announced = 0;
                    self.tainted = false;
                    self.removal_seen = false;
                    self.removal_candidate = None;
                    self.in_flight.clear();
                } else {
                    self.phase = Some(Phase::AwaitingClear);
                }
                JourneyResult::Total(Some(total))
            }
            Err(e) => {
                self.phase = Some(Phase::AwaitingClear);
                JourneyResult::DeliveryFailed(e)
            }
        }
    }
    fn key(&self, kind: &str, n: usize) -> String {
        format!(
            "epoch:{}:collection:{}:{}:{}",
            self.epoch, self.serial, kind, n
        )
    }
}

fn deliver<D: AnnouncementDelivery>(
    delivery: &mut D,
    room: &str,
    text: &str,
    key: String,
) -> Result<(), DeliveryError> {
    delivery.deliver(&AnnouncementRequest {
        text: text.to_owned(),
        room: room.to_owned(),
        deduplication_key: key,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RecordingDelivery, UnavailableDelivery};
    use serde_json::json;
    const ROOM: &str = "living_room";
    fn audience() -> String {
        ROOM.to_owned()
    }
    fn state(scores: &[u8]) -> LocalEvent {
        let throws: Vec<_> = scores
            .iter()
            .map(|s| {
                let (number, multiplier) = if *s == 0 {
                    (0, 1)
                } else if *s == 50 {
                    (25, 2)
                } else if *s % 3 == 0 && *s <= 60 {
                    (*s / 3, 3)
                } else if *s % 2 == 0 && *s <= 40 {
                    (*s / 2, 2)
                } else {
                    (*s, 1)
                };
                json!({"segment":{"number":number,"multiplier":multiplier}})
            })
            .collect();
        LocalEvent::State(
            parse_state(&json!({"running":true,"numThrows":scores.len(),"throws":throws})).unwrap(),
        )
    }
    fn ingest(j: &mut LocalJourney, e: LocalEvent, d: &mut RecordingDelivery) -> JourneyResult {
        j.ingest(true, AnnouncementMode::EachDartThenTotal, ROOM, e, d)
    }
    fn empty() -> LocalEvent {
        state(&[])
    }
    fn partial() -> LocalEvent {
        LocalEvent::Motion {
            takeout_partial: Some(true),
            takeout_full: None,
        }
    }
    fn full() -> LocalEvent {
        LocalEvent::Motion {
            takeout_partial: Some(false),
            takeout_full: Some(true),
        }
    }

    #[test]
    fn three_same_score_collections_and_incremental_speech() {
        let mut j = LocalJourney::default();
        let mut d = RecordingDelivery::default();
        for _ in 0..3 {
            ingest(&mut j, empty(), &mut d);
            ingest(&mut j, state(&[20]), &mut d);
            assert_eq!(
                ingest(&mut j, partial(), &mut d),
                JourneyResult::Total(Some(20))
            );
            assert_eq!(ingest(&mut j, full(), &mut d), JourneyResult::Suppressed);
        }
        assert_eq!(
            d.requests
                .iter()
                .map(|r| r.text.as_str())
                .collect::<Vec<_>>(),
            ["20", "20", "20", "20", "20", "20"]
        );
        assert_eq!(
            d.requests
                .iter()
                .map(|r| r.deduplication_key.as_str())
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            6
        );
    }

    #[test]
    fn each_dart_then_total_and_total_only() {
        for mode in [
            AnnouncementMode::EachDartThenTotal,
            AnnouncementMode::TotalOnly,
        ] {
            let mut j = LocalJourney::default();
            let mut d = RecordingDelivery::default();
            j.ingest(true, mode, &audience(), empty(), &mut d);
            for n in 1..=3 {
                j.ingest(true, mode, &audience(), state(&[20, 5, 1][..n]), &mut d);
            }
            assert_eq!(
                j.ingest(true, mode, &audience(), partial(), &mut d),
                JourneyResult::Total(Some(26))
            );
            assert_eq!(
                d.requests
                    .iter()
                    .map(|r| r.text.as_str())
                    .collect::<Vec<_>>(),
                if mode == AnnouncementMode::EachDartThenTotal {
                    vec!["20", "5", "1", "26"]
                } else {
                    vec!["26"]
                }
            );
        }
    }

    #[test]
    fn clear_then_marker_uses_cached_snapshot_once_and_gates_next_collection() {
        let mut j = LocalJourney::default();
        let mut d = RecordingDelivery::default();
        ingest(&mut j, empty(), &mut d);
        ingest(&mut j, state(&[20]), &mut d);
        ingest(&mut j, state(&[20, 5]), &mut d);
        ingest(&mut j, state(&[5]), &mut d);
        assert_eq!(
            ingest(&mut j, partial(), &mut d),
            JourneyResult::Total(Some(25))
        );
        assert_eq!(ingest(&mut j, full(), &mut d), JourneyResult::Suppressed);
        ingest(&mut j, state(&[]), &mut d);
        ingest(&mut j, state(&[]), &mut d);
        ingest(&mut j, state(&[20]), &mut d);
        assert!(d.requests.iter().any(|r| r.text == "20"));
    }

    #[test]
    fn marker_before_clear_and_repeated_markers() {
        let mut j = LocalJourney::default();
        let mut d = RecordingDelivery::default();
        ingest(&mut j, empty(), &mut d);
        ingest(&mut j, state(&[20]), &mut d);
        assert_eq!(
            ingest(&mut j, partial(), &mut d),
            JourneyResult::Total(Some(20))
        );
        ingest(&mut j, state(&[]), &mut d);
        assert_eq!(ingest(&mut j, full(), &mut d), JourneyResult::Suppressed);
    }
    #[test]
    fn clear_before_marker_starts_next_collection_without_second_clear() {
        for mode in [
            AnnouncementMode::EachDartThenTotal,
            AnnouncementMode::TotalOnly,
        ] {
            let mut j = LocalJourney::default();
            let mut d = RecordingDelivery::default();
            j.ingest(true, mode, ROOM, empty(), &mut d);
            j.ingest(true, mode, ROOM, state(&[20]), &mut d);
            j.ingest(true, mode, ROOM, empty(), &mut d);
            assert_eq!(
                j.diagnostics().readiness,
                AnnouncementReadiness::AwaitingRemoval
            );
            assert_eq!(
                j.ingest(true, mode, ROOM, partial(), &mut d),
                JourneyResult::Total(Some(20))
            );
            assert_eq!(
                j.diagnostics().readiness,
                AnnouncementReadiness::Collecting { dart_count: 0 }
            );
            j.ingest(true, mode, ROOM, state(&[7]), &mut d);
            assert_eq!(
                j.ingest(true, mode, ROOM, full(), &mut d),
                JourneyResult::Total(Some(7))
            );
            assert_eq!(
                d.requests
                    .iter()
                    .map(|request| request.text.as_str())
                    .collect::<Vec<_>>(),
                if mode == AnnouncementMode::EachDartThenTotal {
                    vec!["20", "20", "7", "7"]
                } else {
                    vec!["20", "7"]
                }
            );
        }
    }

    #[test]
    fn clear_waits_for_explicit_removal_before_announcing_total() {
        let mut j = LocalJourney::default();
        let mut d = RecordingDelivery::default();
        ingest(&mut j, empty(), &mut d);
        ingest(&mut j, state(&[20]), &mut d);
        ingest(&mut j, state(&[]), &mut d);
        assert_eq!(d.requests.len(), 1);
        assert_eq!(
            ingest(&mut j, full(), &mut d),
            JourneyResult::Total(Some(20))
        );
        assert_eq!(d.requests.len(), 2);
        assert_eq!(ingest(&mut j, full(), &mut d), JourneyResult::Suppressed);
    }

    #[test]
    fn invalid_stopped_disconnect_disable_settings_clear_candidates() {
        let mut j = LocalJourney::default();
        let mut d = RecordingDelivery::default();
        ingest(&mut j, empty(), &mut d);
        ingest(&mut j, state(&[20]), &mut d);
        ingest(&mut j, state(&[]), &mut d);
        j.reject_input();
        assert_eq!(ingest(&mut j, partial(), &mut d), JourneyResult::Suppressed);
        j.reconnect();
        j.ingest(
            false,
            AnnouncementMode::EachDartThenTotal,
            &audience(),
            empty(),
            &mut d,
        );
        j.ingest(
            true,
            AnnouncementMode::EachDartThenTotal,
            &audience(),
            state(&[20]),
            &mut d,
        );
        assert_eq!(
            j.ingest(
                true,
                AnnouncementMode::EachDartThenTotal,
                &audience(),
                partial(),
                &mut d
            ),
            JourneyResult::Suppressed
        );
        let mut bad = UnavailableDelivery;
        j.ingest(
            true,
            AnnouncementMode::TotalOnly,
            &audience(),
            state(&[20]),
            &mut bad,
        );
        j.ingest(
            true,
            AnnouncementMode::TotalOnly,
            &audience(),
            LocalEvent::Invalid,
            &mut bad,
        );
        assert_eq!(
            j.ingest(
                true,
                AnnouncementMode::TotalOnly,
                &audience(),
                full(),
                &mut bad
            ),
            JourneyResult::Suppressed
        );
    }

    #[test]
    fn bull_and_miss_fixture_encodings_are_synthetic_and_bounded() {
        assert!(parse_state(&json!({"numThrows":2,"throws":[{"segment":{"number":25,"multiplier":1}},{"segment":{"number":25,"multiplier":2}}]})).is_ok());
        assert!(
            parse_state(&json!({"numThrows":1,"throws":[{"segment":{"number":0,"multiplier":1}}]}))
                .is_ok()
        );
        assert!(
            parse_state(&json!({"numThrows":1,"throws":[{"segment":{"number":0,"multiplier":0}}]}))
                .is_err()
        );
        assert!(
            parse_state(
                &json!({"numThrows":1,"throws":[{"segment":{"number":21,"multiplier":1}}]})
            )
            .is_err()
        );
    }
    #[test]
    fn omitted_zero_count_is_a_valid_clean_empty_state() {
        let state = parse_state(&json!({"running": true, "numThrows": 0})).unwrap();
        assert_eq!(state.num_throws, Some(0));
        assert_eq!(state.throws, Some(Vec::new()));
    }

    #[test]
    fn parser_rejects_segments_21_through_24_and_ignores_unrelated_topic() {
        for number in 21..=24 {
            assert!(
                parse_state(
                    &json!({"numThrows":1,"throws":[{"segment":{"number":number,"multiplier":1}}]})
                )
                .is_err()
            );
        }
        assert_eq!(
            parse_json(br#"{"type":"camera_status","data":{"anything":true}}"#).unwrap(),
            LocalEvent::Unknown
        );
    }

    #[test]
    fn total_only_clear_before_marker_and_three_collections() {
        let mut j = LocalJourney::default();
        let mut d = RecordingDelivery::default();
        for completed in 0..3 {
            j.ingest(
                true,
                AnnouncementMode::TotalOnly,
                &audience(),
                empty(),
                &mut d,
            );
            j.ingest(
                true,
                AnnouncementMode::TotalOnly,
                &audience(),
                state(&[20]),
                &mut d,
            );
            assert_eq!(d.requests.len(), completed);
            assert_eq!(
                j.ingest(
                    true,
                    AnnouncementMode::TotalOnly,
                    &audience(),
                    empty(),
                    &mut d
                ),
                JourneyResult::Suppressed
            );
            assert_eq!(
                j.ingest(
                    true,
                    AnnouncementMode::TotalOnly,
                    &audience(),
                    full(),
                    &mut d
                ),
                JourneyResult::Total(Some(20))
            );
        }
        assert_eq!(
            d.requests
                .iter()
                .map(|r| r.text.as_str())
                .collect::<Vec<_>>(),
            ["20", "20", "20"]
        );
    }

    #[test]
    fn attempt_history_is_bounded_by_collection_baselines() {
        let mut j = LocalJourney::default();
        let mut unavailable = UnavailableDelivery;
        for _ in 0..300 {
            j.ingest(
                true,
                AnnouncementMode::TotalOnly,
                &audience(),
                empty(),
                &mut unavailable,
            );
            j.ingest(
                true,
                AnnouncementMode::TotalOnly,
                &audience(),
                state(&[20]),
                &mut unavailable,
            );
            assert_eq!(
                j.ingest(
                    true,
                    AnnouncementMode::TotalOnly,
                    &audience(),
                    partial(),
                    &mut unavailable
                ),
                JourneyResult::DeliveryFailed(DeliveryError::Unavailable)
            );
            assert_eq!(
                j.ingest(
                    true,
                    AnnouncementMode::TotalOnly,
                    &audience(),
                    full(),
                    &mut unavailable
                ),
                JourneyResult::Suppressed
            );
        }
        assert!(j.in_flight.len() <= 1);
    }
    #[test]
    fn diagnostics_distinguish_readiness_and_event_disposition() {
        let mut j = LocalJourney::default();
        let mut d = RecordingDelivery::default();
        assert_eq!(j.diagnostics().readiness, AnnouncementReadiness::Disabled);
        j.ingest(
            true,
            AnnouncementMode::EachDartThenTotal,
            ROOM,
            empty(),
            &mut d,
        );
        assert_eq!(
            j.diagnostics().readiness,
            AnnouncementReadiness::Collecting { dart_count: 0 }
        );
        assert_eq!(
            j.diagnostics().last_event,
            Some(AnnouncementEvent {
                disposition: AnnouncementDisposition::Processed,
                reason: AnnouncementReason::BaselineEstablished,
            })
        );
        j.ingest(
            true,
            AnnouncementMode::EachDartThenTotal,
            ROOM,
            LocalEvent::Unknown,
            &mut d,
        );
        assert_eq!(
            j.diagnostics().last_event,
            Some(AnnouncementEvent {
                disposition: AnnouncementDisposition::Ignored,
                reason: AnnouncementReason::UnrelatedTopic,
            })
        );
    }
    #[test]
    fn startup_nonempty_state_is_suppressed_until_clean_baseline() {
        let mut j = LocalJourney::default();
        let mut d = RecordingDelivery::default();
        assert_eq!(
            j.ingest(
                true,
                AnnouncementMode::EachDartThenTotal,
                ROOM,
                state(&[3, 7, 7]),
                &mut d,
            ),
            JourneyResult::Baseline
        );
        assert_eq!(
            j.diagnostics(),
            AnnouncementDiagnostics {
                readiness: AnnouncementReadiness::AwaitingBaseline,
                last_event: Some(AnnouncementEvent {
                    disposition: AnnouncementDisposition::Suppressed,
                    reason: AnnouncementReason::StateSuppressed,
                }),
            }
        );
        assert!(d.requests.is_empty());
    }

    #[test]
    fn parser_rejects_unknown_counts_and_contradictory_empty_shapes() {
        for value in [
            json!({"numThrows": null, "throws": []}),
            json!({"numThrows": 1}),
            json!({"numThrows": 0, "throws": [{"segment": {"number": 3, "multiplier": 1}}]}),
            json!({"numThrows": 0, "throws": null}),
            json!({"numThrows": 1, "throws": null}),
            json!({"numThrows": 4, "throws": []}),
        ] {
            assert!(
                parse_state(&value).is_err(),
                "accepted invalid state: {value}"
            );
        }
        assert_eq!(
            parse_json(br#"{"type":"camera_status","data":{"anything":true}}"#).unwrap(),
            LocalEvent::Unknown
        );
        assert_eq!(
            parse_json(br#"{"type":"camera_status","data":{"numThrows":"bad"}}"#).unwrap(),
            LocalEvent::Unknown
        );
        assert_eq!(
            parse_json(br#"{"type":"camera_status"}"#).unwrap(),
            LocalEvent::Unknown
        );
    }
}
