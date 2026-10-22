use crate::ast::{Expr, ExprKind};
use crate::token::{Token, TokenType};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    #[inline(always)]
    fn peek(&self) -> &Token {
        if self.pos >= self.tokens.len() {
            &self.tokens[self.tokens.len() - 1]
        } else {
            &self.tokens[self.pos]
        }
    }

    #[inline(always)]
    fn consume(&mut self) -> Token {
        let tok = self.peek().clone();
        self.pos += 1;
        tok
    }

    fn skip_comments_and_commas(&mut self) {
        while self.pos < self.tokens.len() {
            let k = &self.tokens[self.pos].kind;
            if *k == TokenType::Comment || *k == TokenType::Comma {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn expect(&mut self, kind: TokenType, msg: &str) -> Result<Token, String> {
        let tok = self.consume();
        if tok.kind != kind {
            Err(format!(
                "Parser Error: {} (Expected {:?}, got {:?} at line {})",
                msg, kind, tok.kind, tok.line
            ))
        } else {
            Ok(tok)
        }
    }

    pub fn parse_expr(&mut self) -> Result<Expr, String> {
        self.skip_comments_and_commas();
        let tok = self.peek();

        match &tok.kind {
            TokenType::Quote => {
                let start_tok = self.consume();
                let inner = self.parse_expr()?;
                Ok(Expr::new(
                    ExprKind::Quote(Box::new(inner)),
                    start_tok.line,
                    start_tok.column,
                ))
            }

            TokenType::LBracket => {
                let start_tok = self.consume();
                let mut elements = Vec::new();

                loop {
                    self.skip_comments_and_commas();
                    if self.peek().kind == TokenType::RBracket {
                        break;
                    }
                    if self.peek().kind == TokenType::Eof {
                        return Err(format!(
                            "Unexpected EOF while parsing list expression starting at line {}",
                            start_tok.line
                        ));
                    }
                    elements.push(self.parse_expr()?);
                }

                self.expect(TokenType::RBracket, "Expected closing bracket ']'")?;
                Ok(Expr::new(
                    ExprKind::List(elements),
                    start_tok.line,
                    start_tok.column,
                ))
            }

            TokenType::LBrace => {
                let start_tok = self.consume();
                let mut pairs = Vec::new();

                loop {
                    self.skip_comments_and_commas();
                    if self.peek().kind == TokenType::RBrace {
                        break;
                    }
                    if self.peek().kind == TokenType::Eof {
                        return Err(format!(
                            "Unexpected EOF while parsing object expression starting at line {}",
                            start_tok.line
                        ));
                    }

                    let key_expr = self.parse_expr()?;
                    self.skip_comments_and_commas();

                    // Optional colon between key and value
                    if self.peek().kind == TokenType::Colon {
                        self.consume();
                        self.skip_comments_and_commas();
                    }

                    let val_expr = self.parse_expr()?;
                    pairs.push((key_expr, val_expr));
                }

                self.expect(TokenType::RBrace, "Expected closing brace '}'")?;
                Ok(Expr::new(
                    ExprKind::Object(pairs),
                    start_tok.line,
                    start_tok.column,
                ))
            }

            TokenType::Int => {
                let t = self.consume();
                let val = t.value.parse::<i64>().map_err(|e| {
                    format!("Invalid integer '{}' at line {}: {}", t.value, t.line, e)
                })?;
                Ok(Expr::new(ExprKind::Int(val), t.line, t.column))
            }

            TokenType::Float => {
                let t = self.consume();
                let val = t.value.parse::<f64>().map_err(|e| {
                    format!("Invalid float '{}' at line {}: {}", t.value, t.line, e)
                })?;
                Ok(Expr::new(ExprKind::Float(val), t.line, t.column))
            }

            TokenType::String => {
                let t = self.consume();
                Ok(Expr::new(ExprKind::String(t.value), t.line, t.column))
            }

            TokenType::Bool => {
                let t = self.consume();
                let val = t.value.eq_ignore_ascii_case("true");
                Ok(Expr::new(ExprKind::Bool(val), t.line, t.column))
            }

            TokenType::Ident => {
                let t = self.consume();
                self.skip_comments_and_commas();
                if self.peek().kind == TokenType::Pipe {
                    self.consume(); // consume '|'
                    self.skip_comments_and_commas();
                    let type_tok = self.expect(TokenType::Ident, "Expected type name after '|'")?;
                    Ok(Expr::new(
                        ExprKind::TypedId(t.value, type_tok.value),
                        t.line,
                        t.column,
                    ))
                } else {
                    Ok(Expr::new(ExprKind::Id(t.value), t.line, t.column))
                }
            }

            TokenType::Colon => {
                let t = self.consume();
                Err(format!(
                    "Unexpected colon ':' at line {}. Type annotations must use pipe '|' (e.g. x|int)",
                    t.line
                ))
            }

            TokenType::Pipe => {
                let t = self.consume();
                Err(format!(
                    "Unexpected pipe '|' at line {}. Pipe is used for type annotations on identifiers (e.g. x|int)",
                    t.line
                ))
            }

            TokenType::Comment => {
                let t = self.consume();
                Ok(Expr::new(ExprKind::Comment(t.value), t.line, t.column))
            }

            TokenType::Eof => Err(format!(
                "Unexpected EOF while parsing expression at line {}",
                tok.line
            )),

            _ => {
                let t = self.consume();
                Err(format!(
                    "Unexpected token in expression: '{}' at line {}",
                    t.value, t.line
                ))
            }
        }
    }

    pub fn parse(&mut self) -> Result<Vec<Expr>, String> {
        let mut program = Vec::new();
        while self.peek().kind != TokenType::Eof {
            self.skip_comments_and_commas();
            if self.peek().kind == TokenType::Eof {
                break;
            }
            let expr = self.parse_expr()?;
            // Only add non-comment expressions to program
            if !matches!(expr.kind, ExprKind::Comment(_)) {
                program.push(expr);
            }
        }
        Ok(program)
    }
}

pub fn parse(tokens: Vec<Token>) -> Result<Vec<Expr>, String> {
    let mut parser = Parser::new(tokens);
    parser.parse()
}
