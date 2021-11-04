pub mod tests;
use tests::TestsSuite;

fn setup() {
    println!("----------------- Setup -----------------")
}

fn teardown() {
    println!("----------------- Teardown -----------------")
}
fn main() {
    // Setup test env
    setup();

    // Run test cases
    for t in inventory::iter::<TestsSuite> {
        (t.test_sth)()
    }

    // Teardown test env
    teardown();
}
