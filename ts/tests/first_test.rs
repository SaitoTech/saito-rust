use super::TestsSuite;

fn first_test() {
    println!("Running the first test")
}

inventory::submit!(TestsSuite {
    name: "first",
    test_sth: first_test
});
