pub mod ast_lowerer;
pub mod error;
pub mod extension;
pub mod lexer;
pub mod parser;
pub mod resolver;
pub mod session;
pub mod test_runner;
pub mod type_checker;

pub use ast_lowerer::AstLowerer;
pub use lexer::Lexer;
pub use parser::Parser;
pub use resolver::Resolver;
pub use session::Session;

pub use type_checker::TypeCollector;

pub fn compile_source(session: &Session) {
    let mut lexer = Lexer::new(session);
    let tokens = lexer.tokenize();

    let mut parser = Parser::new(session, tokens);
    let ast = parser.parse_crate();

    let mut resolver = Resolver::new(session, &ast);
    let defs = resolver.resolve();

    let mut ast_lowerer = AstLowerer::new(defs, &ast);
    let (hir, hir_nodes, def_to_hir) = ast_lowerer.lower();

    let mut type_collector = TypeCollector::new(session, &hir, &hir_nodes, &def_to_hir);
    type_collector.collect_items();
    // dbg!(type_collector.items);
}
