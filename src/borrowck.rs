use crate::ast::{AssignTarget, DestructPattern, Expr, Pos, Stmt, TemplatePart, UnaryOp};
use crate::error::Error;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
enum BorrowKind {
    Shared,
    Mutable,
}

#[derive(Debug, Clone)]
struct BorrowInfo {
    kind: BorrowKind,
    pos: Pos,
}

#[derive(Debug, Clone)]
enum VarState {
    Owned { is_mut: bool },
    Moved,
}

#[derive(Debug)]
struct Scope {
    vars: HashMap<String, VarState>,
    borrows: Vec<(String, BorrowInfo)>,
}

#[derive(Debug)]
pub struct BorrowChecker {
    scopes: Vec<Scope>,
}

impl BorrowChecker {
    pub fn new() -> Self {
        BorrowChecker {
            scopes: vec![Scope {
                vars: HashMap::new(),
                borrows: Vec::new(),
            }],
        }
    }

    pub fn check(&mut self, stmts: &[Stmt]) -> Result<(), Error> {
        for stmt in stmts {
            self.check_stmt(stmt)?;
        }
        Ok(())
    }

    fn push_scope(&mut self) {
        self.scopes.push(Scope {
            vars: HashMap::new(),
            borrows: Vec::new(),
        });
    }

    fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    fn current_scope_mut(&mut self) -> &mut Scope {
        self.scopes.last_mut().unwrap()
    }

    fn find_var(&self, name: &str) -> Option<&VarState> {
        for scope in self.scopes.iter().rev() {
            if let Some(state) = scope.vars.get(name) {
                return Some(state);
            }
        }
        None
    }

