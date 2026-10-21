use autodarts_core::local_score::JourneyResult;
use autodarts_core::{
    AnnouncementDelivery, AnnouncementSession, AnnouncementSettings, RecordingDelivery,
};

fn run(
    session: &mut AnnouncementSession,
    delivery: &mut impl AnnouncementDelivery,
    json: &str,
) -> JourneyResult {
    session.ingest_json(json.as_bytes(), delivery)
}

fn main() {
    let settings = AnnouncementSettings::new(true, Some("each_dart_then_total"), "living_room")
        .expect("valid replay settings");
    let mut session = AnnouncementSession::new(settings);
    let mut delivery = RecordingDelivery::default();
    println!(
        "Synthetic source-derived examples (not captures); settings → JSON → session → recording delivery:"
    );
    for collection in 1..=3 {
        assert_eq!(
            run(
                &mut session,
                &mut delivery,
                r#"{"type":"state","data":{"running":true,"numThrows":0,"throws":[]}}"#
            ),
            JourneyResult::Baseline
        );
        assert_eq!(
            run(
                &mut session,
                &mut delivery,
                r#"{"type":"state","data":{"running":true,"numThrows":1,"throws":[{"segment":{"number":20,"multiplier":1}}]}}"#
            ),
            JourneyResult::Dart(Some(20))
        );
        if collection == 2 {
            assert_eq!(
                run(
                    &mut session,
                    &mut delivery,
                    r#"{"type":"motion_state","data":{"isTakeoutPartial":true}}"#
                ),
                JourneyResult::Total(Some(20))
            );
            assert_eq!(
                run(
                    &mut session,
                    &mut delivery,
                    r#"{"type":"motion_state","data":{"isTakeoutFull":true}}"#
                ),
                JourneyResult::Suppressed
            );
        } else {
            assert_eq!(
                run(
                    &mut session,
                    &mut delivery,
                    r#"{"type":"motion_state","data":{"isTakeoutFull":true}}"#
                ),
                JourneyResult::Total(Some(20))
            );
        }
        println!("collection {collection}: total 20");
    }
    assert_eq!(
        delivery
            .requests
            .iter()
            .map(|r| r.text.as_str())
            .collect::<Vec<_>>(),
        ["20", "20", "20", "20", "20", "20"]
    );
    let totals: Vec<_> = delivery
        .requests
        .iter()
        .filter(|r| r.deduplication_key.contains(":total:"))
        .map(|r| r.deduplication_key.as_str())
        .collect();
    assert_eq!(totals.len(), 3);
    assert_eq!(
        totals
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        3
    );
    for request in &delivery.requests {
        println!(
            "request audience={:?} text={} key={}",
            request.room, request.text, request.deduplication_key
        );
    }
    println!("production delivery remains unavailable; recording adapter only");
}
