use futures_util::SinkExt;
use niles_core::{DeviceRegistry, EventBus, RoomName};
use niles_plugins::speech_stream::{SessionSpeechHost, SpeechStreamHost};
use niles_plugins::{House, Limits, MemoryStore, Runtime};
use niles_tts::speech_session::{
    Eligibility, PluginOwner, SpeechEligibility, SpeechOutput, SpeechSessionManager,
};
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::runtime::Handle;
use tokio_tungstenite::tungstenite::Message;

const MANIFEST: &str = include_str!("../niles-integration.toml");
const BASELINE: &str = r#"{"type":"state","data":{"running":true,"numThrows":0,"throws":[]}}"#;
const DART: &str = r#"{"type":"state","data":{"running":true,"numThrows":1,"throws":[{"segment":{"number":20,"multiplier":1}}]}}"#;
const DART_TWO: &str = r#"{"type":"state","data":{"running":true,"numThrows":2,"throws":[{"segment":{"number":20,"multiplier":1}},{"segment":{"number":5,"multiplier":1}}]}}"#;
const DART_THREE: &str = r#"{"type":"state","data":{"running":true,"numThrows":3,"throws":[{"segment":{"number":20,"multiplier":1}},{"segment":{"number":5,"multiplier":1}},{"segment":{"number":1,"multiplier":1}}]}}"#;
const PARTIAL: &str = r#"{"type":"motion_state","data":{"isTakeoutPartial":true}}"#;
const FULL: &str = r#"{"type":"motion_state","data":{"isTakeoutFull":true}}"#;

type Prepared = (String, String);

#[derive(Default)]
struct OutputTrace {
    prepared: Mutex<Vec<(String, String)>>,
    delivered: Mutex<Vec<(String, String)>>,
}

fn speech_host(trace: Arc<OutputTrace>) -> Arc<SessionSpeechHost<Prepared>> {
    let prepare_trace = Arc::clone(&trace);
    let deliver_trace = Arc::clone(&trace);
    let output = SpeechOutput::new(
        move |room: RoomName, text: String| {
            let trace = Arc::clone(&prepare_trace);
            async move {
                trace
                    .prepared
                    .lock()
                    .unwrap()
                    .push((room.to_string(), text.clone()));
                Ok((room.to_string(), text))
            }
        },
        move |prepared: Prepared| {
            let trace = Arc::clone(&deliver_trace);
            async move {
                trace.delivered.lock().push(prepared);
                Ok(())
            }
        },
    );
    let policy =
        SpeechEligibility::new(|_: PluginOwner, _: RoomName| async { Eligibility::Eligible });
    let manager = Arc::new(SpeechSessionManager::new(output, policy));
    Arc::new(SessionSpeechHost::with_route_validator(
        manager,
        Handle::current(),
        |_| true,
    ))
}

async fn wait_for_deliveries(trace: &OutputTrace, expected: &[&str]) {
    let deadline = Instant::now() + Duration::from_secs(12);
    loop {
        let rows = trace.delivered.lock().clone();
        let texts: Vec<_> = rows.iter().map(|(_, text)| text.as_str()).collect();
        if texts == expected {
            assert!(rows.iter().all(|(room, _)| room == "living_room"));
            return;
        }
        assert!(
            Instant::now() < deadline,
            "deliveries were {texts:?}, wanted {expected:?}"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

async fn prove_mode(mode: &str, expected: &[&str]) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("loopback listener available");
    let address = listener.local_addr().expect("listener address");
    let server = tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            tokio::spawn(async move {
                let mut peek = [0u8; 256];
                let count = loop {
                    let count = stream.peek(&mut peek).await.unwrap();
                    if count > 0 {
                        break count;
                    }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                };
                let request = String::from_utf8_lossy(&peek[..count]);
                if request.starts_with("GET /api/events ") {
                    let mut socket = tokio_tungstenite::accept_async(stream).await.unwrap();
                    let frames = if mode == "total_only" {
                        [BASELINE, DART, DART_TWO, DART_THREE, FULL]
                    } else {
                        [BASELINE, DART, DART_TWO, DART_THREE, PARTIAL]
                    };
                    for frame in frames {
                        socket.send(Message::Text(frame.into())).await.unwrap();
                    }
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    return;
                }
                let body = if request.starts_with("GET /api/version ") {
                    b"1.0.0-fixture".as_slice()
                } else if request.starts_with("GET /api/state ") {
                    br#"{"connected":true,"running":true,"numThrows":0}"#.as_slice()
                } else {
                    b"".as_slice()
                };
                let status = if body.is_empty() {
                    "404 Not Found"
                } else {
                    "200 OK"
                };
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                use tokio::io::AsyncWriteExt;
                stream.write_all(response.as_bytes()).await.unwrap();
                stream.write_all(body).await.unwrap();
            });
        }
    });

    let mut settings = toml::Table::new();
    settings.insert(
        "address".into(),
        toml::Value::String(format!("127.0.0.1:{}", address.port())),
    );
    settings.insert("room".into(), toml::Value::String("living_room".into()));
    settings.insert("poll_interval".into(), toml::Value::Integer(5));
    settings.insert("announcements_enabled".into(), toml::Value::Boolean(true));
    settings.insert("announcement_mode".into(), toml::Value::String(mode.into()));
    settings.insert("speech_stream_enabled".into(), toml::Value::Boolean(true));
    let read = Arc::new(settings);
    let config = read.clone();
    let house = House::new(
        Arc::new(DeviceRegistry::new()),
        EventBus::new(128),
        Arc::new(MemoryStore::new()),
    )
    .with_settings(Arc::new(move |_| (*config).clone()))
    .allowing_loopback();
    let runtime = Runtime::new(house, Limits::default()).expect("Niles runtime initializes");
    let trace = Arc::new(OutputTrace::default());
    runtime.connect_speech_stream(speech_host(Arc::clone(&trace)));
    runtime
        .install(
            MANIFEST,
            std::fs::read(
                std::env::var("NILES_COMPONENT_WASM")
                    .expect("set NILES_COMPONENT_WASM to the exact staged component Wasm"),
            )
            .expect("read staged component Wasm"),
        )
        .await
        .expect("manifest and compiled Autodarts component install");
    wait_for_deliveries(&trace, expected).await;
    assert_eq!(trace.prepared.lock().len(), expected.len());
    server.abort();
    let _ = server.await;
    runtime.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn compiled_component_uses_real_niles_speech_scheduler() {
    prove_mode("each_dart_then_total", &["20", "5", "1", "26"]).await;
    prove_mode("total_only", &["26"]).await;
}
