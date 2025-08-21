use atmos::{Lexer, Session};
use miette::{GraphicalReportHandler, NamedSource};
use std::fs;

fn main() {
    miette::set_hook(Box::new(|_| Box::new(GraphicalReportHandler::new()))).unwrap();
    let file_name = "example/source.at";
    let content = fs::read_to_string(file_name).unwrap();

    let session = Session::new(NamedSource::new(file_name, content));
    let mut lexer = Lexer::new(&session);
    let tokens = lexer.tokenize();

    session.emit_all();
    for token in &tokens {
        println!("{token:?}");
    }
}
