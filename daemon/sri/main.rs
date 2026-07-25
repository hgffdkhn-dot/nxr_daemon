use std::os::unix::net::UnixListener as StdUnixListener;
use std::os::unix::fs::PermissionsExt;
use std::thread;

fn main() {
    let socket_path = "/data/local/tmp/nxr_daemon";
    let _ = std::fs::create_dir_all("/data/local/tmp");
    let _ = std::fs::remove_file(socket_path);

    let listener = StdUnixListener::bind(socket_path).expect("Bind failed");
    println!("Bound to {}", socket_path);

    // 设置权限 777
    let mut perms = std::fs::metadata(socket_path).unwrap().permissions();
    perms.set_mode(0o777);
    let _ = std::fs::set_permissions(socket_path, perms);
    println!("Permissions set to 777");

    // 只接受连接，打印然后关闭
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("ACCEPTED a connection!");
                // 直接关闭，不处理数据
                drop(stream);
            }
            Err(e) => eprintln!("Accept error: {}", e),
        }
    }
}
