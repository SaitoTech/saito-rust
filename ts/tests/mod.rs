pub mod first_test;

#[derive(Debug)]
pub struct TestsSuite {
    pub name: &'static str,
    pub test_sth: fn(),
}

inventory::collect!(TestsSuite);
