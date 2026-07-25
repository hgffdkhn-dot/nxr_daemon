use std::collections::HashMap;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener as StdUnixListener;
use std::sync::{Arc, Mutex};
use std::thread;

use prost::Message;

// 生成 protobuf 代码
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

// ---------- 客户端处理（同步） ----------
fn handle_client(mut stream: std::os::unix::net::UnixStream, state: Arc<Mutex<AppState>>) {
    let mut buf = vec![0u8; 4096];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break, // 连接关闭
            Ok(n) => {
                if let Ok(req) = Request::decode(&buf[..n]) {
                    let state = state.lock().unwrap();
                    let resp = state.handle_request(req);
                    let mut resp_buf = Vec::new();
                    resp.encode(&mut resp_buf).unwrap();
                    if let Err(e) = stream.write_all(&resp_buf) {
                        eprintln!("Write error: {}", e);
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
fn main() {
    // 应用沙盒内的 socket 路径，应用绝对可访问
    let socket_path = "/data/data/com.nexusroot.manager/files/nxr_daemon";

    // 确保目录存在（root 运行，可创建）
    let _ = std::fs::create_dir_all("/data/data/com.nexusroot.manager/files");
    let _ = std::fs::remove_file(socket_path);

    let listener = StdUnixListener::bind(socket_path).expect("Bind failed");
    // 设置 socket 文件权限 777
    let mut perms = std::fs::metadata(socket_path).unwrap().permissions();
    perms.set_mode(0o777);
    let _ = std::fs::set_permissions(socket_path, perms);
    // 同时确保父目录可被应用遍历（通常已可）
    let _ = std::fs::set_permissions(
        "/data/data/com.nexusroot.manager/files",
        std::fs::Permissions::from_mode(0o755),
    );

    println!("nexusrootd listening on {}", socket_path);

    let state = Arc::new(Mutex::new(AppState::new()));

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("Accepted connection");
                let state = Arc::clone(&state);
                thread::spawn(move || handle_client(stream, state));
            }
            Err(e) => eprintln!("Accept error: {}", e),
        }
    }
}