    fn find_var_mut(&mut self, name: &str) -> Option<&mut VarState> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(state) = scope.vars.get_mut(name) {
                return Some(state);
            }
        }
        None
    }

    fn get_borrows(&self, name: &str) -> Vec<&BorrowInfo> {
        let mut result = Vec::new();
        for scope in &self.scopes {
            for (n, info) in &scope.borrows {
                if n == name {
                    result.push(info);
                }
            }
        }
        result
    }

    fn declare_var(&mut self, name: &str, is_mut: bool) {
        self.current_scope_mut()
            .vars
            .insert(name.to_string(), VarState::Owned { is_mut });
    }

    fn check_stmt(&mut self, stmt: &Stmt) -> Result<(), Error> {
        match stmt {
            Stmt::Let { pattern, init, .. } => {
                self.check_expr(init)?;
                for var in Self::pattern_bound_vars(pattern) {
                    self.declare_var(&var, false);
                }
            }
            Stmt::Mut { pattern, init, .. } => {
                self.check_expr(init)?;
                for var in Self::pattern_bound_vars(pattern) {
                    self.declare_var(&var, true);
                }
            }
            Stmt::Assign { target, value, pos } => {
                self.check_expr(value)?;
                self.check_assign_target(target, *pos)?;
            }
            Stmt::Block(stmts) => {
                self.push_scope();
                for s in stmts {
                    self.check_stmt(s)?;
                }
                self.pop_scope();
            }
            Stmt::If {
                cond,
                then_blk,
                else_blk,
                ..
            } => {
                self.check_expr(cond)?;
                self.check_stmt(then_blk)?;
                if let Some(else_blk) = else_blk {
                    self.check_stmt(else_blk)?;
                }
            }
            Stmt::Loop { body, .. } => {
                self.check_stmt(body)?;
            }
            Stmt::While { cond, body, .. } => {
                self.check_expr(cond)?;
                self.check_stmt(body)?;
            }
            Stmt::For {
                init,
                cond,
                update,
                body,
                ..
            } => {
                self.push_scope();
                if let Some(init) = init {
                    self.check_stmt(init)?;
                }
                if let Some(cond) = cond {
                    self.check_expr(cond)?;
                }
                if let Some(update) = update {
                    self.check_expr(update)?;
                }
                self.check_stmt(body)?;
                self.pop_scope();
            }
            Stmt::ForIn { var, iterable, body, .. } => {
                self.push_scope();
                self.check_expr(iterable)?;
                self.declare_var(var, false);
                self.check_stmt(body)?;
                self.pop_scope();
            }
            Stmt::Break { value, .. } => {
                if let Some(value) = value {
                    self.check_expr(value)?;
                }
            }
            Stmt::Continue { .. } => {}
            Stmt::Return { value, .. } => {
                if let Some(value) = value {
                    self.check_expr(value)?;
                }
            }
            Stmt::Throw { value, .. } => {
                self.check_expr(value)?;
            }
            Stmt::Try { try_blk, catch_var, catch_blk, finally_blk, .. } => {
                self.check_stmt(try_blk)?;
                if let (Some(var), Some(blk)) = (catch_var, catch_blk) {
                    self.push_scope();
                    self.declare_var(var, false);
                    self.check_stmt(blk)?;
                    self.pop_scope();
                }
                if let Some(blk) = finally_blk {
                    self.check_stmt(blk)?;
                }
            }
            Stmt::Expr(e) => {
                self.check_expr(e)?;
            }
        }
        Ok(())
    }

    fn check_expr(&mut self, expr: &Expr) -> Result<(), Error> {
        match expr {
            Expr::Null(_)
            | Expr::Bool(_, _)
            | Expr::Num(_, _)
            | Expr::Str(_, _)
            | Expr::Array(_, _)
            | Expr::Object(_, _) => {}
            Expr::TemplateStr(parts, _) => {
                for part in parts {
                    if let TemplatePart::Expr(e) = part {
                        self.check_expr(e)?;
                    }
                }
            }
            Expr::Ident(name, pos) => {
                self.check_read_access(name, *pos)?;
            }
            Expr::Index { object, index, .. } => {
                self.check_expr(object)?;
                self.check_expr(index)?;
            }
            Expr::Field { object, .. } => {
                self.check_expr(object)?;
            }
            Expr::EnumVariant { value, .. } => {
                if let Some(value) = value {
                    self.check_expr(value)?;
                }
            }
            Expr::Match { expr, arms, .. } => {
                self.check_expr(expr)?;
                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        self.check_expr(guard)?;
                    }
                    self.check_expr(&arm.body)?;
                }
            }
            Expr::Result { value, .. } => {
                self.check_expr(value)?;
            }
            Expr::Option { value, .. } => {
                if let Some(value) = value {
                    self.check_expr(value)?;
                }
            }
            Expr::TryExpr { expr, .. } => {
                self.check_expr(expr)?;
            }
            Expr::BinOp { left, right, .. } => {
                self.check_expr(left)?;
                self.check_expr(right)?;
            }
            Expr::UnaryOp { op, expr, pos } => {
                match op {
                    UnaryOp::Ref => {
                        self.check_borrow(expr, BorrowKind::Shared, *pos)?;
                    }
                    UnaryOp::RefMut => {
                        self.check_borrow(expr, BorrowKind::Mutable, *pos)?;
                    }
                    UnaryOp::Deref => {
                        self.check_expr(expr)?;
                    }
                    UnaryOp::Neg => {
                        self.check_expr(expr)?;
                    }
                }
            }
            Expr::Call { callee, args, .. } => {
                self.check_expr(callee)?;
                for arg in args {
                    self.check_expr(arg)?;
                }
            }
            Expr::Func { body, .. } => {
                self.push_scope();
                self.check_stmt(body)?;
                self.pop_scope();
            }
            Expr::Arrow { body, .. } => {
                self.check_expr(body)?;
            }
            Expr::Pipe { left, right, .. } => {
                self.check_expr(left)?;
                self.check_expr(right)?;
            }
        }
        Ok(())
    }

    fn check_borrow(&mut self, expr: &Expr, kind: BorrowKind, pos: Pos) -> Result<(), Error> {
        if let Expr::Ident(name, _) = expr {
            self.check_valid_borrow(name, kind.clone(), pos)?;
            let info = BorrowInfo { kind, pos };
            self.current_scope_mut()
                .borrows
                .push((name.clone(), info));
            self.check_expr(expr)?;
        } else {
            self.check_expr(expr)?;
        }
        Ok(())
    }

    fn check_valid_borrow(&self, name: &str, kind: BorrowKind, pos: Pos) -> Result<(), Error> {
        let var_state = self.find_var(name).cloned();
        match var_state {
            Some(VarState::Owned { is_mut }) => {
                if kind == BorrowKind::Mutable && !is_mut {
                    return Err(Error::Compile {
                        pos,
                        msg: format!(
                            "cannot borrow '{}' as mutable, as it is not declared as mutable",
                            name
                        ),
                    });
                }
                let borrows = self.get_borrows(name);
                match kind {
                    BorrowKind::Shared => {
                        for b in &borrows {
                            if b.kind == BorrowKind::Mutable {
                                return Err(Error::Compile {
                                    pos,
                                    msg: format!(
                                        "cannot borrow '{}' as shared because it is also borrowed as mutable",
                                        name
                                    ),
                                });
                            }
                        }
                    }
                    BorrowKind::Mutable => {
                        if !borrows.is_empty() {
                            return Err(Error::Compile {
                                pos,
                                msg: format!(
                                    "cannot borrow '{}' as mutable more than once at a time",
                                    name
                                ),
                            });
                        }
                    }
                }
                Ok(())
            }
            Some(VarState::Moved) => Err(Error::Compile {
                pos,
                msg: format!("borrow of moved value: '{}'", name),
            }),
            None => Ok(()),
        }
    }

    fn check_mutable_access(&self, name: &str, pos: Pos) -> Result<(), Error> {
        let var_state = self.find_var(name).cloned();
        match var_state {
            Some(VarState::Owned { is_mut }) => {
                if !is_mut {
                    return Err(Error::Compile {
                        pos,
                        msg: format!(
                            "cannot assign to '{}' as it is not declared as mutable",
                            name
                        ),
                    });
                }
                let borrows = self.get_borrows(name);
                if !borrows.is_empty() {
                    return Err(Error::Compile {
                        pos,
                        msg: format!(
                            "cannot assign to '{}' because it is borrowed",
                            name
                        ),
                    });
                }
                Ok(())
            }
            Some(VarState::Moved) => Err(Error::Compile {
                pos,
                msg: format!("assignment to moved value: '{}'", name),
            }),
            None => Ok(()),
        }
    }

    fn check_read_access(&self, name: &str, pos: Pos) -> Result<(), Error> {
        let var_state = self.find_var(name).cloned();
        match var_state {
            Some(VarState::Owned { .. }) => {
                let borrows = self.get_borrows(name);
                for b in &borrows {
                    if b.kind == BorrowKind::Mutable {
                        return Err(Error::Compile {
                            pos,
                            msg: format!(
                                "cannot use '{}' because it was mutably borrowed",
                                name
                            ),
                        });
                    }
                }
                Ok(())
            }
            Some(VarState::Moved) => Err(Error::Compile {
                pos,
                msg: format!("use of moved value: '{}'", name),
            }),
            None => Ok(()),
        }
    }

    fn pattern_bound_vars(pattern: &DestructPattern) -> Vec<String> {
        let mut vars = Vec::new();
        match pattern {
            DestructPattern::Ident(name) => {
                vars.push(name.clone());
            }
            DestructPattern::Array(patterns) => {
                for p in patterns {
                    vars.extend(Self::pattern_bound_vars(p));
                }
            }
            DestructPattern::Object(fields) => {
                for (_, p) in fields {
                    vars.extend(Self::pattern_bound_vars(p));
                }
            }
        }
        vars
    }

    fn check_assign_target(&mut self, target: &AssignTarget, pos: Pos) -> Result<(), Error> {
        match target {
            AssignTarget::Ident(name) => {
                self.check_mutable_access(name, pos)?;
            }
            AssignTarget::Index { object, index } => {
                self.check_expr(object)?;
                self.check_expr(index)?;
            }
            AssignTarget::Field { object, .. } => {
                self.check_expr(object)?;
            }
        }
        Ok(())
    }
}
