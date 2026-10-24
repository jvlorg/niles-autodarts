use autodarts_core::local_score::JourneyResult;
use autodarts_core::{
    AnnouncementDelivery, AnnouncementMode, AnnouncementSession, AnnouncementSettings,
    RecordingDelivery,
};

const BASELINE: &str = r#"{"type":"state","data":{"running":true,"numThrows":0,"throws":[]}}"#;
const DART_ONE: &str = r#"{"type":"state","data":{"running":true,"numThrows":1,"throws":[{"segment":{"number":20,"multiplier":1}}]}}"#;
const DART_TWO: &str = r#"{"type":"state","data":{"running":true,"numThrows":2,"throws":[{"segment":{"number":20,"multiplier":1}},{"segment":{"number":5,"multiplier":1}}]}}"#;
const DART_THREE: &str = r#"{"type":"state","data":{"running":true,"numThrows":3,"throws":[{"segment":{"number":20,"multiplier":1}},{"segment":{"number":5,"multiplier":1}},{"segment":{"number":1,"multiplier":1}}]}}"#;
const CLEAR: &str = r#"{"type":"state","data":{"running":true,"numThrows":0,"throws":[]}}"#;
const PARTIAL: &str = r#"{"type":"motion_state","data":{"isTakeoutPartial":true}}"#;
const FULL: &str = r#"{"type":"motion_state","data":{"isTakeoutFull":true}}"#;

fn run(
    session: &mut AnnouncementSession,
    delivery: &mut impl AnnouncementDelivery,
    json: &str,
) -> JourneyResult {
    session.ingest_json(json.as_bytes(), delivery)
}

fn replay_mode(mode: AnnouncementMode) -> RecordingDelivery {
    let mode_name = match mode {
        AnnouncementMode::EachDartThenTotal => "each_dart_then_total",
        AnnouncementMode::TotalOnly => "total_only",
    };
    let settings = AnnouncementSettings::new(true, Some(mode_name), "living_room")
        .expect("valid replay settings");
    let mut session = AnnouncementSession::new(settings);
    let mut delivery = RecordingDelivery::default();
    for collection in 1..=3 {
        assert_eq!(
            run(&mut session, &mut delivery, BASELINE),
            JourneyResult::Baseline
        );
        assert_eq!(
            run(&mut session, &mut delivery, DART_ONE),
            JourneyResult::Dart((mode == AnnouncementMode::EachDartThenTotal).then_some(20))
        );
        assert_eq!(
            run(&mut session, &mut delivery, DART_TWO),
            JourneyResult::Dart((mode == AnnouncementMode::EachDartThenTotal).then_some(5))
        );
        assert_eq!(
            run(&mut session, &mut delivery, DART_THREE),
            JourneyResult::Dart((mode == AnnouncementMode::EachDartThenTotal).then_some(1))
        );
        if mode == AnnouncementMode::TotalOnly && collection == 2 {
            assert_eq!(
                run(&mut session, &mut delivery, CLEAR),
                JourneyResult::Suppressed
            );
        }
        let marker = if collection == 2 { PARTIAL } else { FULL };
        assert_eq!(
            run(&mut session, &mut delivery, marker),
            JourneyResult::Total(Some(26))
        );
        assert_eq!(
            run(&mut session, &mut delivery, marker),
            JourneyResult::Suppressed
        );
        println!("{mode_name} collection {collection}: policy total 26");
    }
    delivery
}

fn replay_capture_with_synthetic_continuation(mode: AnnouncementMode) -> RecordingDelivery {
    let mode_name = match mode {
        AnnouncementMode::EachDartThenTotal => "each_dart_then_total",
        AnnouncementMode::TotalOnly => "total_only",
    };
    let settings = AnnouncementSettings::new(true, Some(mode_name), "living_room")
        .expect("valid replay settings");
    let mut session = AnnouncementSession::new(settings);
    let mut delivery = RecordingDelivery::default();
    let capture: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../tests/fixtures/board-manager-1.0.7-reduced-capture.json"
    ))
    .expect("valid reduced capture");
    for envelope in capture.as_array().expect("capture array") {
        let json = serde_json::to_vec(envelope).expect("capture envelope");
        run(
            &mut session,
            &mut delivery,
            std::str::from_utf8(&json).expect("capture JSON"),
        );
    }
    assert!(delivery.requests.is_empty());
    println!("{mode_name}: captured prefix produced no policy submissions");

    let continuation: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../tests/fixtures/board-manager-1.0.7-synthetic-continuation.json"
    ))
    .expect("valid synthetic continuation");
    for envelope in continuation.as_array().expect("continuation array") {
        let json = serde_json::to_vec(envelope).expect("continuation envelope");
        run(
            &mut session,
            &mut delivery,
            std::str::from_utf8(&json).expect("continuation JSON"),
        );
    }
    println!(
        "{mode_name}: synthetic continuation accepted {} policy submissions",
        delivery.requests.len()
    );
    delivery
}

fn main() {
    println!(
        "Synthetic source-derived replay (not captures): settings → JSON → session → recording"
    );
    let each = replay_mode(AnnouncementMode::EachDartThenTotal);
    assert_eq!(each.requests.len(), 12);
    assert_eq!(
        each.requests
            .iter()
            .map(|request| request.text.as_str())
            .collect::<Vec<_>>(),
        [
            "20", "5", "1", "26", "20", "5", "1", "26", "20", "5", "1", "26"
        ]
    );
    let total = replay_mode(AnnouncementMode::TotalOnly);
    assert_eq!(total.requests.len(), 3);
    assert_eq!(
        total
            .requests
            .iter()
            .map(|request| request.text.as_str())
            .collect::<Vec<_>>(),
        ["26", "26", "26"]
    );
    let captured_each =
        replay_capture_with_synthetic_continuation(AnnouncementMode::EachDartThenTotal);
    assert_eq!(
        captured_each
            .requests
            .iter()
            .map(|request| request.text.as_str())
            .collect::<Vec<_>>(),
        [
            "3", "7", "7", "17", "3", "7", "7", "17", "3", "7", "7", "17"
        ]
    );
    let captured_total = replay_capture_with_synthetic_continuation(AnnouncementMode::TotalOnly);
    assert_eq!(
        captured_total
            .requests
            .iter()
            .map(|request| request.text.as_str())
            .collect::<Vec<_>>(),
        ["17", "17", "17"]
    );
    for request in each.requests.iter().chain(total.requests.iter()) {
        println!(
            "policy submission room={} text={} key={}",
            request.room, request.text, request.deduplication_key
        );
    }
    println!("These are policy submissions; they do not prove transport or audibility.");
}
