use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Mutex;

use lazy_static::lazy_static;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[cfg(feature = "native")]
mod native {
    use super::*;
    use vo_ext::prelude::*;
    use vo_runtime::builtins::error_helper::{write_error_to, write_nil_error};

    struct WatchState {
        _watcher: RecommendedWatcher,
        rx: Receiver<notify::Result<Event>>,
    }

    #[derive(Deserialize)]
    struct CreateReq {
        path: String,
        recursive: bool,
    }

    #[derive(Deserialize)]
    struct IdReq {
        id: u32,
    }

    #[derive(Deserialize)]
    struct PollReq {
        id: u32,
        max: usize,
    }

    #[derive(Serialize)]
    struct EventOut {
        path: String,
        op: String,
    }

    lazy_static! {
        static ref WATCHERS: Mutex<HashMap<u32, WatchState>> = Mutex::new(HashMap::new());
    }

    static NEXT_ID: AtomicU32 = AtomicU32::new(1);

    fn handle_create(input: &str) -> Result<Vec<u8>, String> {
        let req: CreateReq = serde_json::from_str(input).map_err(|e| e.to_string())?;
        let (tx, rx) = mpsc::channel();

        let mut watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            Config::default(),
        )
        .map_err(|e| e.to_string())?;

        let mode = if req.recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };
        watcher
            .watch(Path::new(&req.path), mode)
            .map_err(|e| e.to_string())?;

        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let mut map = WATCHERS
            .lock()
            .map_err(|_| "notify lock poisoned".to_string())?;
        map.insert(
            id,
            WatchState {
                _watcher: watcher,
                rx,
            },
        );

        serde_json::to_vec(&json!({ "id": id })).map_err(|e| e.to_string())
    }

    fn handle_poll(input: &str) -> Result<Vec<u8>, String> {
        let req: PollReq = serde_json::from_str(input).map_err(|e| e.to_string())?;
        let mut map = WATCHERS
            .lock()
            .map_err(|_| "notify lock poisoned".to_string())?;
        let state = map
            .get_mut(&req.id)
            .ok_or_else(|| format!("invalid watcher id {}", req.id))?;

        let max = req.max.max(1);
        let mut out = Vec::new();
        for _ in 0..max {
            match state.rx.try_recv() {
                Ok(Ok(ev)) => {
                    let op = format!("{:?}", ev.kind);
                    for p in ev.paths {
                        out.push(EventOut {
                            path: p.to_string_lossy().to_string(),
                            op: op.clone(),
                        });
                    }
                }
                Ok(Err(e)) => return Err(e.to_string()),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    return Err("watcher channel disconnected".to_string())
                }
            }
        }

        serde_json::to_vec(&json!({ "events": out })).map_err(|e| e.to_string())
    }

    fn handle_close(input: &str) -> Result<Vec<u8>, String> {
        let req: IdReq = serde_json::from_str(input).map_err(|e| e.to_string())?;
        let mut map = WATCHERS
            .lock()
            .map_err(|_| "notify lock poisoned".to_string())?;
        map.remove(&req.id)
            .ok_or_else(|| format!("invalid watcher id {}", req.id))?;
        Ok(Vec::new())
    }

    fn dispatch(op: &str, input: &str) -> Result<Vec<u8>, String> {
        match op {
            "create" => handle_create(input),
            "poll" => handle_poll(input),
            "close" => handle_close(input),
            _ => Err(format!("unsupported operation: {op}")),
        }
    }

    #[vo_fn("github.com/vo-lang/notify", "RawCall")]
    pub fn raw_call(call: &mut ExternCallContext) -> ExternResult {
        let op = call.arg_str(0);
        let input = call.arg_str(1);

        match dispatch(op, input) {
            Ok(bytes) => {
                let out_ref = call.alloc_bytes(&bytes);
                call.ret_ref(0, out_ref);
                write_nil_error(call, 1);
            }
            Err(msg) => {
                call.ret_nil(0);
                write_error_to(call, 1, &msg);
            }
        }

        ExternResult::Ok
    }
}

#[cfg(feature = "native")]
vo_ext::export_extensions!();
