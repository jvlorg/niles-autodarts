//! Synthetic Niles imports. The exported component uses the production WIT bindings.
pub mod niles {
    pub mod integration {
        pub mod config {
            use std::cell::RefCell;
            #[derive(Clone, Debug)]
            #[allow(dead_code)]
            pub enum SettingValue {
                Text(String),
                Number(f64),
                Boolean(bool),
            }
            thread_local! { static VALUES: RefCell<std::collections::BTreeMap<String, SettingValue>> = const { RefCell::new(std::collections::BTreeMap::new()) }; }
            pub fn set(key: &str, value: SettingValue) {
                VALUES.with(|values| {
                    values.borrow_mut().insert(key.into(), value);
                });
            }
            pub fn reset() {
                VALUES.with(|values| values.borrow_mut().clear());
            }
            pub fn get(key: &str) -> Option<SettingValue> {
                VALUES.with(|values| values.borrow().get(key).cloned())
            }
        }
        pub mod log {
            #[derive(Debug)]
            pub enum Level {
                Warn,
            }
            pub fn log(_: Level, _: &str) {}
        }
        pub mod http {
            use std::cell::RefCell;
            #[derive(Clone, Copy, Debug, PartialEq, Eq)]
            #[allow(dead_code)]
            pub enum Method {
                Get,
                Head,
                Post,
                Put,
                Patch,
                Delete,
            }
            #[derive(Clone, Debug)]
            #[allow(dead_code)]
            pub enum HttpError {
                NotPermitted(String),
                InvalidRequest(String),
                Timeout,
                Connection(String),
                TooLarge,
            }
            #[derive(Clone, Debug)]
            #[allow(dead_code)]
            pub struct Request {
                pub method: Method,
                pub url: String,
                pub headers: Vec<(String, String)>,
                pub body: Option<Vec<u8>>,
                pub timeout_ms: Option<u32>,
            }
            #[derive(Clone, Debug)]
            #[allow(dead_code)]
            pub struct Response {
                pub status: u16,
                pub headers: Vec<(String, String)>,
                pub body: Vec<u8>,
            }
            #[derive(Clone, Debug, PartialEq, Eq)]
            pub struct ObservedRequest {
                pub method: Method,
                pub url: String,
                pub timeout_ms: Option<u32>,
                pub body: Option<Vec<u8>>,
            }
            thread_local! { static FAIL_NEXT: RefCell<Option<HttpError>> = const { RefCell::new(None) }; }
            pub fn fail_next(error: HttpError) {
                FAIL_NEXT.with(|slot| *slot.borrow_mut() = Some(error));
            }
            thread_local! {
                static REPLIES: RefCell<std::collections::VecDeque<Result<Response, HttpError>>> = RefCell::new(Default::default());
                static REQUESTS: RefCell<Vec<ObservedRequest>> = const { RefCell::new(Vec::new()) };
            }
            pub fn set_replies(replies: impl IntoIterator<Item = Result<Response, HttpError>>) {
                REPLIES.with(|slot| *slot.borrow_mut() = replies.into_iter().collect());
                REQUESTS.with(|slot| slot.borrow_mut().clear());
                FAIL_NEXT.with(|slot| *slot.borrow_mut() = None);
            }
            pub fn requests() -> Vec<ObservedRequest> {
                REQUESTS.with(|slot| slot.borrow().clone())
            }
            pub fn send(request: &Request) -> Result<Response, HttpError> {
                REQUESTS.with(|slot| {
                    slot.borrow_mut().push(ObservedRequest {
                        method: request.method,
                        url: request.url.clone(),
                        timeout_ms: request.timeout_ms,
                        body: request.body.clone(),
                    })
                });
                if let Some(error) = FAIL_NEXT.with(|slot| slot.borrow_mut().take()) {
                    return Err(error);
                }
                REPLIES.with(|slot| slot.borrow_mut().pop_front().expect("HTTP fixture reply"))
            }
        }
        pub mod devices {
            use super::super::super::super::bindings::niles::integration::types;
            use std::cell::RefCell;
            #[derive(Clone, Debug)]
            #[allow(dead_code)]
            pub struct RecordedDevice {
                pub id: String,
                pub state: types::DeviceState,
            }
            #[derive(Default, Clone)]
            pub struct Calls {
                pub upserts: Vec<RecordedDevice>,
                pub availability: Vec<(String, bool)>,
            }
            thread_local! { static CALLS: RefCell<Calls> = RefCell::new(Calls::default()); }
            pub fn reset() {
                CALLS.with(|slot| *slot.borrow_mut() = Calls::default());
            }
            pub fn upsert(device: &types::Device) -> Result<(), ()> {
                CALLS.with(|slot| {
                    slot.borrow_mut().upserts.push(RecordedDevice {
                        id: device.id.clone(),
                        state: device.state.clone(),
                    })
                });
                Ok(())
            }
            pub fn set_available(id: &str, available: bool) -> Result<(), ()> {
                CALLS.with(|slot| slot.borrow_mut().availability.push((id.into(), available)));
                Ok(())
            }
            pub fn calls() -> Calls {
                CALLS.with(|slot| slot.borrow().clone())
            }
        }
        pub mod timers {
            use std::cell::RefCell;
            thread_local! { static CALLS: RefCell<Vec<(String, u32)>> = const { RefCell::new(Vec::new()) }; }
            pub fn every(id: &str, seconds: u32) -> Result<(), ()> {
                CALLS.with(|slot| slot.borrow_mut().push((id.into(), seconds)));
                Ok(())
            }
            #[allow(dead_code)]
            pub fn calls() -> Vec<(String, u32)> {
                CALLS.with(|slot| slot.borrow().clone())
            }
        }
        pub mod ui {
            use std::cell::RefCell;
            thread_local! { static INVALIDATIONS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) }; }
            pub fn invalidate(surface: &str) {
                INVALIDATIONS.with(|slot| slot.borrow_mut().push(surface.into()));
            }
            #[allow(dead_code)]
            pub fn invalidations() -> Vec<String> {
                INVALIDATIONS.with(|slot| slot.borrow().clone())
            }
        }
        pub mod speech_stream {
            use super::super::super::super::bindings::niles::integration::{
                speech_stream::RequestStatus, types::HostError,
            };
            use std::cell::RefCell;
            use std::collections::{BTreeMap, VecDeque};

            #[derive(Default, Clone)]
            pub struct Calls {
                pub opened_rooms: Vec<String>,
                pub opened_sessions: Vec<(u32, String)>,
                pub submitted: Vec<(u32, String)>,
                pub submitted_ids: Vec<(u32, u64, String)>,
                pub closed: Vec<u32>,
                pub close_failures: Vec<u32>,
                pub statuses: Vec<RequestStatus>,
                pub status_calls: Vec<(u32, u64)>,
                next_session: u32,
                next_request: u64,
                open_error: Option<HostError>,
                submit_errors: VecDeque<HostError>,
                status_errors: VecDeque<HostError>,
                status_sequences: BTreeMap<u64, VecDeque<Result<RequestStatus, HostError>>>,
                close_error: Option<HostError>,
            }
            thread_local! {
                static CALLS: RefCell<Calls> = RefCell::new(Calls::default());
                static NEXT_SESSION: RefCell<u32> = const { RefCell::new(7) };
                static NEXT_REQUEST: RefCell<u64> = const { RefCell::new(11) };
            }
            pub fn reset() {
                let session = NEXT_SESSION.with(|slot| {
                    let mut value = slot.borrow_mut();
                    let current = *value;
                    *value = value.saturating_add(1);
                    current
                });
                let request = NEXT_REQUEST.with(|slot| {
                    let mut value = slot.borrow_mut();
                    let current = *value;
                    *value = value.saturating_add(128);
                    current
                });
                CALLS.with(|slot| {
                    *slot.borrow_mut() = Calls {
                        next_session: session,
                        next_request: request,
                        ..Calls::default()
                    }
                });
            }
            pub fn calls() -> Calls {
                CALLS.with(|slot| slot.borrow().clone())
            }
            pub fn set_statuses(statuses: Vec<RequestStatus>) {
                CALLS.with(|slot| slot.borrow_mut().statuses = statuses);
            }
            pub fn set_open_error(error: Option<HostError>) {
                CALLS.with(|slot| slot.borrow_mut().open_error = error);
            }
            pub fn set_submit_errors(errors: impl IntoIterator<Item = HostError>) {
                CALLS.with(|slot| slot.borrow_mut().submit_errors = errors.into_iter().collect());
            }
            pub fn set_status_errors(errors: impl IntoIterator<Item = HostError>) {
                CALLS.with(|slot| slot.borrow_mut().status_errors = errors.into_iter().collect());
            }
            pub fn open(room: &str) -> Result<u32, HostError> {
                CALLS.with(|slot| {
                    let mut calls = slot.borrow_mut();
                    calls.opened_rooms.push(room.into());
                    if let Some(error) = &calls.open_error {
                        return Err(error.clone());
                    }
                    let session = calls.next_session;
                    calls.next_session = calls.next_session.saturating_add(1);
                    calls.opened_sessions.push((session, room.into()));
                    Ok(session)
                })
            }
            pub fn submit(session: u32, text: &str) -> Result<u64, HostError> {
                CALLS.with(|slot| {
                    let mut calls = slot.borrow_mut();
                    let request = calls.next_request;
                    if let Some(error) = calls.submit_errors.pop_front() {
                        return Err(error);
                    }
                    calls.next_request = calls.next_request.saturating_add(1);
                    calls.submitted.push((session, text.into()));
                    calls.submitted_ids.push((session, request, text.into()));
                    Ok(request)
                })
            }
            pub fn poll(session: u32) -> Result<Vec<RequestStatus>, HostError> {
                CALLS.with(|slot| {
                    let mut calls = slot.borrow_mut();
                    let next = calls.status_calls.len() as u64;
                    calls.status_calls.push((session, next));
                    if let Some(error) = calls.status_errors.pop_front() {
                        return Err(error);
                    }
                    let mut statuses = calls.statuses.clone();
                    for sequence in calls.status_sequences.values_mut() {
                        if let Some(result) = sequence.pop_front() {
                            statuses.push(result?);
                        }
                    }
                    Ok(statuses)
                })
            }
            pub fn close(session: u32) {
                CALLS.with(|slot| {
                    let mut calls = slot.borrow_mut();
                    calls.closed.push(session);
                    if calls.close_error.is_some() {
                        calls.close_failures.push(session);
                    }
                });
            }
        }
        pub mod websocket {
            use super::super::super::super::bindings::niles::integration::{
                types::HostError,
                websocket::{ConnectionStatus, Message},
            };
            use std::cell::RefCell;
            #[derive(Default, Clone)]
            pub struct Calls {
                pub opened: Vec<(String, String)>,
                pub closed: Vec<u32>,
                pub fail_open: bool,
            }
            thread_local! { static CALLS: RefCell<Calls> = RefCell::new(Calls::default()); }
            pub fn reset() {
                CALLS.with(|slot| *slot.borrow_mut() = Calls::default());
            }
            pub fn calls() -> Calls {
                CALLS.with(|slot| slot.borrow().clone())
            }
            pub fn connect(
                grant: &str,
                url: &str,
                _: &[(String, String)],
            ) -> Result<u32, HostError> {
                CALLS.with(|slot| {
                    let mut calls = slot.borrow_mut();
                    calls.opened.push((grant.into(), url.into()));
                    if calls.fail_open {
                        Err(HostError::Unavailable("fixture".into()))
                    } else {
                        Ok(40 + u32::try_from(calls.opened.len()).unwrap())
                    }
                })
            }
            pub fn close(conn: u32) {
                CALLS.with(|slot| slot.borrow_mut().closed.push(conn));
            }
            #[allow(dead_code)]
            pub fn message(conn: u32, message: Message) {
                <super::super::super::super::Plugin as super::super::super::super::bindings::Guest>::on_websocket_message(conn, message);
            }
            #[allow(dead_code)]
            pub fn status(conn: u32, status: ConnectionStatus) {
                <super::super::super::super::Plugin as super::super::super::super::bindings::Guest>::on_websocket_status(conn, status);
            }
        }
        #[allow(dead_code)]
        pub enum Event {}
        #[allow(dead_code)]
        pub struct ToolDef;
    }
}
