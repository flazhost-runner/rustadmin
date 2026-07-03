//! RustAdmin — thin server binary. All logic lives in the `rust_admin` library crate.

#[macro_use]
extern crate rocket;

#[launch]
fn rocket() -> _ {
    rust_admin::build_rocket()
}
