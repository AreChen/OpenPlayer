#[cfg(not(windows))]
fn main() {
    openplayer_desktop_lib::run_window_smoke();
}

#[cfg(windows)]
fn main() {
    eprintln!("Use the OpenPlayer native executable for Windows smoke tests.");
    std::process::exit(1);
}
