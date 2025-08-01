#[derive(Debug)]
pub(crate) enum TokenKind {
    Equal,
    LeftBrace,
    RightBrace,
    EndOfLine,
}

#[derive(Debug)]
pub(crate) struct Token<'a> {
    pub kind: TokenKind,
    pub origin: &'a str,
}

pub(crate) struct Lexer<'a> {
    rest: &'a str,
}

impl<'a> Lexer<'a> {
    pub fn new(buffer: &'a str) -> Lexer<'a> {
        Lexer { rest: buffer }
    }
}

const NEW_LINE: char = 0x0a as char;

impl<'a> Iterator for Lexer<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut last = self.rest;
        let mut words = 0;
        for c in self.rest.chars() {
            self.rest = &self.rest[c.len_utf8()..];
            let kind = match c {
                '=' => TokenKind::Equal,
                '{' => TokenKind::LeftBrace,
                '}' => TokenKind::RightBrace,
                NEW_LINE => TokenKind::EndOfLine,
                _ => {
                    if c.is_whitespace() {
                        if words == 0 {
                            last = &last[c.len_utf8()..];
                        }
                    } else {
                        words += c.len_utf8();
                    }
                    continue;
                }
            };
            let token = Token {
                origin: &last[..words],
                kind,
            };
            return Some(token);
        }
        None
    }
}
