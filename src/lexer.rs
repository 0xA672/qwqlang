use crate::ast::Pos;
use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Let(Pos),
    Mut(Pos),
    Fn(Pos),
    If(Pos),
    Else(Pos),
    True(Pos),
    False(Pos),
    Null(Pos),
    Return(Pos),
    And(Pos),
    Or(Pos),
    Loop(Pos),
    Break(Pos),
    Ident(String, Pos),
    Label(String, Pos),
    Num(f64, Pos),
    Str(String, Pos),
    Add(Pos),
    Sub(Pos),
    Mul(Pos),
    Div(Pos),
    Eq(Pos),
    Neq(Pos),
    Lt(Pos),
    Gt(Pos),
    Lte(Pos),
    Gte(Pos),
    Assign(Pos),
    Pipe(Pos),
    PipeSingle(Pos),
    LParen(Pos),
    RParen(Pos),
    LBrace(Pos),
    RBrace(Pos),
    Semicolon(Pos),
    Comma(Pos),
    LBracket(Pos),
    RBracket(Pos),
    Eof(Pos),
}

#[derive(Debug)]
pub struct Lex<'a> {
    chars: Peekable<Chars<'a>>,
    line: usize,
    col: usize,
}

impl<'a> Lex<'a> {
    pub fn new(input: &'a str) -> Self {
        Lex {
            chars: input.chars().peekable(),
            line: 1,
            col: 1,
        }
    }

    pub fn next(&mut self) -> Tok {
        loop {
            match self.chars.next() {
                None => return Tok::Eof(self.pos()),
                Some(c) => {
                    let pos = self.pos();
                    match c {
                        ' ' | '\t' => self.col += 1,
                        '\n' => {
                            self.line += 1;
                            self.col = 1;
                        }
                        '\r' => {}
                        '+' => {
                            self.col += 1;
                            return Tok::Add(pos);
                        }
                        '-' => {
                            self.col += 1;
                            return Tok::Sub(pos);
                        }
                        '*' => {
                            self.col += 1;
                            return Tok::Mul(pos);
                        }
                        '/' => {
                            self.col += 1;
                            if let Some(&'/') = self.chars.peek() {
                                self.skip_line();
                            } else if let Some(&'*') = self.chars.peek() {
                                self.skip_block();
                            } else {
                                return Tok::Div(pos);
                            }
                        }
                        '=' => {
                            self.col += 1;
                            if let Some(&'=') = self.chars.peek() {
                                self.col += 1;
                                self.chars.next();
                                return Tok::Eq(pos);
                            }
                            return Tok::Assign(pos);
                        }
                        '!' => {
                            self.col += 1;
                            if let Some(&'=') = self.chars.peek() {
                                self.col += 1;
                                self.chars.next();
                                return Tok::Neq(pos);
                            }
                            return self.error(pos, "unexpected '!'");
                        }
                        '<' => {
                            self.col += 1;
                            if let Some(&'=') = self.chars.peek() {
                                self.col += 1;
                                self.chars.next();
                                return Tok::Lte(pos);
                            }
                            return Tok::Lt(pos);
                        }
                        '>' => {
                            self.col += 1;
                            if let Some(&'=') = self.chars.peek() {
                                self.col += 1;
                                self.chars.next();
                                return Tok::Gte(pos);
                            } else if let Some(&'|') = self.chars.peek() {
                                self.col += 1;
                                self.chars.next();
                                return Tok::Pipe(pos);
                            }
                            return Tok::Gt(pos);
                        }
                        '|' => {
                            self.col += 1;
                            if let Some(&'>') = self.chars.peek() {
                                self.col += 1;
                                self.chars.next();
                                return Tok::Pipe(pos);
                            }
                            return Tok::PipeSingle(pos);
                        }
                        '(' => {
                            self.col += 1;
                            return Tok::LParen(pos);
                        }
                        ')' => {
                            self.col += 1;
                            return Tok::RParen(pos);
                        }
                        '{' => {
                            self.col += 1;
                            return Tok::LBrace(pos);
                        }
                        '}' => {
                            self.col += 1;
                            return Tok::RBrace(pos);
                        }
                        ';' => {
                            self.col += 1;
                            return Tok::Semicolon(pos);
                        }
                        ',' => {
                            self.col += 1;
                            return Tok::Comma(pos);
                        }
                        '[' => {
                            self.col += 1;
                            return Tok::LBracket(pos);
                        }
                        ']' => {
                            self.col += 1;
                            return Tok::RBracket(pos);
                        }
                        '\'' => {
                            self.col += 1;
                            let label = self.identifier();
                            return Tok::Label(label, pos);
                        }
                        '"' => {
                            self.col += 1;
                            let s = self.string();
                            return Tok::Str(s, pos);
                        }
                        '#' => {
                            self.col += 1;
                            if let Some(&'?') = self.chars.peek() {
                                self.col += 1;
                                self.chars.next();
                                self.skip_line();
                            } else {
                                return self.error(pos, "unexpected '#'");
                            }
                        }
                        c if c.is_alphabetic() || c == '_' => {
                            let ident = self.identifier_start(c);
                            let tok = match ident.as_str() {
                                "let" => Tok::Let(pos),
                                "mut" => Tok::Mut(pos),
                                "fn" => Tok::Fn(pos),
                                "if" => Tok::If(pos),
                                "else" => Tok::Else(pos),
                                "true" => Tok::True(pos),
                                "false" => Tok::False(pos),
                                "null" => Tok::Null(pos),
                                "return" => Tok::Return(pos),
                                "and" => Tok::And(pos),
                                "or" => Tok::Or(pos),
                                "loop" => Tok::Loop(pos),
                                "break" => Tok::Break(pos),
                                _ => Tok::Ident(ident, pos),
                            };
                            return tok;
                        }
                        c if c.is_ascii_digit() => {
                            let num = self.number(c);
                            return Tok::Num(num, pos);
                        }
                        c => return self.error(pos, &format!("unexpected character '{}'", c)),
                    }
                }
            }
        }
    }

