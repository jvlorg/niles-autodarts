use autodarts_core::local_score::JourneyResult;
use autodarts_core::{
    AnnouncementMode, AnnouncementSession, AnnouncementSettings, RecordingDelivery,
};

fn session(mode: AnnouncementMode) -> AnnouncementSession {
    let name = match mode {
        AnnouncementMode::EachDartThenTotal => "each_dart_then_total",
        AnnouncementMode::TotalOnly => "total_only",
    };
    AnnouncementSession::new(
        AnnouncementSettings::new(true, Some(name), "living_room").expect("valid replay settings"),
    )
}

fn replay_fixture(mode: AnnouncementMode) -> RecordingDelivery {
    let mut session = session(mode);
    let mut delivery = RecordingDelivery::default();
    let capture: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../tests/fixtures/board-manager-jvl14-three-round-reduced-capture.json"
    ))
    .expect("valid JVL-14 fixture projection");
    for envelope in capture.as_array().expect("capture array") {
        let json = serde_json::to_vec(envelope).expect("capture envelope");
        session.ingest_json(&json, &mut delivery);
    }
    delivery
}

fn replay_synthetic_partial() -> RecordingDelivery {
    let mut session = session(AnnouncementMode::TotalOnly);
    let mut delivery = RecordingDelivery::default();
    let scenarios: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../tests/fixtures/board-manager-jvl14-synthetic-scenarios.json"
    ))
    .expect("valid synthetic scenarios");
    for envelope in scenarios["partial_round_takeout"].as_array().unwrap() {
        let json = serde_json::to_vec(envelope).unwrap();
        session.ingest_json(&json, &mut delivery);
    }
    delivery
}

fn replay_deadline_first() -> RecordingDelivery {
    let mut session = session(AnnouncementMode::TotalOnly);
    let mut delivery = RecordingDelivery::default();
    let scenarios: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../tests/fixtures/board-manager-jvl14-synthetic-scenarios.json"
    ))
    .expect("valid synthetic scenarios");
    for envelope in scenarios["deadline_timer_first"].as_array().unwrap() {
        let json = serde_json::to_vec(envelope).unwrap();
        session.ingest_json(&json, &mut delivery);
    }
    let token = session
        .pending_total_deadline()
        .expect("synthetic deadline");
    assert_eq!(
        session.on_total_deadline(token, &mut delivery),
        JourneyResult::Total(Some(26))
    );
    delivery
}

fn texts(delivery: &RecordingDelivery) -> Vec<&str> {
    delivery
        .requests
        .iter()
        .map(|request| request.text.as_str())
        .collect()
}

fn main() {
    let each = replay_fixture(AnnouncementMode::EachDartThenTotal);
    assert_eq!(
        texts(&each),
        [
            "60", "20", "7", "87", "40", "20", "0", "60", "12", "18", "0", "30"
        ]
    );

    let totals = replay_fixture(AnnouncementMode::TotalOnly);
    assert_eq!(texts(&totals), ["87", "60", "30"]);

    let partial = replay_synthetic_partial();
    assert_eq!(texts(&partial), ["25"]);

    let timer = replay_deadline_first();
    assert_eq!(texts(&timer), ["26"]);

    println!("JVL-14 fixture projection: 87, 60, 30 totals in both modes");
    println!("Synthetic partial and deadline-first scenarios remain separately labelled");
    println!("These are policy submissions; they do not prove host transport or audibility.");
}
