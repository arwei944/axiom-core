fn main() {
    archcheck::build_hook::check_current_crate(env!("CARGO_PKG_NAME"));
}
