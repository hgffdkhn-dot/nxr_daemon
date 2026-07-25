use std::collections::HashMap;
use std::os::unix::net::UnixListener as StdUnixListener;
use std::os::unix::fs::PermissionsExt;
use std::sync::{Arc, Mutex};
use std::thread;
use std::io::{Read, Write};
use prost::Message;

mod pb {
    include!(concat!(env!("OUT_DIR"), "/nexusroot.rs"));
}
use pb::*;

struct AppState {
    whitelist: HashMap<i32, WhitelistItem>,
}

impl AppState {
    fn new() -> Self { Self { whitelist: HashMap::new() } }
    fn handle_request(&self, req: Request) -> Response {
        // 此处插入完整的 match 逻辑，同之前
        todo!()
    }
}

fn handle_client(mut stream: std::os::unix::net::UnixStream, state: Arc<Mutex<AppState>>) {
    let mut buf = vec![0u8; 4096];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
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
            Err(e) => { eprintln!("Read error: {}", e); break; }
        }
    }
}

fn main() {
    let socket_path = "/data/local/tmp/nxr_daemon";
    let _ = std::fs::create_dir_all("/data/local/tmp");
    let _ = std::fs::remove_file(socket_path);

    let listener = StdUnixListener::bind(socket_path).expect("Bind failed");
    let mut perms = std::fs::metadata(socket_path).unwrap().permissions();
    perms.set_mode(0o777);
    let _ = std::fs::set_permissions(socket_path, perms);
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
