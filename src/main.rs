//! RustAdmin — thin server binary. All logic lives in the `rust_admin` library crate.

use rocket::error::ErrorKind;

#[rocket::main]
async fn main() {
    let rocket = rust_admin::build_rocket();
    // The port the figment will bind (APP_PORT/ROCKET_PORT merged in build_rocket) —
    // read up front so the bind-failure message below can name it.
    let port: u16 = rocket.figment().extract_inner("port").unwrap_or(8000);

    if let Err(err) = rocket.launch().await {
        // Fail fast with a clear, actionable message (parity: NodeAdmin's EADDRINUSE
        // handler / GoAdmin's listen fail-fast) instead of rocket::Error's panicking
        // drop handler.
        match err.kind() {
            ErrorKind::Bind(io) if io.kind() == std::io::ErrorKind::AddrInUse => {
                eprintln!(
                    "Port {port} sudah dipakai proses lain. \
                     Hentikan instance lama atau ubah APP_PORT di .env."
                );
            }
            _ => eprintln!("Server gagal dijalankan: {err}"),
        }
        std::process::exit(1);
    }
}
