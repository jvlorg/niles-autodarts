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
            #[derive(Default, Clone)]
            pub struct Calls {
                pub opened_rooms: Vec<String>,
                pub submitted: Vec<(u32, String)>,
                pub closed: Vec<u32>,
                pub statuses: Vec<RequestStatus>,
            }
            thread_local! { static CALLS: RefCell<Calls> = RefCell::new(Calls::default()); }
            pub fn reset() {
                CALLS.with(|slot| *slot.borrow_mut() = Calls::default());
            }
            pub fn calls() -> Calls {
                CALLS.with(|slot| slot.borrow().clone())
            }
            pub fn set_statuses(statuses: Vec<RequestStatus>) {
                CALLS.with(|slot| slot.borrow_mut().statuses = statuses);
            }
            pub fn open(room: &str) -> Result<u32, HostError> {
                CALLS.with(|slot| slot.borrow_mut().opened_rooms.push(room.into()));
                Ok(7)
            }
            pub fn submit(session: u32, text: &str) -> Result<u64, HostError> {
                CALLS.with(|slot| slot.borrow_mut().submitted.push((session, text.into())));
                Ok(11)
            }
            pub fn poll(_: u32) -> Result<Vec<RequestStatus>, HostError> {
                Ok(CALLS.with(|slot| slot.borrow().statuses.clone()))
            }
            pub fn close(session: u32) {
                CALLS.with(|slot| slot.borrow_mut().closed.push(session));
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
