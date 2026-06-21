use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pos {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Let { name: String, init: Expr, pos: Pos },
    Mut { name: String, init: Expr, pos: Pos },
    Assign { name: String, value: Expr, pos: Pos },
    Block(Vec<Stmt>),
    If {
        cond: Expr,
        then_blk: Box<Stmt>,
        else_blk: Option<Box<Stmt>>,
        pos: Pos,
    },
    Loop {
        label: Option<String>,
        body: Box<Stmt>,
        pos: Pos,
    },
    Break {
        label: Option<String>,
        value: Option<Expr>,
        pos: Pos,
    },
    Return { value: Option<Expr>, pos: Pos },
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Null(Pos),
    Bool(bool, Pos),
    Num(f64, Pos),
    Str(String, Pos),
    Ident(String, Pos),
    BinOp {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
        pos: Pos,
    },
    UnaryOp { op: UnaryOp, expr: Box<Expr>, pos: Pos },
    Call { callee: Box<Expr>, args: Vec<Expr>, pos: Pos },
    Func {
        params: Vec<String>,
        captures: Vec<String>,
        body: Box<Stmt>,
        pos: Pos,
    },
    Arrow {
        params: Vec<String>,
        body: Box<Expr>,
        is_block: bool,
        pos: Pos,
    },
    Pipe {
        left: Box<Expr>,
        right: Box<Expr>,
        has_placeholder: bool,
        pos: Pos,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add, Sub, Mul, Div,
    Eq, Neq, Lt, Gt, Lte, Gte,
    And, Or,
    Assign,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOp {
    Neg,
}

impl fmt::Display for Pos {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}
