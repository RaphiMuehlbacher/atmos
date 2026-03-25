use atmos::{compile_source, Session};
use miette::{GraphicalReportHandler, NamedSource};
use std::fs;

fn main() {
    miette::set_hook(Box::new(|_| Box::new(GraphicalReportHandler::new()))).unwrap();
    let file_name = "example/source.at";
    let content = fs::read_to_string(file_name).unwrap();

    let session = Session::new(NamedSource::new(file_name, content));
    compile_source(session);
}