    fn pos(&self) -> Pos {
        Pos {
            line: self.line,
            col: self.col,
        }
    }

    fn skip_line(&mut self) {
        while let Some(&c) = self.chars.peek() {
            if c == '\n' {
                self.chars.next();
                self.line += 1;
                self.col = 1;
                break;
            }
            self.chars.next();
            self.col += 1;
        }
    }

    fn skip_block(&mut self) {
        self.col += 1;
        self.chars.next();
        let mut depth = 1;
        while let Some(c) = self.chars.next() {
            match c {
                '*' if self.chars.peek() == Some(&'/') => {
                    depth -= 1;
                    self.col += 2;
                    self.chars.next();
                    if depth == 0 {
                        break;
                    }
                }
                '/' if self.chars.peek() == Some(&'*') => {
                    depth += 1;
                    self.col += 2;
                    self.chars.next();
                }
                '\n' => {
                    self.line += 1;
                    self.col = 1;
                }
                _ => self.col += 1,
            }
        }
    }

    fn identifier_start(&mut self, first: char) -> String {
        let mut s = String::new();
        s.push(first);
        while let Some(&c) = self.chars.peek() {
            if c.is_alphanumeric() || c == '_' {
                s.push(c);
                self.chars.next();
                self.col += 1;
            } else {
                break;
            }
        }
        s
    }

    fn identifier(&mut self) -> String {
        self.identifier_start('_')
    }

    fn string(&mut self) -> String {
        let mut s = String::new();
        while let Some(c) = self.chars.next() {
            match c {
                '"' => {
                    self.col += 1;
                    break;
                }
                '\\' => {
                    self.col += 1;
                    if let Some(nc) = self.chars.next() {
                        self.col += 1;
                        match nc {
                            'n' => s.push('\n'),
                            'r' => s.push('\r'),
                            't' => s.push('\t'),
                            '"' => s.push('"'),
                            '\\' => s.push('\\'),
                            _ => s.push(nc),
                        }
                    }
                }
                '\n' => {
                    self.line += 1;
                    self.col = 1;
                    s.push('\n');
                }
                _ => {
                    s.push(c);
                    self.col += 1;
                }
            }
        }
        s
    }

    fn number(&mut self, first: char) -> f64 {
        let mut s = String::new();
        s.push(first);
        let mut has_dot = false;
        while let Some(&c) = self.chars.peek() {
            if c.is_ascii_digit() {
                s.push(c);
                self.chars.next();
                self.col += 1;
            } else if c == '.' && !has_dot {
                s.push(c);
                has_dot = true;
                self.chars.next();
                self.col += 1;
            } else {
                break;
            }
        }
        s.parse().unwrap_or(0.0)
    }

    fn error(&mut self, pos: Pos, msg: &str) -> Tok {
        eprintln!("{}:{}: error: {}", pos.line, pos.col, msg);
        self.skip_line();
        Tok::Eof(pos)
    }
}
