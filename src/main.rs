use atmos::{AstLowerer, Lexer, Parser as AtmosParser, Resolver, Session, compile_source};
use clap::{Parser, ValueEnum};
use miette::{GraphicalReportHandler, NamedSource};
use std::{fs, path::PathBuf, process};

#[derive(Clone, Copy, ValueEnum)]
enum EmitMode {
    Result,
    Diagnostics,
    Tokens,
    Ast,
    Hir,
}

#[derive(Parser)]
#[command(name = "atmos", about = "Atmos compiler")]
struct CliOptions {
    #[arg(long, value_enum, default_value_t = EmitMode::Result)]
    emit: EmitMode,

    #[arg(default_value = "example/source.at")]
    file_name: PathBuf,
}

fn main() {
    miette::set_hook(Box::new(|_| Box::new(GraphicalReportHandler::new()))).unwrap();

    let options = CliOptions::parse();
    let content = fs::read_to_string(&options.file_name).unwrap_or_else(|error| {
        eprintln!("failed to read `{}`: {error}", options.file_name.display());
        process::exit(1);
    });
    let file_name = options.file_name.display().to_string();

    let session = Session::new(NamedSource::new(file_name, content));

    match options.emit {
        EmitMode::Result => emit_result(&session),
        EmitMode::Diagnostics => emit_diagnostics(&session),
        EmitMode::Tokens => emit_tokens(&session),
        EmitMode::Ast => emit_ast(&session),
        EmitMode::Hir => emit_hir(&session),
    }
}

fn emit_result(session: &Session) {
    compile_source(session);
    if error_count(session) == 0 {
        println!("Compilation finished successfully.");
    } else {
        session.emit_all();
    }
}

fn emit_diagnostics(session: &Session) {
    compile_source(session);
    if error_count(session) == 0 {
        println!("No diagnostics.");
    } else {
        session.emit_all();
    }
}

fn emit_tokens(session: &Session) {
    let mut lexer = Lexer::new(session);
    let tokens = lexer.tokenize();
    println!("{tokens:#?}");
    session.emit_all();
}

fn emit_ast(session: &Session) {
    let mut lexer = Lexer::new(session);
    let tokens = lexer.tokenize();

    let mut parser = AtmosParser::new(session, tokens);
    let ast = parser.parse_crate();

    println!("{ast:#?}");
    session.emit_all();
}

fn emit_hir(session: &Session) {
    let mut lexer = Lexer::new(session);
    let tokens = lexer.tokenize();

    let mut parser = AtmosParser::new(session, tokens);
    let ast = parser.parse_crate();

    let mut resolver = Resolver::new(session, &ast);
    let defs = resolver.resolve();

    let mut ast_lowerer = AstLowerer::new(defs, &ast);
    let (hir, hir_nodes, def_to_hir) = ast_lowerer.lower();

    println!("HIR:\n{hir:#?}\n");
    println!("HIR nodes:\n{hir_nodes:#?}\n");
    println!("Definitions to HIR:\n{def_to_hir:#?}");
    session.emit_all();
}

fn error_count(session: &Session) -> usize {
    session.error_handler.borrow().error_count()
}
