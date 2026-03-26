pub mod ast_lowerer;
pub mod error;
pub mod extension;
pub mod lexer;
pub mod parser;
pub mod resolver;
pub mod session;
pub mod type_checker;

pub use ast_lowerer::AstLowerer;
pub use lexer::Lexer;
pub use parser::Parser;
pub use resolver::Resolver;
pub use session::Session;

pub use type_checker::TypeChecker;

pub fn compile_source(session: &Session) {
    let mut lexer = Lexer::new(session);
    let tokens = lexer.tokenize();

    let mut parser = Parser::new(session, tokens);
    let ast = parser.parse_crate();

    let mut resolver = Resolver::new(session, &ast);
    let defs = resolver.resolve();

    let mut ast_lowerer = AstLowerer::new(defs, &ast);
    let hir = ast_lowerer.lower();

    let mut type_checker = TypeChecker::new(session, &hir);
}
