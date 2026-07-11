#![allow(clippy::result_large_err)]
pub mod ast_lowerer;
pub mod error;
pub mod extension;
pub mod lexer;
pub mod parser;
pub mod public;
pub mod resolver;
pub mod session;
pub mod type_checker;

pub use ast_lowerer::AstLowerer;
pub use lexer::Lexer;
pub use parser::Parser;
pub use resolver::Resolver;
pub use session::Session;

use crate::{
    session::session::{Output, OutputType},
    type_checker::TypeChecker,
};
pub use type_checker::TypeCollector;

pub fn compile_source(session: &Session) -> Output {
    let mut output = Output::default();

    let mut lexer = Lexer::new(session);
    let tokens = lexer.tokenize();

    if session.flags.contains(&OutputType::Tokens) {
        output.tokens = Some(tokens.clone());
    }

    let mut parser = Parser::new(session, tokens.clone());
    let ast = parser.parse_crate();

    if session.flags.contains(&OutputType::Ast) {
        output.ast = Some(ast.clone());
    }

    let mut resolver = Resolver::new(session, &ast);
    let defs = resolver.resolve();

    output.ast_to_def = defs.ast_to_def.clone();
    output.resolutions = defs.resolutions.clone();
    output.def_map = defs.definitions.iter().map(|(id, def)| (*id, def.kind)).collect();

    let mut ast_lowerer = AstLowerer::new(defs, &ast);
    let (hir, hir_nodes, def_to_hir) = ast_lowerer.lower();

    if session.flags.contains(&OutputType::Hir) {
        output.hir = Some(hir.clone());
    }

    let mut type_collector = TypeCollector::new(session, &hir_nodes, &def_to_hir);
    let collected_types = type_collector.collect_items();

    output.collected_types = collected_types.clone();

    let mut type_checker = TypeChecker::new(session, &hir, collected_types);
    type_checker.check();

    output
}
