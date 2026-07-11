use atmos::{Session, compile_source, public::PublicOutput, session::session::OutputType};
use clap::Parser;
use miette::{GraphicalReportHandler, NamedSource};
use std::{fs, path::PathBuf, process};

#[derive(Parser)]
#[command(name = "atmos", about = "Atmos compiler")]
struct CliOptions {
    #[arg(long, value_enum, value_delimiter = ',')]
    emit: Vec<OutputType>,

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

    let session = Session::new(
        NamedSource::new(file_name, content),
        options.emit.iter().cloned().collect(),
    );

    let output = compile_source(&session);
    let public_output = PublicOutput::from(output);

    let json = serde_json::to_string_pretty(&public_output).unwrap();
    if !options.emit.is_empty() {
        println!("{json}");
    }
}
