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
    .expect("valid exact JVL-14 reduced capture");
    for envelope in capture.as_array().expect("capture array") {
        let json = serde_json::to_vec(envelope).expect("capture envelope");
        session.ingest_json(&json, &mut delivery);
    }
    delivery
}

fn replay_synthetic_partial(mode: AnnouncementMode) -> RecordingDelivery {
    let mut session = session(mode);
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

fn replay_deadline_first(mode: AnnouncementMode) -> RecordingDelivery {
    let mut session = session(mode);
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

fn replay_capture_with_synthetic_continuation(mode: AnnouncementMode) -> RecordingDelivery {
    let mut session = session(mode);
    let mut delivery = RecordingDelivery::default();
    let capture: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../tests/fixtures/board-manager-1.0.7-reduced-capture.json"
    ))
    .expect("valid reduced capture");
    for envelope in capture.as_array().expect("capture array") {
        session.ingest_json(&serde_json::to_vec(envelope).unwrap(), &mut delivery);
        assert!(delivery.requests.is_empty());
    }
    assert_eq!(
        session.diagnostics().readiness,
        autodarts_core::AnnouncementReadiness::Collecting { dart_count: 0 }
    );
    let continuation: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../tests/fixtures/board-manager-1.0.7-synthetic-continuation.json"
    ))
    .expect("valid synthetic continuation");
    for (index, envelope) in continuation
        .as_array()
        .expect("continuation array")
        .iter()
        .enumerate()
    {
        // Separate synthetic motion schedule; do not rewrite the maintained fixture.
        if matches!(index, 0 | 7 | 12) {
            session.ingest_json(
                br#"{"type":"motion_state","data":{"isTakeoutPartial":false,"isTakeoutFull":false}}"#,
                &mut delivery,
            );
        }
        session.ingest_json(&serde_json::to_vec(envelope).unwrap(), &mut delivery);
    }
    delivery
}

fn replay_synthetic_rounds(mode: AnnouncementMode) -> RecordingDelivery {
    let mut session = session(mode);
    let mut delivery = RecordingDelivery::default();
    let scenarios: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../tests/fixtures/board-manager-jvl14-synthetic-scenarios.json"
    ))
    .expect("valid synthetic scenarios");
    for round in 0..3 {
        session.ingest_json(
            br#"{"type":"motion_state","data":{"isTakeoutPartial":false,"isTakeoutFull":false}}"#,
            &mut delivery,
        );
        for envelope in scenarios["deadline_timer_first"].as_array().unwrap() {
            session.ingest_json(&serde_json::to_vec(envelope).unwrap(), &mut delivery);
        }
        let token = session
            .pending_total_deadline()
            .expect("third dart deadline");
        if round == 1 {
            session.ingest_json(
                br#"{"type":"state","data":{"running":true,"numThrows":0}}"#,
                &mut delivery,
            );
        }
        let marker = if round == 1 {
            br#"{"type":"motion_state","data":{"isTakeoutPartial":true}}"#.as_slice()
        } else {
            br#"{"type":"motion_state","data":{"isTakeoutFull":true}}"#.as_slice()
        };
        assert_eq!(
            session.ingest_json(marker, &mut delivery),
            JourneyResult::Total(Some(26))
        );
        assert_eq!(
            session.ingest_json(marker, &mut delivery),
            JourneyResult::Suppressed
        );
        assert_eq!(
            session.on_total_deadline(token, &mut delivery),
            JourneyResult::Suppressed
        );
    }
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

    for mode in [
        AnnouncementMode::EachDartThenTotal,
        AnnouncementMode::TotalOnly,
    ] {
        let partial = replay_synthetic_partial(mode);
        let timer = replay_deadline_first(mode);
        let old_capture = replay_capture_with_synthetic_continuation(mode);
        let rounds = replay_synthetic_rounds(mode);
        if mode == AnnouncementMode::EachDartThenTotal {
            assert_eq!(texts(&partial), ["20", "5", "25"]);
            assert_eq!(texts(&timer), ["20", "5", "1", "26"]);
            assert_eq!(
                texts(&old_capture),
                [
                    "3", "7", "7", "17", "3", "7", "7", "17", "3", "7", "7", "17"
                ]
            );
            assert_eq!(
                texts(&rounds),
                [
                    "20", "5", "1", "26", "20", "5", "1", "26", "20", "5", "1", "26"
                ]
            );
        } else {
            assert_eq!(texts(&partial), ["25"]);
            assert_eq!(texts(&timer), ["26"]);
            assert_eq!(texts(&old_capture), ["17", "17", "17"]);
            assert_eq!(texts(&rounds), ["26", "26", "26"]);
        }
    }

    println!("JVL-14 exact reduced capture: 87, 60, 30 totals in both modes");
    println!("JVL-13 reduced capture stays silent; separate synthetic continuation: 17, 17, 17");
    println!("Synthetic partial and deadline-first scenarios remain separately labelled");
    println!("These are policy submissions; they do not prove host transport or audibility.");
}
