use std::fs;
use std::path::PathBuf;

#[derive(Debug, PartialEq, Clone)]
struct TestCase {
    source: PathBuf,
    stderr: Option<PathBuf>,
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

            self.tests.push(TestCase { source, stderr })
        }

        println!("{:?}", self.tests);
    }
}
