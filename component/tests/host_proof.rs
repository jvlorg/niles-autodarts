use futures_util::SinkExt;
use niles_core::{DeviceRegistry, EventBus, RoomName};
use niles_plugins::speech_stream::{
    SessionSpeechHost, SpeechError, SpeechOutcome, SpeechStreamHost,
};
use niles_plugins::{Context, House, Limits, MemoryStore, Runtime};
use niles_tts::speech_session::{
    Eligibility, PluginOwner, SpeechEligibility, SpeechOutput, SpeechSessionManager, TerminalResult,
};
use parking_lot::Mutex;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::Handle;
use tokio::sync::{Notify, Semaphore, mpsc};
use tokio::task::{JoinHandle, JoinSet};
use tokio_tungstenite::{WebSocketStream, tungstenite::Message};

const BASELINE: &str = r#"{"type":"state","data":{"running":true,"numThrows":0,"throws":[]}}"#;
const DART: &str = r#"{"type":"state","data":{"running":true,"numThrows":1,"throws":[{"segment":{"number":20,"multiplier":1}}]}}"#;
const DART_TWO: &str = r#"{"type":"state","data":{"running":true,"numThrows":2,"throws":[{"segment":{"number":20,"multiplier":1}},{"segment":{"number":5,"multiplier":1}}]}}"#;
const DART_THREE: &str = r#"{"type":"state","data":{"running":true,"numThrows":3,"throws":[{"segment":{"number":20,"multiplier":1}},{"segment":{"number":5,"multiplier":1}},{"segment":{"number":1,"multiplier":1}}]}}"#;
const PARTIAL: &str = r#"{"type":"motion_state","data":{"isTakeoutPartial":true}}"#;
const FULL: &str = r#"{"type":"motion_state","data":{"isTakeoutFull":true}}"#;
const WAIT: Duration = Duration::from_secs(15);
type Prepared = (String, String);
type Socket = WebSocketStream<TcpStream>;
type Outcomes = Vec<(u64, SpeechOutcome)>;

struct OutputTrace {
    prepared: Mutex<Vec<Prepared>>,
    entered: Mutex<Vec<Prepared>>,
    delivered: Mutex<Vec<Prepared>>,
    prepare_gate: Semaphore,
    deliver_gate: Semaphore,
    changed: Notify,
}

impl OutputTrace {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            prepared: Mutex::new(Vec::new()),
            entered: Mutex::new(Vec::new()),
            delivered: Mutex::new(Vec::new()),
            prepare_gate: Semaphore::new(0),
            deliver_gate: Semaphore::new(0),
            changed: Notify::new(),
        })
    }

    async fn wait_for(&self, rows: &Mutex<Vec<Prepared>>, count: usize) {
        tokio::time::timeout(WAIT, async {
            loop {
                let changed = self.changed.notified();
                if rows.lock().len() >= count {
                    return;
                }
                changed.await;
            }
        })
        .await
        .expect("speech output callback acknowledgement");
    }

    fn allow_one(&self) {
        self.prepare_gate.add_permits(1);
        self.deliver_gate.add_permits(1);
    }

    fn assert_delivered(&self, room: &str, texts: &[&str]) {
        let expected: Vec<_> = texts
            .iter()
            .map(|text| (room.to_owned(), (*text).to_owned()))
            .collect();
        assert_eq!(*self.delivered.lock(), expected);
    }
}

fn manager(trace: &Arc<OutputTrace>) -> Arc<SpeechSessionManager<Prepared>> {
    let prepare_trace = Arc::clone(trace);
    let deliver_trace = Arc::clone(trace);
    let output = SpeechOutput::new(
        move |room: RoomName, text: String| {
            let trace = Arc::clone(&prepare_trace);
            async move {
                let prepared = (room.to_string(), text);
                trace.prepared.lock().push(prepared.clone());
                trace.changed.notify_one();
                trace.prepare_gate.acquire().await.unwrap().forget();
                Ok(prepared)
            }
        },
        move |prepared: Prepared| {
            let trace = Arc::clone(&deliver_trace);
            async move {
                trace.entered.lock().push(prepared.clone());
                trace.changed.notify_one();
                trace.deliver_gate.acquire().await.unwrap().forget();
                trace.delivered.lock().push(prepared);
                trace.changed.notify_one();
                Ok(())
            }
        },
    );
    Arc::new(SpeechSessionManager::new(
        output,
        SpeechEligibility::new(|_: PluginOwner, _: RoomName| async { Eligibility::Eligible }),
    ))
}

