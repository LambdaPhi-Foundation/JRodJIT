#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // Delimiters
    LBracket, // [
    RBracket, // ]
    LParen,   // (
    RParen,   // )
    LBrace,   // {
    RBrace,   // }

    // Operators & Punctuation
    Colon,  // :
    Pipe,   // |
    Comma,  // ,
    Period, // .
    Quote,  // '

    // Literals & Identifiers
    Ident,
    String,
    Int,
    Float,
    Bool,
    Comment,
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenType,
    pub value: String,
    pub line: usize,
    pub column: usize,
}

impl Token {
    #[inline(always)]
    pub fn new(kind: TokenType, value: String, line: usize, column: usize) -> Self {
        Self {
            kind,
            value,
            line,
            column,
        }
    }
}
