use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pos {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Let {
        pattern: DestructPattern,
        init: Expr,
        pos: Pos,
    },
    Mut {
        pattern: DestructPattern,
        init: Expr,
        pos: Pos,
    },
    Assign {
        target: AssignTarget,
        value: Expr,
        pos: Pos,
    },
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
    While {
        cond: Expr,
        body: Box<Stmt>,
        pos: Pos,
    },
    For {
        init: Option<Box<Stmt>>,
        cond: Option<Expr>,
        update: Option<Expr>,
        body: Box<Stmt>,
        pos: Pos,
    },
    ForIn {
        var: String,
        iterable: Expr,
        body: Box<Stmt>,
        pos: Pos,
    },
    Break {
        label: Option<String>,
        value: Option<Expr>,
        pos: Pos,
    },
    Continue {
        label: Option<String>,
        pos: Pos,
    },
    Return {
        value: Option<Expr>,
        pos: Pos,
    },
    Throw {
        value: Expr,
        pos: Pos,
    },
    Try {
        try_blk: Box<Stmt>,
        catch_var: Option<String>,
        catch_blk: Option<Box<Stmt>>,
        finally_blk: Option<Box<Stmt>>,
        pos: Pos,
    },
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum DestructPattern {
    Ident(String),
    Array(Vec<DestructPattern>),
    Object(Vec<(String, DestructPattern)>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum AssignTarget {
    Ident(String),
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
    },
    Field {
        object: Box<Expr>,
        field: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Null(Pos),
    Bool(bool, Pos),
    Num(f64, Pos),
    Str(String, Pos),
    TemplateStr(Vec<TemplatePart>, Pos),
    Ident(String, Pos),
    Array(Vec<Expr>, Pos),
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
        pos: Pos,
    },
    Object(Vec<(String, Expr)>, Pos),
    Field {
        object: Box<Expr>,
        field: String,
        pos: Pos,
    },
    EnumVariant {
        enum_name: String,
        variant: String,
        value: Option<Box<Expr>>,
        pos: Pos,
    },
    Match {
        expr: Box<Expr>,
        arms: Vec<MatchArm>,
        pos: Pos,
    },
    Result {
        is_ok: bool,
        value: Box<Expr>,
        pos: Pos,
    },
    Option {
        is_some: bool,
        value: Option<Box<Expr>>,
        pos: Pos,
    },
    TryExpr {
        expr: Box<Expr>,
        pos: Pos,
    },
    BinOp {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
        pos: Pos,
    },
    UnaryOp {
        op: UnaryOp,
        expr: Box<Expr>,
        pos: Pos,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        pos: Pos,
    },
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

#[derive(Debug, Clone, PartialEq)]
pub enum TemplatePart {
    Literal(String),
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Literal(Expr),
    Wildcard,
    Ident(String),
    Array(Vec<Pattern>),
    Object(Vec<(String, Pattern)>),
    EnumVariant {
        enum_name: String,
        variant: String,
        binding: Option<String>,
    },
    ResultOk {
        binding: String,
    },
    ResultErr {
        binding: String,
    },
    OptionSome {
        binding: String,
    },
    OptionNone,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Neq,
    Lt,
    Gt,
    Lte,
    Gte,
    And,
    Or,
    Assign,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOp {
    Neg,
    Ref,
    RefMut,
    Deref,
}

impl fmt::Display for Pos {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}
