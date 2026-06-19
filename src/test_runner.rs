use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, PartialEq, Clone)]
struct TestCase {
    source: PathBuf,
    stdout: Option<PathBuf>,
}

pub struct TestRunner {
    compiler_path: PathBuf,
    tests: Vec<TestCase>,
}

impl TestRunner {
    pub fn new(compiler_path: PathBuf) -> Self {
        Self {
            compiler_path,
            tests: vec![],
        }
    }

    pub fn run_tests(&mut self) {
        let tests_dir = PathBuf::from("tests/ui");
        for file in fs::read_dir(tests_dir).unwrap() {
            let source = file.unwrap().path();

            if source.extension().unwrap() != "at" {
                continue;
            }
            let stderr_path = source.with_extension("stderr");
            let stderr = stderr_path.exists().then_some(stderr_path);

            self.tests.push(TestCase { source, stdout: stderr })
        }

        let mut failures = 0;
        for test in &self.tests {
            let atmos = Command::new(&self.compiler_path)
                .args([test.source.as_os_str()])
                .output()
                .unwrap();

            let stdout = String::from_utf8(atmos.stdout).unwrap();
            let expected_stdout = fs::read_to_string(test.stdout.clone().unwrap()).unwrap();

            let expected_stdout = normalize(expected_stdout);
            let stdout = normalize(stdout);
            if stdout != expected_stdout {
                failures += 1;
                eprintln!("ui test failed: {}", test.source.display());
                eprintln!("expected:\n{expected_stdout}\n\nactual:\n{stdout}");
            }
        }
        assert_eq!(failures, 0, "{failures} UI test(s) failed")
    }
}

fn normalize(s: String) -> String {
    s.replace("\r\n", "\n").trim_end().to_string()
}
