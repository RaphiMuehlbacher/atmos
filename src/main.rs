use atmos::{compile_source, Session};
use miette::NamedSource;
use std::fs;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: atmos <file>");
        std::process::exit(1);
    }

    let file_name = &args[1];
    let content = fs::read_to_string(file_name).unwrap();

    let session = Session::new(NamedSource::new(file_name.clone(), content));
    compile_source(&session);
    session.emit_all();
}
