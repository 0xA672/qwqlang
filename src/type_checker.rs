use crate::ast::{Expr, Stmt, Type, DestructPattern};
use crate::error::Error;
use std::collections::HashMap;

// ============================================================
// Gradual Type Checker
// ============================================================

pub struct TC {
    env: Vec<HashMap<String, Type>>,
    errors: Vec<TypeError>,
}

#[derive(Debug, Clone)]
pub struct TypeError {
    pub msg: String,
    pub pos: Option<crate::ast::Pos>,
}

impl TC {
    pub fn new() -> Self {
        TC {
            env: vec![HashMap::new()],
            errors: Vec::new(),
        }
    }

    /// Push a new scope.
    fn push_scope(&mut self) {
        self.env.push(HashMap::new());
    }

    /// Pop the current scope.
    fn pop_scope(&mut self) {
        self.env.pop();
    }

    /// Look up a variable's type in the environment chain.
    fn lookup(&self, name: &str) -> Option<&Type> {
        for scope in self.env.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty);
            }
        }
        None
    }

    /// Bind a variable to a type in the innermost scope.
    fn bind(&mut self, name: &str, ty: Type) {
        self.env.last_mut().unwrap().insert(name.to_string(), ty);
    }

    /// Check consistency between an annotated type and an inferred type.
    /// `Dyn` is consistent with everything.
    fn consistent(a: &Type, b: &Type) -> bool {
        match (a, b) {
            (Type::Dyn, _) | (_, Type::Dyn) => true,
            (Type::Array(ta), Type::Array(tb)) => Self::consistent(ta, tb),
            (Type::Object(ta), Type::Object(tb)) => Self::consistent(ta, tb),
            (Type::Function { params: pa, ret: ra },
             Type::Function { params: pb, ret: rb }) => {
                pa.len() == pb.len()
                    && pa.iter().zip(pb).all(|(a, b)| Self::consistent(a, b))
                    && Self::consistent(ra, rb)
            }
            _ => a == b,
        }
    }

    // --------------------------------------------------------
    // Type inference
    // --------------------------------------------------------

    pub fn infer_expr(&mut self, expr: &Expr) -> Type {
        match expr {
            Expr::Null(_) => Type::Null,
            Expr::Bool(_, _) => Type::Bool,
            Expr::Num(_, _) => Type::Num,
            Expr::Str(_, _) => Type::Str,
            Expr::Ident(name, _) => {
                self.lookup(name).cloned().unwrap_or(Type::Dyn)
            }
            Expr::Array(elements, _) => {
                if let Some(first) = elements.first() {
                    let elem_type = self.infer_expr(first);
                    for e in &elements[1..] {
                        let et = self.infer_expr(e);
                        if !Self::consistent(&elem_type, &et) {
                            self.error(format!(
                                "array element type mismatch: expected {}, got {}",
                                elem_type, et
                            ), None);
                        }
                    }
                    Type::Array(Box::new(elem_type))
                } else {
                    Type::Array(Box::new(Type::Dyn))
                }
            }
            Expr::Object(fields, _) => {
                if let Some((_, first_val)) = fields.first() {
                    let val_type = self.infer_expr(first_val);
                    for (_, v) in &fields[1..] {
                        let vt = self.infer_expr(v);
                        if !Self::consistent(&val_type, &vt) {
                            self.error(format!(
                                "object value type mismatch: expected {}, got {}",
                                val_type, vt
                            ), None);
                        }
                    }
                    Type::Object(Box::new(val_type))
                } else {
                    Type::Object(Box::new(Type::Dyn))
                }
            }
            Expr::BinOp { op, left, right, .. } => {
                let lt = self.infer_expr(left);
                let rt = self.infer_expr(right);
                match op {
                    crate::ast::BinOp::Add | crate::ast::BinOp::Sub
                    | crate::ast::BinOp::Mul | crate::ast::BinOp::Div => {
                        if !Self::consistent(&Type::Num, &lt) {
                            self.error(format!("expected Num, got {}", lt), None);
                        }
                        if !Self::consistent(&Type::Num, &rt) {
                            self.error(format!("expected Num, got {}", rt), None);
                        }
                        Type::Num
                    }
                    _ => Type::Bool,
                }
            }
            Expr::UnaryOp { op: _, expr, .. } => {
                self.infer_expr(expr)
            }
            Expr::Func { params, ret_type, body, .. } => {
                self.push_scope();
                let mut param_types = Vec::new();
                for (name, anno) in params {
                    let pty = anno.clone().unwrap_or(Type::Dyn);
                    self.bind(name, pty.clone());
                    param_types.push(pty);
                }
                // Infer body return type
                let body_type = self.infer_stmt(body);
                let ret = ret_type.clone().unwrap_or(body_type.clone());
                if let Some(anno) = ret_type {
                    if !Self::consistent(anno, &body_type) {
                        self.error(format!(
                            "return type mismatch: expected {}, inferred {}",
                            anno, body_type
                        ), None);
                    }
                }
                self.pop_scope();
                Type::Function {
                    params: param_types,
                    ret: Box::new(ret),
                }
            }
            Expr::Arrow { params, ret_type, body, .. } => {
                self.push_scope();
                let mut param_types = Vec::new();
                for (name, anno) in params {
                    let pty = anno.clone().unwrap_or(Type::Dyn);
                    self.bind(name, pty.clone());
                    param_types.push(pty);
                }
                let body_type = self.infer_expr(body);
                let ret = ret_type.clone().unwrap_or(body_type.clone());
                if let Some(anno) = ret_type {
                    if !Self::consistent(anno, &body_type) {
                        self.error(format!(
                            "return type mismatch: expected {}, inferred {}",
                            anno, body_type
                        ), None);
                    }
                }
                self.pop_scope();
                Type::Function {
                    params: param_types,
                    ret: Box::new(ret),
                }
            }
            Expr::Pipe { args, func, .. } => {
                for arg in args {
                    self.infer_expr(arg);
                }
                let func_type = self.infer_expr(func);
                match func_type {
                    Type::Function { ret, .. } => *ret,
                    _ => Type::Dyn,
                }
            }
            _ => Type::Dyn, // fallback for all other expressions
        }
    }

    pub fn infer_stmt(&mut self, stmt: &Stmt) -> Type {
        match stmt {
            Stmt::Let { pattern, type_anno, init, .. } => {
                let init_type = self.infer_expr(init);
                if let Some(anno) = type_anno {
                    if !Self::consistent(anno, &init_type) {
                        self.error(format!(
                            "type mismatch: expected `{}`, got `{}`",
                            anno, init_type
                        ), None);
                    }
                }
                self.bind_pattern(pattern, init_type);
                Type::Null
            }
            Stmt::Mut { pattern, type_anno, init, .. } => {
                let init_type = self.infer_expr(init);
                if let Some(anno) = type_anno {
                    if !Self::consistent(anno, &init_type) {
                        self.error(format!(
                            "type mismatch: expected `{}`, got `{}`",
                            anno, init_type
                        ), None);
                    }
                }
                self.bind_pattern(pattern, init_type);
                Type::Null
            }
            Stmt::Assign { target, value, .. } => {
                self.infer_expr(value);
                Type::Null
            }
            Stmt::Expr(expr) => self.infer_expr(expr),
            Stmt::Block(stmts) => {
                self.push_scope();
                let mut result = Type::Null;
                for s in stmts {
                    result = self.infer_stmt(s);
                }
                self.pop_scope();
                result
            }
            Stmt::If { cond, then_blk, else_blk, .. } => {
                self.infer_expr(cond);
                let then_t = self.infer_stmt(then_blk);
                let else_t = else_blk.as_ref()
                    .map(|b| self.infer_stmt(b))
                    .unwrap_or(Type::Null);
                // Return the consistent lub (least upper bound)
                if Self::consistent(&then_t, &else_t) { then_t } else { Type::Dyn }
            }
            Stmt::Return { value, .. } => {
                value.as_ref().map(|v| self.infer_expr(v)).unwrap_or(Type::Null)
            }
            Stmt::Loop { body, .. } | Stmt::While { body, .. } => {
                self.infer_stmt(body);
                Type::Null
            }
            Stmt::ForIn { iterable, body, .. } => {
                self.infer_expr(iterable);
                self.infer_stmt(body);
                Type::Null
            }
            _ => Type::Null,
        }
    }

    fn bind_pattern(&mut self, pattern: &DestructPattern, ty: Type) {
        match pattern {
            DestructPattern::Ident(name) => {
                self.bind(name, ty);
            }
            DestructPattern::Array(elems) => {
                if let Type::Array(elem_ty) = ty {
                    for elem in elems {
                        self.bind_pattern(elem, *elem_ty.clone());
                    }
                }
            }
            DestructPattern::Object(fields) => {
                if let Type::Object(val_ty) = ty {
                    for (_, fp) in fields {
                        self.bind_pattern(fp, *val_ty.clone());
                    }
                }
            }
        }
    }

    fn error(&mut self, msg: String, pos: Option<crate::ast::Pos>) {
        self.errors.push(TypeError { msg, pos });
    }

    pub fn check_program(&mut self, stmts: &[Stmt]) -> Vec<TypeError> {
        self.errors.clear();
        self.env.clear();
        self.env.push(HashMap::new());
        for stmt in stmts {
            self.infer_stmt(stmt);
        }
        self.errors.clone()
    }
}
