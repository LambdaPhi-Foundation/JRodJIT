use crate::token::{Token, TokenType};

pub struct Lexer<'a> {
    source: &'a [u8],
    pos: usize,
    line: usize,
    col: usize,
}

impl<'a> Lexer<'a> {
    #[inline(always)]
    pub fn new(source: &'a str) -> Self {
        Self {
            source: source.as_bytes(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    #[inline(always)]
    fn peek(&self) -> u8 {
        if self.pos >= self.source.len() {
            b'\0'
        } else {
            self.source[self.pos]
        }
    }

    #[inline(always)]
    fn peek_next(&self) -> u8 {
        if self.pos + 1 >= self.source.len() {
            b'\0'
        } else {
            self.source[self.pos + 1]
        }
    }

    #[inline(always)]
    fn advance(&mut self) -> u8 {
        if self.pos >= self.source.len() {
            return b'\0';
        }
        let ch = self.source[self.pos];
        self.pos += 1;
        self.col += 1;
        if ch == b'\n' {
            self.line += 1;
            self.col = 1;
        }
        ch
    }

    #[inline(always)]
    fn skip_whitespace(&mut self) {
        while self.pos < self.source.len() {
            match self.source[self.pos] {
                b' ' | b'\t' | b'\r' => {
                    self.pos += 1;
                    self.col += 1;
                }
                b'\n' => {
                    self.pos += 1;
                    self.line += 1;
                    self.col = 1;
                }
                _ => break,
            }
        }
    }

    fn read_identifier(&mut self) -> String {
        let start = self.pos;
        while self.pos < self.source.len() {
            let ch = self.source[self.pos];
            if ch.is_ascii_alphanumeric() || ch == b'_' || ch == b'-' || ch == b'?' {
                self.pos += 1;
                self.col += 1;
            } else {
                break;
            }
        }
        // Safety: characters are valid ASCII
        unsafe { std::str::from_utf8_unchecked(&self.source[start..self.pos]) }.to_string()
    }

    fn read_number(&mut self, is_negative: bool) -> (String, bool) {
        let mut has_dot = false;
        let mut buf = String::with_capacity(16);
        if is_negative {
            buf.push('-');
        }

        while self.pos < self.source.len() {
            let ch = self.source[self.pos];
            if ch.is_ascii_digit() {
                buf.push(ch as char);
                self.advance();
            } else if ch == b'.' && !has_dot {
                // Check if followed by digit or whitespace/delimiter
                has_dot = true;
                buf.push('.');
                self.advance();
            } else {
                break;
            }
        }
        (buf, has_dot)
    }

    fn read_string(&mut self) -> Result<String, String> {
        self.advance(); // Skip opening quote
        let mut res = String::new();
        let mut escaping = false;

        while self.pos < self.source.len() {
            let ch = self.advance();
            if escaping {
                match ch {
                    b'n' => res.push('\n'),
                    b't' => res.push('\t'),
                    b'r' => res.push('\r'),
                    b'\\' => res.push('\\'),
                    b'"' => res.push('"'),
                    _ => res.push(ch as char),
                }
                escaping = false;
            } else if ch == b'\\' {
                escaping = true;
            } else if ch == b'"' {
                return Ok(res);
            } else {
                res.push(ch as char);
            }
        }
        Err(format!("Unterminated string literal at line {}", self.line))
    }

    fn read_comment(&mut self) -> String {
        self.advance(); // Skip '#'
        let start = self.pos;
        while self.pos < self.source.len() && self.source[self.pos] != b'\n' {
            self.pos += 1;
            self.col += 1;
        }
        let bytes = &self.source[start..self.pos];
        String::from_utf8_lossy(bytes).into_owned()
    }

    pub fn next_token(&mut self) -> Result<Token, String> {
        self.skip_whitespace();

        let line = self.line;
        let col = self.col;
        let ch = self.peek();

        if ch == b'\0' {
            return Ok(Token::new(TokenType::Eof, String::new(), line, col));
        }

        match ch {
            b'[' => {
                self.advance();
                Ok(Token::new(TokenType::LBracket, "[".to_string(), line, col))
            }
            b']' => {
                self.advance();
                Ok(Token::new(TokenType::RBracket, "]".to_string(), line, col))
            }
            b'(' => {
                self.advance();
                Ok(Token::new(TokenType::LParen, "(".to_string(), line, col))
            }
            b')' => {
                self.advance();
                Ok(Token::new(TokenType::RParen, ")".to_string(), line, col))
            }
            b'{' => {
                self.advance();
                Ok(Token::new(TokenType::LBrace, "{".to_string(), line, col))
            }
            b'}' => {
                self.advance();
                Ok(Token::new(TokenType::RBrace, "}".to_string(), line, col))
            }
            b':' => {
                self.advance();
                Ok(Token::new(TokenType::Colon, ":".to_string(), line, col))
            }
            b'|' => {
                self.advance();
                Ok(Token::new(TokenType::Pipe, "|".to_string(), line, col))
            }
            b',' => {
                self.advance();
                Ok(Token::new(TokenType::Comma, ",".to_string(), line, col))
            }
            b'.' => {
                self.advance();
                Ok(Token::new(TokenType::Period, ".".to_string(), line, col))
            }
            b'\'' => {
                self.advance();
                Ok(Token::new(TokenType::Quote, "'".to_string(), line, col))
            }
            b'#' => {
                let val = self.read_comment();
                Ok(Token::new(TokenType::Comment, val, line, col))
            }
            b'"' => {
                let val = self.read_string()?;
                Ok(Token::new(TokenType::String, val, line, col))
            }
            b'-' => {
                let next = self.peek_next();
                if next.is_ascii_digit() {
                    self.advance(); // consume '-'
                    let (val, has_dot) = self.read_number(true);
                    if has_dot {
                        Ok(Token::new(TokenType::Float, val, line, col))
                    } else {
                        Ok(Token::new(TokenType::Int, val, line, col))
                    }
                } else {
                    let val = self.read_identifier();
                    Ok(Token::new(TokenType::Ident, val, line, col))
                }
            }
            b'0'..=b'9' => {
                let (val, has_dot) = self.read_number(false);
                if has_dot {
                    Ok(Token::new(TokenType::Float, val, line, col))
                } else {
                    Ok(Token::new(TokenType::Int, val, line, col))
                }
            }
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                let val = self.read_identifier();
                let lower = val.to_ascii_lowercase();
                if lower == "true" || lower == "false" {
                    Ok(Token::new(TokenType::Bool, val, line, col))
                } else {
                    Ok(Token::new(TokenType::Ident, val, line, col))
                }
            }
            _ => Err(format!(
                "Unexpected character: '{}' at line {} col {}",
                ch as char, line, col
            )),
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::with_capacity(32);
        while self.peek() != b'\0' {
            let tok = self.next_token()?;
            tokens.push(tok);
        }
        tokens.push(Token::new(
            TokenType::Eof,
            String::new(),
            self.line,
            self.col,
        ));
        Ok(tokens)
    }
}

pub fn tokenize(source: &str) -> Result<Vec<Token>, String> {
    let mut lexer = Lexer::new(source);
    lexer.tokenize()
}
