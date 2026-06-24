//! The `restmd-devserver` binary: a local server for the demo `.restmd` files.

fn main() -> std::io::Result<()> {
    let addr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8787".to_string());
    println!("restmd-devserver listening on http://{addr}  (Ctrl-C to stop)");
    restmd_tui::devserver::serve(addr)
}