#[derive(Debug)]
struct Request {
    session: u32,
    id: u64,
    room: String,
    text: String,
    outcome: Option<SpeechOutcome>,
}

#[derive(Default)]
struct SpeechTrace {
    sessions: BTreeMap<u32, (String, String)>,
    requests: Vec<Request>,
    buffered: BTreeMap<u32, Outcomes>,
    closes: Vec<u32>,
    forwarded: usize,
}

// Only observe/forward the released adapter. Its actual manager owns arbitration,
// replacement, expiry, transport and close. Polls used by the proof are buffered
// for the guest, never stolen or replaced with manufactured terminal results.
struct ObservedHost {
    inner: SessionSpeechHost<Prepared>,
    trace: Mutex<SpeechTrace>,
    changed: Notify,
}

impl ObservedHost {
    fn new(manager: Arc<SpeechSessionManager<Prepared>>) -> Arc<Self> {
        Arc::new(Self {
            inner: SessionSpeechHost::with_route_validator(manager, Handle::current(), |_| true),
            trace: Mutex::new(SpeechTrace::default()),
            changed: Notify::new(),
        })
    }

    fn collect(
        &self,
        trace: &mut SpeechTrace,
        owner: &str,
        session: u32,
    ) -> Result<(), SpeechError> {
        let outcomes = self.inner.poll(owner, session)?;
        for (id, outcome) in &outcomes {
            let request = trace
                .requests
                .iter_mut()
                .find(|request| request.session == session && request.id == *id)
                .expect("terminal result belongs to accepted request");
            assert!(request.outcome.replace(*outcome).is_none());
            println!(
                "host terminal: session={} id={} room={} text={:?} outcome={outcome:?}",
                request.session, request.id, request.room, request.text
            );
        }
        trace.buffered.entry(session).or_default().extend(outcomes);
        Ok(())
    }

