use atmos::Lexer;
use std::fs;

fn main() {
    let content = fs::read_to_string("source.at").unwrap();

    let mut lexer = Lexer::new(&content);
    let tokens = lexer.tokenize();

    for token in &tokens {
        println!("{token}");
    }
}
