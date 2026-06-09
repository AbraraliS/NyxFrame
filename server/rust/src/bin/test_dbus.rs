fn main() {
    println!("Testing Wayland DBus connection as root...");
    let dbus_addr = std::env::var("DBUS_SESSION_BUS_ADDRESS").unwrap_or_else(|_| "NOT SET".to_string());
    println!("DBUS_SESSION_BUS_ADDRESS: {}", dbus_addr);
    
    // We don't actually need to compile the whole server, just run a quick bash script to check dbus
}
