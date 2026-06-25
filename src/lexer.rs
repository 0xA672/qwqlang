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
    Continue(Pos),
    While(Pos),
    For(Pos),
    In(Pos),
    Match(Pos),
    Enum(Pos),
    Struct(Pos),
    Throw(Pos),
    Try(Pos),
    Catch(Pos),
    Finally(Pos),
    Ok(Pos),
    Err(Pos),
    Some(Pos),
    None(Pos),
    Ident(String, Pos),
    Label(String, Pos),
    Num(f64, Pos),
    Str(String, Pos),
    TemplateStr(String, Pos),
    Dollar(Pos),
    Spread(Pos),
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
    Ref(Pos),
    Dot(Pos),
    Colon(Pos),
    DoubleColon(Pos),
    FatArrow(Pos),
    Question(Pos),
    Eof(Pos),
}

impl Tok {
    pub fn describe(&self) -> String {
        match self {
            Tok::Let(_) => "keyword 'let'".to_string(),
            Tok::Mut(_) => "keyword 'mut'".to_string(),
            Tok::Fn(_) => "keyword 'fn'".to_string(),
            Tok::If(_) => "keyword 'if'".to_string(),
            Tok::Else(_) => "keyword 'else'".to_string(),
            Tok::True(_) => "'true'".to_string(),
            Tok::False(_) => "'false'".to_string(),
            Tok::Null(_) => "'null'".to_string(),
            Tok::Return(_) => "keyword 'return'".to_string(),
            Tok::And(_) => "keyword 'and'".to_string(),
            Tok::Or(_) => "keyword 'or'".to_string(),
            Tok::Loop(_) => "keyword 'loop'".to_string(),
            Tok::Break(_) => "keyword 'break'".to_string(),
            Tok::Continue(_) => "keyword 'continue'".to_string(),
            Tok::While(_) => "keyword 'while'".to_string(),
            Tok::For(_) => "keyword 'for'".to_string(),
            Tok::In(_) => "keyword 'in'".to_string(),
            Tok::Match(_) => "keyword 'match'".to_string(),
            Tok::Enum(_) => "keyword 'enum'".to_string(),
            Tok::Struct(_) => "keyword 'struct'".to_string(),
            Tok::Throw(_) => "keyword 'throw'".to_string(),
            Tok::Try(_) => "keyword 'try'".to_string(),
            Tok::Catch(_) => "keyword 'catch'".to_string(),
            Tok::Finally(_) => "keyword 'finally'".to_string(),
            Tok::Ok(_) => "keyword 'Ok'".to_string(),
            Tok::Err(_) => "keyword 'Err'".to_string(),
            Tok::Some(_) => "keyword 'Some'".to_string(),
            Tok::None(_) => "keyword 'None'".to_string(),
            Tok::Ident(s, _) => format!("identifier '{}'", s),
            Tok::Label(s, _) => format!("label '{}'", s),
            Tok::Num(n, _) => format!("number {}", n),
            Tok::Str(s, _) => format!("string \"{}\"", s),
            Tok::TemplateStr(s, _) => format!("template string \"{}\"", s),
            Tok::Dollar(_) => "'$'".to_string(),
            Tok::Spread(_) => "'..'".to_string(),
            Tok::Add(_) => "'+'".to_string(),
            Tok::Sub(_) => "'-'".to_string(),
            Tok::Mul(_) => "'*'".to_string(),
            Tok::Div(_) => "'/'".to_string(),
            Tok::Eq(_) => "'=='".to_string(),
            Tok::Neq(_) => "'!='".to_string(),
            Tok::Lt(_) => "'<'".to_string(),
            Tok::Gt(_) => "'>'".to_string(),
            Tok::Lte(_) => "'<='".to_string(),
            Tok::Gte(_) => "'>='".to_string(),
            Tok::Assign(_) => "'='".to_string(),
            Tok::Pipe(_) => "'|>'".to_string(),
            Tok::PipeSingle(_) => "'|>'".to_string(),
            Tok::LParen(_) => "'('".to_string(),
            Tok::RParen(_) => "')'".to_string(),
            Tok::LBrace(_) => "'{'".to_string(),
            Tok::RBrace(_) => "'}'".to_string(),
            Tok::Semicolon(_) => "';'".to_string(),
            Tok::Comma(_) => "','".to_string(),
            Tok::LBracket(_) => "'['".to_string(),
            Tok::RBracket(_) => "']'".to_string(),
            Tok::Ref(_) => "'&'".to_string(),
            Tok::Dot(_) => "'.'".to_string(),
            Tok::Colon(_) => "':'".to_string(),
            Tok::DoubleColon(_) => "'::'".to_string(),
            Tok::FatArrow(_) => "'->'".to_string(),
            Tok::Question(_) => "'?'".to_string(),
            Tok::Eof(_) => "end of file".to_string(),
        }
    }
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
                        '&' => {
                            self.col += 1;
                            return Tok::Ref(pos);
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
                            } else if let Some(&'>') = self.chars.peek() {
                                self.col += 1;
                                self.chars.next();
                                return Tok::FatArrow(pos);
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
                        '?' => {
                            self.col += 1;
                            return Tok::Question(pos);
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
                        '.' => {
                            self.col += 1;
                            if let Some(&'.') = self.chars.peek() {
                                self.col += 1;
                                self.chars.next();
                                if let Some(&'.') = self.chars.peek() {
                                    self.col += 1;
                                    self.chars.next();
                                }
                            }
                            return Tok::Dot(pos);
                        }
                        ':' => {
                            self.col += 1;
                            if let Some(&':') = self.chars.peek() {
                                self.col += 1;
                                self.chars.next();
                                return Tok::DoubleColon(pos);
                            }
                            return Tok::Colon(pos);
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
                        '`' => {
                            self.col += 1;
                            let s = self.template_string();
                            return Tok::TemplateStr(s, pos);
                        }
                        '$' => {
                            self.col += 1;
                            return Tok::Dollar(pos);
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
                            self.col += 1;
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
                                "continue" => Tok::Continue(pos),
                                "while" => Tok::While(pos),
                                "for" => Tok::For(pos),
                                "in" => Tok::In(pos),
                                "match" => Tok::Match(pos),
                                "enum" => Tok::Enum(pos),
                                "struct" => Tok::Struct(pos),
                                "throw" => Tok::Throw(pos),
                                "try" => Tok::Try(pos),
                                "catch" => Tok::Catch(pos),
                                "finally" => Tok::Finally(pos),
                                "Ok" => Tok::Ok(pos),
                                "Err" => Tok::Err(pos),
                                "Some" => Tok::Some(pos),
                                "None" => Tok::None(pos),
                                _ => Tok::Ident(ident, pos),
                            };
                            return tok;
                        }
                        c if c.is_ascii_digit() => {
                            self.col += 1;
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
        let first = self.chars.next().unwrap_or('_');
        self.col += 1;
        self.identifier_start(first)
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

    fn template_string(&mut self) -> String {
        let mut s = String::new();
        while let Some(c) = self.chars.next() {
            match c {
                '`' => {
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
                            '`' => s.push('`'),
                            '\\' => s.push('\\'),
                            '$' => s.push('$'),
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
