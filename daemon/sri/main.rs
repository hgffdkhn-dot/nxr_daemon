use std::collections::HashMap;
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener;
use tokio::sync::Mutex;

use prost::Message;

mod pb {
    include!(concat!(env!("OUT_DIR"), "/nexusroot.rs"));
}
use pb::*;

// ---------- 白名单存储 ----------
struct AppState {
    whitelist: HashMap<i32, WhitelistItem>, // key = uid
}

impl AppState {
    fn new() -> Self {
        Self {
            whitelist: HashMap::new(),
        }
    }

    fn handle_request(&self, req: Request) -> Response {
        match req.payload {
            Some(request::Payload::Status(_)) => Response {
                success: true,
                payload: Some(response::Payload::Status(StatusResponse {
                    daemon_alive: true,
                    su_version: "NexusRoot v1.0.0".into(),
                    su_path: "/data/adb/nxr/bin/nr-su".into(),
                    se_context: "u:r:nxr_daemon:s0".into(),
                })),
            },
            Some(request::Payload::Whitelist(wl_req)) => {
                match WhitelistRequestAction::try_from(wl_req.action).unwrap() {
                    WhitelistRequestAction::List => {
                        let items: Vec<WhitelistItem> =
                            self.whitelist.values().cloned().collect();
                        Response {
                            success: true,
                            payload: Some(response::Payload::Whitelist(WhitelistResponse {
                                items,
                            })),
                        }
                    }
                    WhitelistRequestAction::Add => Response {
                        success: true,
                        payload: Some(response::Payload::Whitelist(WhitelistResponse {
                            items: wl_req.items,
                        })),
                    },
                    WhitelistRequestAction::Remove => Response {
                        success: true,
                        payload: Some(response::Payload::Whitelist(WhitelistResponse {
                            items: wl_req.items,
                        })),
                    },
                }
            }
            _ => Response {
                success: false,
                payload: None,
            },
        }
    }
}

#[derive(Debug, PartialEq)]
enum WhitelistRequestAction {
    List = 0,
    Add = 1,
    Remove = 2,
}

impl TryFrom<i32> for WhitelistRequestAction {
    type Error = ();
    fn try_from(v: i32) -> Result<Self, ()> {
        match v {
            0 => Ok(WhitelistRequestAction::List),
            1 => Ok(WhitelistRequestAction::Add),
            2 => Ok(WhitelistRequestAction::Remove),
            _ => Err(()),
        }
    }
}

// ---------- 客户端处理 ----------
async fn handle_client(mut stream: tokio::net::UnixStream, state: Arc<Mutex<AppState>>) {
    let mut buf = vec![0u8; 4096];
    loop {
        match stream.read(&mut buf).await {
            Ok(0) => break, // 连接关闭
            Ok(n) => {
                if let Ok(req) = Request::decode(&buf[..n]) {
                    let state = state.lock().await;
                    let resp = state.handle_request(req);
                    let mut resp_buf = Vec::new();
                    resp.encode(&mut resp_buf).unwrap();
                    if let Err(e) = stream.write_all(&resp_buf).await {
                        eprintln!("Failed to write response: {}", e);
                        break;
                    }
                }
            }
            Err(e) => {
                eprintln!("Read error: {}", e);
                break;
            }
        }
    }
}

// ---------- 主入口 ----------
#[tokio::main]
async fn main() {
    let socket_path = "/data/local/tmp/nxr_daemon";
    let _ = std::fs::create_dir_all("/data/local/tmp");
    let _ = std::fs::remove_file(socket_path);

    println!("Attempting to bind to {}", socket_path);
    let listener = UnixListener::bind(socket_path).expect("Bind failed");
    println!("Bind successful");

    if let Ok(metadata) = std::fs::metadata(socket_path) {
        let mut perms = metadata.permissions();
        perms.set_mode(0o777);
        let _ = std::fs::set_permissions(socket_path, perms);
        println!("Permissions set to 777");
    }

    let state = Arc::new(Mutex::new(AppState::new()));

    println!("Entering accept loop...");
    loop {
        println!("Waiting for client...");
        match listener.accept().await {
            Ok((stream, addr)) => {
                println!("Accepted connection from {:?}", addr);
                let state = Arc::clone(&state);
                tokio::spawn(handle_client(stream, state));
            }
            Err(e) => eprintln!("Accept error: {}", e),
        }
    }
}
