use xenith::lexer;
use xenith::position::Position;
use xenith::tokens::{Token, TokenType};

fn main() {
    println!("Hello, Xenith!");

    // Dummy usage to satisfy the compiler
    let pos = Position::new(0, 1, 0, "example.xen", "let x = 42");
    let tok = Token::new(
        TokenType::Int,
        Some("42".to_string()),
        Some(pos.clone()),
        None,
    );

    println!("Token: {:?}", tok);

    // Dummy call to lexer just to silence unused import
    let _ = lexer::dummy();
}
