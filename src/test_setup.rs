#[cfg(test)]
#[ctor::ctor]
fn init_tests() {
    // initialize a logger only for tests (to capture info and trace logs based on RUST_LOG)
    env_logger::init();
}
