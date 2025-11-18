use atmos::{Lexer, Parser, Session};
use miette::{GraphicalReportHandler, NamedSource};
use std::fs;

fn main() {
    miette::set_hook(Box::new(|_| Box::new(GraphicalReportHandler::new()))).unwrap();
    let file_name = "example/source.at";
    let content = fs::read_to_string(file_name).unwrap();
    let content = format!("{content} ");

    let session = Session::new(NamedSource::new(file_name, content));
    let mut lexer = Lexer::new(&session);
    let tokens = lexer.tokenize();

    let mut parser = Parser::new(&session, tokens);
    let ast = parser.parse_crate();

    if session.error_handler.borrow().error_count() == 0 {
        dbg!(&ast);
    }

    session.emit_all();
}
