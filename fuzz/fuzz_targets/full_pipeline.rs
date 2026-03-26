#![no_main]

use atmos::{compile_source, Session};
use libfuzzer_sys::fuzz_target;
use miette::NamedSource;

fuzz_target!(|data: &[u8]| {
    if let Ok(str) = std::str::from_utf8(data) {
        let session = Session::new(NamedSource::new("fuzz.at", str.to_string()));
        compile_source(&session);
    }
});
