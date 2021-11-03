#[cfg(test)]
#[ctor::ctor]
fn init_tests() {
    // initialize a logger only for tests (to capture info and trace logs)
    simple_logger::SimpleLogger::new().init().unwrap();
}
