use futures_util::SinkExt;
use niles_core::{DeviceRegistry, EventBus, RoomName};
use niles_plugins::speech_stream::{SpeechError, SpeechOutcome, SpeechStreamHost};
use niles_plugins::{House, Limits, MemoryStore, Runtime};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio_tungstenite::tungstenite::Message;

const MANIFEST: &str = include_str!("../niles-integration.toml");

const BASELINE: &str = r#"{"type":"state","data":{"running":true,"numThrows":0,"throws":[]}}"#;
const DART: &str = r#"{"type":"state","data":{"running":true,"numThrows":1,"throws":[{"segment":{"number":20,"multiplier":1}}]}}"#;
const PARTIAL: &str = r#"{"type":"motion_state","data":{"isTakeoutPartial":true}}"#;
const FULL: &str = r#"{"type":"motion_state","data":{"isTakeoutFull":true}}"#;

#[derive(Default)]
struct RecordingSpeech {
    room: Mutex<Option<String>>,
    submitted: Mutex<Vec<(u64, String, String)>>,
    polled: Mutex<Vec<u64>>,
}
impl SpeechStreamHost for RecordingSpeech {
    fn open(&self, _: &str, room: RoomName) -> Result<u32, SpeechError> {
        *self.room.lock().unwrap() = Some(room.to_string());
        Ok(1)
    }
    fn submit(&self, _: &str, session: u32, text: String) -> Result<u64, SpeechError> {
        assert_eq!(session, 1);
        let mut rows = self.submitted.lock().unwrap();
        let id = rows.len() as u64 + 1;
        rows.push((id, self.room.lock().unwrap().clone().unwrap(), text));
        Ok(id)
    }
    fn poll(&self, _: &str, _: u32) -> Result<Vec<(u64, SpeechOutcome)>, SpeechError> {
        let rows = self.submitted.lock().unwrap();
        let statuses: Vec<_> = rows.iter().map(|(id, _, _)| (*id, SpeechOutcome::Delivered)).collect();
        self.polled.lock().unwrap().extend(statuses.iter().map(|(id, _)| *id));
        Ok(statuses)
    }
    fn close(&self, _: &str, _: u32) {}
    fn close_owner(&self, _: &str) {}
}

async fn wait_for_speech(speech: &RecordingSpeech, expected: &[&str]) {
    let deadline = Instant::now() + Duration::from_secs(12);
    loop {
        let rows = speech.submitted.lock().unwrap().clone();
        let texts: Vec<_> = rows.iter().map(|(_, _, text)| text.as_str()).collect();
        if texts == expected {
            assert!(rows.iter().all(|(_, room, _)| room == "living_room"));
            return;
        }
        assert!(
            Instant::now() < deadline,
            "speech requests were {texts:?}, wanted {expected:?}"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

async fn wait_for_terminal_poll(speech: &RecordingSpeech) {
    let deadline = Instant::now() + Duration::from_secs(12);
    loop {
        let submitted = speech.submitted.lock().unwrap().len();
        let polled = speech.polled.lock().unwrap().len();
        if submitted > 0 && polled >= submitted {
            return;
        }
        assert!(Instant::now() < deadline, "terminal poll did not observe speech requests");
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

async fn prove_mode(mode: &str, expected: &[&str]) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3180")
        .await
        .expect("loopback port 3180 available");
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
            eprintln!(
                "fixture request: {}",
                request.lines().next().unwrap_or_default()
            );
            if request.starts_with("GET /api/events ") {
                let mut socket = tokio_tungstenite::accept_async(stream).await.unwrap();
                for frame in [BASELINE, DART, PARTIAL, PARTIAL, FULL] {
                    socket.send(Message::Text(frame.into())).await.unwrap();
                    tokio::time::sleep(Duration::from_millis(80)).await;
                }
                tokio::time::sleep(Duration::from_secs(15)).await;
                return;
            }
            let body = if request.starts_with("GET /api/version ") {
                b"1.0.0-fixture".as_slice()
            } else if request.starts_with("GET /api/state ") {
                br#"{"connected":true,"running":true,"numThrows":0}"#.as_slice()
            } else {
                b"".as_slice()
            };
            let status = if body.is_empty() { "404 Not Found" } else { "200 OK" };
            let response = format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
            use tokio::io::AsyncWriteExt;
            stream.write_all(response.as_bytes()).await.unwrap();
            stream.write_all(body).await.unwrap();
            });
        }
    });
    let mut settings = toml::Table::new();
    settings.insert(
        "address".into(),
        toml::Value::String("127.0.0.1:3180".into()),
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
    let speech = Arc::new(RecordingSpeech::default());
    runtime.connect_speech_stream(speech.clone());
    runtime
        .install(
            MANIFEST,
            std::fs::read(std::env::var("NILES_COMPONENT_WASM").expect("set NILES_COMPONENT_WASM to the built component"))
                .expect("read freshly built component Wasm"),
        )
        .await
        .expect("manifest and compiled Autodarts component install");
    if tokio::time::timeout(Duration::from_secs(12), wait_for_speech(&speech, expected))
        .await
        .is_err()
    {
        panic!(
            "speech timeout; runtime status={:?}, logs={:?}",
            runtime.info("autodarts"),
            runtime.log("autodarts")
        );
    }
    wait_for_terminal_poll(&speech).await;
    server.abort();
    let _ = server.await;
    runtime.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn compiled_component_runs_through_niles_websocket_policy_and_recording_speech_host() {
    prove_mode("each_dart_then_total", &["20", "20"]).await;
    prove_mode("total_only", &["20"]).await;
}
