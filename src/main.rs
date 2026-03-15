use atmos::ast_lowerer::AstLowerer;
use atmos::{Lexer, Parser, Resolver, Session, TypeChecker};
use miette::{GraphicalReportHandler, NamedSource};
use std::fs;

fn main() {
    miette::set_hook(Box::new(|_| Box::new(GraphicalReportHandler::new()))).unwrap();
    let file_name = "example/source.at";
    let content = fs::read_to_string(file_name).unwrap();

    let session = Session::new(NamedSource::new(file_name, content));
    let mut lexer = Lexer::new(&session);
    let tokens = lexer.tokenize();

    let mut parser = Parser::new(&session, tokens);
    let ast = parser.parse_crate();

    let mut resolver = Resolver::new(&session, &ast);
    let defs = resolver.resolve();

    let mut ast_lowerer = AstLowerer::new(defs, &ast);
    let hir = ast_lowerer.lower();

    let mut type_checker = TypeChecker::new(&session, &hir);

    session.emit_all();
}
