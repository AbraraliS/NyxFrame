use tokio::net::UnixListener;
use std::path::Path;
use std::fs;

#[tokio::main]
async fn main() {
    let socket_path = "/tmp/nyxframe_stream.sock";
    let path = Path::new(socket_path);
    
    if path.exists() {
        println!("Removing file...");
        if let Err(e) = fs::remove_file(path) {
            println!("remove_file failed: {}", e);
            return;
        }
    }

    println!("Binding...");
    match UnixListener::bind(path) {
        Ok(_) => println!("Bind succeeded!"),
        Err(e) => {
            println!("bind failed: {}", e);
            return;
        }
    }
    
    println!("Setting permissions...");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Err(e) = fs::set_permissions(path, fs::Permissions::from_mode(0o666)) {
            println!("set_permissions failed: {}", e);
            return;
        }
    }
    
    println!("Success!");
}
