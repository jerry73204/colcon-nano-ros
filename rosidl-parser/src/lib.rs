pub mod ast;
pub mod lexer;
pub mod parser;

pub use ast::{Action, ActionSpec, Constant, Field, FieldType, Message, PrimitiveType, Service};
pub use lexer::{Token, TokenKind};
pub use parser::{parse_action, parse_message, parse_service, ParseError};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_message() {
        let input = "int32 x\nfloat64 y\n";
        let result = parse_message(input);
        assert!(result.is_ok());
        let msg = result.unwrap();
        assert_eq!(msg.fields.len(), 2);
    }
}
