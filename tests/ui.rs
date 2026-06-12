use atmos::test_runner;

#[test]
fn ui_tests() {
    let compiler = env!("CARGO_BIN_EXE_atmos");
    let mut test_runner = test_runner::TestRunner::new(compiler.into());
    test_runner.run_tests();
}