    async fn terminals(&self, count: usize) {
        tokio::time::timeout(WAIT, async {
            loop {
                {
                    let mut trace = self.trace.lock();
                    let sessions: Vec<_> = trace
                        .sessions
                        .iter()
                        .map(|(session, (owner, _))| (*session, owner.clone()))
                        .collect();
                    for (session, owner) in sessions {
                        self.collect(&mut trace, &owner, session).unwrap();
                    }
                    if trace
                        .requests
                        .iter()
                        .filter(|r| r.outcome.is_some())
                        .count()
                        >= count
                    {
                        return;
                    }
                }
                // The released adapter has no completion notification API. Yield
                // while polling its real results; no timing delay orders a scenario.
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("real host terminal results");
    }

    async fn submissions(&self, count: usize) {
        tokio::time::timeout(WAIT, async {
            loop {
                let changed = self.changed.notified();
                if self.trace.lock().requests.len() >= count {
                    return;
                }
                changed.await;
            }
        })
        .await
        .expect("compiled guest submission acknowledgement");
    }

    async fn live_room(&self, room: &str) {
        tokio::time::timeout(WAIT, async {
            loop {
                let changed = self.changed.notified();
                let ready = {
                    let trace = self.trace.lock();
                    trace.sessions.iter().any(|(session, (_, opened_room))| {
                        opened_room == room && !trace.closes.contains(session)
                    })
                };
                if ready {
                    return;
                }
                changed.await;
            }
        })
        .await
        .expect("compiled guest opens a live speech session for the configured room");
    }

    fn assert_requests(&self, expected: &[(&str, SpeechOutcome)]) {
        let trace = self.trace.lock();
        let actual: Vec<_> = trace
            .requests
            .iter()
            .map(|request| (request.text.as_str(), request.outcome))
            .collect();
        let expected: Vec<_> = expected
            .iter()
            .map(|(text, outcome)| (*text, Some(*outcome)))
            .collect();
        assert_eq!(actual, expected);
    }
}

impl SpeechStreamHost for ObservedHost {
    fn open(&self, owner: &str, room: RoomName) -> Result<u32, SpeechError> {
        let mut trace = self.trace.lock();
        let session = self.inner.open(owner, room.clone())?;
        // The released adapter prunes finished sessions once their result queue
        // is drained. Keep their request history, but never poll a pruned slot.
        let retired: Vec<_> = trace
            .sessions
            .keys()
            .filter(|session| {
                trace.closes.contains(session)
                    && trace
                        .requests
                        .iter()
                        .all(|request| request.session != **session || request.outcome.is_some())
            })
            .copied()
            .collect();
        for retired in retired {
            trace.sessions.remove(&retired);
            trace.buffered.remove(&retired);
        }
        trace
            .sessions
            .insert(session, (owner.to_owned(), room.to_string()));
        println!("host open: session={session} room={room}");
        self.changed.notify_one();
        Ok(session)
    }

    fn submit(&self, owner: &str, session: u32, text: String) -> Result<u64, SpeechError> {
        let mut trace = self.trace.lock();
        let id = self.inner.submit(owner, session, text.clone())?;
        let room = trace.sessions[&session].1.clone();
        println!("host accepted: session={session} id={id} room={room} text={text:?}");
        trace.requests.push(Request {
            session,
            id,
            room,
            text,
            outcome: None,
        });
        self.changed.notify_one();
        Ok(id)
    }

    fn poll(&self, owner: &str, session: u32) -> Result<Outcomes, SpeechError> {
        let mut trace = self.trace.lock();
        self.collect(&mut trace, owner, session)?;
        let outcomes = trace.buffered.remove(&session).unwrap_or_default();
        trace.forwarded += outcomes.len();
        self.changed.notify_one();
        Ok(outcomes)
    }

    fn close(&self, owner: &str, session: u32) {
        self.trace.lock().closes.push(session);
        println!("host close: session={session}");
        self.inner.close(owner, session);
        self.changed.notify_one();
    }

    fn close_owner(&self, owner: &str) {
        // Released close_owner intentionally discards results after guest teardown.
        // Do not fabricate cancellations or retain sessions the adapter may prune.
        self.inner.close_owner(owner);
        let mut trace = self.trace.lock();
        let sessions: Vec<_> = trace
            .sessions
            .iter()
            .filter_map(|(session, (existing, _))| (existing == owner).then_some(*session))
            .collect();
        for session in sessions {
            trace.sessions.remove(&session);
            trace.buffered.remove(&session);
        }
    }
}

struct Board {
    task: JoinHandle<()>,
    connections: mpsc::UnboundedReceiver<Socket>,
}

impl Board {
    async fn start() -> Self {
        // Plan portability boundary: both the exact candidate and released host
        // restrict ws:// to declared 80/3180. A port-zero fixture is rejected even
        // if its address is supplied via settings. Never rewrite the candidate or
        // broaden production permissions merely to make this proof pass.
        let listener = TcpListener::bind("127.0.0.1:3180")
            .await
            .expect("host proof requires free loopback port 3180 (candidate permission); ephemeral-port proof is unsupported by the released contract");
        let (connections, rx) = mpsc::unbounded_channel();
        let task = tokio::spawn(async move {
            let mut children = JoinSet::new();
            loop {
                tokio::select! {
                    accepted = listener.accept() => {
                        let (mut stream, _) = accepted.unwrap();
                        let connections = connections.clone();
                        children.spawn(async move {
                            let mut peek = [0u8; 1024];
                            let count = tokio::time::timeout(WAIT, async {
                                loop {
                                    let count = stream.peek(&mut peek).await.unwrap();
                                    if count == 0 {
                                        return 0;
                                    }
                                    if peek[..count].windows(4).any(|part| part == b"\r\n\r\n") {
                                        return count;
                                    }
                                    assert!(count < peek.len(), "oversized fixture request headers");
                                    tokio::task::yield_now().await;
                                }
                            }).await.unwrap();
                            if count == 0 {
                                return;
                            }
                            if peek[..count].starts_with(b"GET /api/events ") {
                                let socket = tokio_tungstenite::accept_async(stream).await.unwrap();
                                let _ = connections.send(socket);
                                return;
                            }
                            // Consume the HTTP request before closing the socket;
                            // unread receive data can otherwise reset the response.
                            stream.read_exact(&mut peek[..count]).await.unwrap();
                            let body = if peek[..count].starts_with(b"GET /api/version ") {
                                "1.0.0-fixture"
                            } else if peek[..count].starts_with(b"GET /api/state ") {
                                r#"{"connected":true,"running":true,"numThrows":0}"#
                            } else {
                                ""
                            };
                            let status = if body.is_empty() { "404 Not Found" } else { "200 OK" };
                            let response = format!(
                                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                                body.len()
                            );
                            stream.write_all(response.as_bytes()).await.unwrap();
                        });
                    }
                    finished = children.join_next(), if !children.is_empty() => {
                        finished.unwrap().expect("fixture connection task");
                    }
                }
            }
        });
        Self {
            task,
            connections: rx,
        }
    }

    async fn connected(&mut self) -> Socket {
        tokio::time::timeout(WAIT, self.connections.recv())
            .await
            .expect("compiled guest connects to Board Manager WebSocket")
            .expect("fixture server remains running")
    }
}

impl Drop for Board {
    fn drop(&mut self) {
        // Dropping the server's JoinSet also aborts every HTTP connection task.
        self.task.abort();
    }
}

async fn send(socket: &mut Socket, frames: &[&str]) {
    for frame in frames {
        socket.send(Message::Text((*frame).into())).await.unwrap();
    }
}

struct Proof {
    runtime: Runtime,
    settings: Arc<Mutex<toml::Table>>,
    board: Board,
    host: Arc<ObservedHost>,
    output: Arc<OutputTrace>,
    manager: Arc<SpeechSessionManager<Prepared>>,
}

impl Proof {
    async fn start(mode: &str) -> (Self, Socket) {
        let wasm = std::path::PathBuf::from(
            std::env::var_os("NILES_COMPONENT_WASM")
                .expect("set NILES_COMPONENT_WASM to the exact staged integration.wasm"),
        );
        assert_eq!(wasm.file_name().unwrap(), "integration.wasm");
        let manifest = wasm.parent().unwrap().join("niles-integration.toml");
        let manifest = std::fs::read_to_string(manifest).expect("read exact staged manifest");
        let component = std::fs::read(&wasm).expect("read exact staged component Wasm");
        println!(
            "compiled candidate: {} (manifest unchanged; loopback port 3180 required)",
            wasm.display()
        );
        let mut board = Board::start().await;
        let settings = Arc::new(Mutex::new(toml::Table::from_iter([
            (
                "address".into(),
                toml::Value::String("127.0.0.1:3180".into()),
            ),
            ("room".into(), toml::Value::String("living_room".into())),
            ("poll_interval".into(), toml::Value::Integer(5)),
            ("announcements_enabled".into(), toml::Value::Boolean(true)),
            ("announcement_mode".into(), toml::Value::String(mode.into())),
            ("speech_stream_enabled".into(), toml::Value::Boolean(true)),
        ])));
        let config = Arc::clone(&settings);
        let house = House::new(
            Arc::new(DeviceRegistry::new()),
            EventBus::new(128),
            Arc::new(MemoryStore::new()),
        )
        .with_settings(Arc::new(move |_| config.lock().clone()))
        .allowing_loopback();
        let runtime = Runtime::new(house, Limits::default()).unwrap();
        let output = OutputTrace::new();
        let manager = manager(&output);
        let host = ObservedHost::new(Arc::clone(&manager));
        runtime.connect_speech_stream(host.clone());
        runtime.install(&manifest, component).await.unwrap();
        let socket = board.connected().await;
        host.live_room("living_room").await;
        (
            Self {
                runtime,
                settings,
                board,
                host,
                output,
                manager,
            },
            socket,
        )
    }

    async fn finish(self, label: &str, forwarded: usize) {
        tokio::time::timeout(WAIT, async {
            loop {
                let changed = self.host.changed.notified();
                if self.host.trace.lock().forwarded >= forwarded {
                    break;
                }
                changed.await;
            }
        })
        .await
        .expect("guest polls genuine terminal outcomes");
        // Includes the guest's own logical request keys/generations alongside the
        // host IDs and outcomes printed by the forwarding adapter above.
        println!(
            "{label} guest surface: {:?}",
            self.runtime
                .render("autodarts", "board", Context::default())
                .await
                .unwrap()
        );
        println!("{label} prepare: {:?}", self.output.prepared.lock());
        println!("{label} transport: {:?}", self.output.delivered.lock());
        self.runtime.shutdown().await;
        self.manager.shutdown().await;
        self.board.task.abort();
        // Await server cancellation so the next scenario can reuse its declared
        // port. The single test below deliberately serializes all scenarios.
        let mut board = self.board;
        let _ = (&mut board.task).await;
    }
}

async fn ordered_and_repeated_removal() {
    let (proof, mut socket) = Proof::start("each_dart_then_total").await;
    send(&mut socket, &[BASELINE]).await;
    for (index, frame) in [DART, DART_TWO, DART_THREE, PARTIAL]
        .into_iter()
        .enumerate()
    {
        proof.output.allow_one();
        send(&mut socket, &[frame]).await;
        proof.host.terminals(index + 1).await;
    }
    // The next clean collection is a positive barrier: all preceding repeated
    // markers must have been processed, without timing-based silence assertions.
    send(&mut socket, &[PARTIAL, FULL, FULL, BASELINE]).await;
    proof.output.allow_one();
    send(&mut socket, &[DART]).await;
    proof.host.terminals(5).await;
    proof.host.assert_requests(&[
        ("20", SpeechOutcome::Delivered),
        ("5", SpeechOutcome::Delivered),
        ("1", SpeechOutcome::Delivered),
        ("26", SpeechOutcome::Delivered),
        ("20", SpeechOutcome::Delivered),
    ]);
    proof
        .output
        .assert_delivered("living_room", &["20", "5", "1", "26", "20"]);
    proof.finish("ordered/repeated-removal", 5).await;
    drop(socket);
}

async fn burst_replacement() {
    let (proof, mut socket) = Proof::start("each_dart_then_total").await;
    send(&mut socket, &[BASELINE, DART]).await;
    proof.output.wait_for(&proof.output.prepared, 1).await;
    send(&mut socket, &[DART_TWO, DART_THREE, PARTIAL]).await;
    proof.host.submissions(4).await;
    proof.host.terminals(3).await;
    // No prepare permit has been issued: all three darts are genuinely replaced
    // by the total, including the dart already observed inside preparation.
    proof.output.allow_one();
    proof.host.terminals(4).await;
    proof.host.assert_requests(&[
        ("20", SpeechOutcome::Replaced),
        ("5", SpeechOutcome::Replaced),
        ("1", SpeechOutcome::Replaced),
        ("26", SpeechOutcome::Delivered),
    ]);
    proof.output.assert_delivered("living_room", &["26"]);
    proof.finish("burst/replacement", 4).await;
    drop(socket);
}

async fn total_only() {
    let (proof, mut socket) = Proof::start("total_only").await;
    proof.output.allow_one();
    send(
        &mut socket,
        &[BASELINE, DART, DART_TWO, DART_THREE, BASELINE, FULL],
    )
    .await;
    proof.host.terminals(1).await;
    proof.output.allow_one();
    send(&mut socket, &[PARTIAL, FULL, BASELINE, DART, PARTIAL]).await;
    proof.host.terminals(2).await;
    proof.host.assert_requests(&[
        ("26", SpeechOutcome::Delivered),
        ("20", SpeechOutcome::Delivered),
    ]);
    proof.output.assert_delivered("living_room", &["26", "20"]);
    proof
        .finish("total-only/clear-before-marker/repeated-removal", 2)
        .await;
    drop(socket);
}

async fn close_reconnect_and_room_switch() {
    let (mut proof, mut socket) = Proof::start("each_dart_then_total").await;
    send(&mut socket, &[BASELINE, DART]).await;
    proof.output.wait_for(&proof.output.prepared, 1).await;
    // Closing the actual fixture socket produces the released host's disconnect
    // callback, which must close the guest speech session during preparation.
    socket.close(None).await.unwrap();
    drop(socket);
    proof.host.terminals(1).await;
    proof
        .host
        .assert_requests(&[("20", SpeechOutcome::Cancelled)]);
    assert!(!proof.host.trace.lock().closes.is_empty());
    proof.output.assert_delivered("living_room", &[]);

    let mut socket = proof.board.connected().await;
    proof.host.live_room("living_room").await;
    proof.output.allow_one();
    send(&mut socket, &[DART, DART_TWO, FULL, BASELINE, DART]).await;
    proof.host.terminals(2).await;
    proof.host.assert_requests(&[
        ("20", SpeechOutcome::Cancelled),
        ("20", SpeechOutcome::Delivered),
    ]);
    proof.output.assert_delivered("living_room", &["20"]);
    // Disable/enable is the released public settings restart path. It deliberately
    // does not pretend that Runtime exposes a private guest-init callback.
    proof.runtime.disable("autodarts").await.unwrap();
    drop(socket);
    proof
        .settings
        .lock()
        .insert("room".into(), toml::Value::String("games_room".into()));
    let forwarded_before_room_switch = proof.host.trace.lock().forwarded;
    proof.runtime.enable("autodarts").await.unwrap();
    let mut socket = proof.board.connected().await;
    proof.host.live_room("games_room").await;
    proof.output.allow_one();
    send(&mut socket, &[DART, DART_TWO, FULL, BASELINE, DART]).await;
    proof.host.terminals(3).await;
    proof.host.assert_requests(&[
        ("20", SpeechOutcome::Cancelled),
        ("20", SpeechOutcome::Delivered),
        ("20", SpeechOutcome::Delivered),
    ]);
    let delivered = proof.output.delivered.lock().clone();
    assert_eq!(
        delivered,
        vec![
            ("living_room".into(), "20".into()),
            ("games_room".into(), "20".into())
        ]
    );
    // Cancellation was observed after disconnect, not forwarded to the discarded
    // guest generation. The new room result must still reach the current guest.
    proof
        .finish(
            "close/reconnect/room-switch",
            forwarded_before_room_switch + 1,
        )
        .await;
    drop(socket);
}

async fn active_transport_is_not_replaced() {
    // The released SessionSpeechHost poll is head-of-line ordered: while the
    // active request is Pending it cannot expose replacement of later requests.
    // Use the SAME released manager's public terminal receivers for this gate,
    // rather than inventing an adapter acknowledgement or using a timing sleep.
    // This scenario is manager-level evidence, NOT compiled-component ingress.
    let output = OutputTrace::new();
    let manager = manager(&output);
    let session = manager
        .open(
            PluginOwner::new("active-proof").unwrap(),
            RoomName::parse("living_room").unwrap(),
        )
        .await
        .unwrap();
    output.prepare_gate.add_permits(1);
    let (first_id, first) = manager.submit(session, "20").await.unwrap();
    output.wait_for(&output.entered, 1).await;
    let (second_id, second) = manager.submit(session, "5").await.unwrap();
    let (third_id, third) = manager.submit(session, "26").await.unwrap();
    assert_eq!(
        tokio::time::timeout(WAIT, second).await.unwrap().unwrap(),
        TerminalResult::Replaced
    );
    output.allow_one();
    assert_eq!(
        tokio::time::timeout(WAIT, first).await.unwrap().unwrap(),
        TerminalResult::DeliveredToOutput
    );
    output.wait_for(&output.entered, 2).await;
    output.deliver_gate.add_permits(1);
    assert_eq!(
        tokio::time::timeout(WAIT, third).await.unwrap().unwrap(),
        TerminalResult::DeliveredToOutput
    );
    output.assert_delivered("living_room", &["20", "26"]);
    println!(
        "manager-only active-transport trace: {first_id:?} 20 DeliveredToOutput; {second_id:?} 5 Replaced; {third_id:?} 26 DeliveredToOutput; output={:?}",
        output.delivered.lock()
    );
    manager.close(session).await;
    manager.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn compiled_component_uses_real_niles_speech_scheduler() {
    active_transport_is_not_replaced().await;
    ordered_and_repeated_removal().await;
    burst_replacement().await;
    total_only().await;
    close_reconnect_and_room_switch().await;
}
