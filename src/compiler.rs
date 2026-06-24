use crate::ast::{AssignTarget, BinOp, DestructPattern, Expr, Pattern, Pos, Stmt, TemplatePart, UnaryOp};
use crate::error::{levenshtein, Error};
use crate::vm::{CompiledFunction, Value};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum Op {
    Constant = 0,
    Null = 1,
    True = 2,
    False = 3,
    Add = 4,
    Sub = 5,
    Mul = 6,
    Div = 7,
    Eq = 8,
    Neq = 9,
    Lt = 10,
    Gt = 11,
    Lte = 12,
    Gte = 13,
    Negate = 14,
    Pop = 15,
    GetGlobal = 16,
    SetGlobal = 17,
    GetLocal = 18,
    SetLocal = 19,
    GetFree = 20,
    SetFree = 21,
    Jump = 22,
    JumpIfFalse = 23,
    JumpIfTrue = 28,
    JumpIfFalsePop = 29,
    Closure = 24,
    Call = 25,
    Return = 26,
    GetBuiltin = 27,
    CaptureGlobal = 30,
    Ref = 31,
    RefMut = 32,
    Deref = 33,
    Array = 34,
    Object = 35,
    Index = 36,
    IndexSet = 37,
    GetField = 38,
    SetField = 39,
    EnumVariant = 40,
    IsEnumVariant = 41,
    GetEnumValue = 42,
    // Result/Option operations
    IsOk = 43,
    IsErr = 44,
    IsSome = 45,
    IsNone = 46,
    UnwrapOk = 47,
    UnwrapErr = 48,
    UnwrapSome = 49,
    MakeResult = 50,
    MakeOption = 51,
    // Exception handling
    Throw = 52,
    PushTry = 53,
    PopTry = 54,
    // String operations
    Concat = 55,
}

#[derive(Debug)]
pub struct Comp {
    bytecode: Vec<u8>,
    constants: Vec<Value>,
    globals: HashMap<String, (usize, bool)>,
    builtins: HashMap<String, usize>,
    locals: Vec<(String, bool)>,
    free: Vec<(String, bool)>,
    loop_stack: Vec<(usize, Option<String>, Vec<(usize, bool)>, Vec<usize>)>,
    func_stack: Vec<FuncCtx>,
    next_global: usize,
}

#[derive(Debug)]
struct FuncCtx {
    locals: Vec<(String, bool)>,
    free: Vec<(String, bool)>,
    captures: HashSet<String>,
}

impl Comp {
    pub fn new() -> Self {
        let mut builtins = HashMap::new();
        builtins.insert("print".to_string(), 0);
        builtins.insert("len".to_string(), 1);
        builtins.insert("push".to_string(), 2);
        builtins.insert("pop".to_string(), 3);
        builtins.insert("is_ok".to_string(), 4);
        builtins.insert("is_err".to_string(), 5);
        builtins.insert("is_some".to_string(), 6);
        builtins.insert("is_none".to_string(), 7);
        builtins.insert("unwrap".to_string(), 8);
        builtins.insert("unwrap_or".to_string(), 9);
        let mut globals = HashMap::new();
        globals.insert("print".to_string(), (0, false));
        Comp {
            bytecode: Vec::new(),
            constants: Vec::new(),
            globals,
            builtins,
            locals: Vec::new(),
            free: Vec::new(),
            loop_stack: Vec::new(),
            func_stack: Vec::new(),
            next_global: 1,
        }
    }

    pub fn reset(&mut self) {
        self.bytecode.clear();
        self.constants.clear();
        self.locals.clear();
        self.free.clear();
        self.loop_stack.clear();
        self.func_stack.clear();
    }

    pub fn compile(&mut self, stmts: &[Stmt]) -> Result<CompiledFunction, Error> {
        for stmt in stmts {
            self.stmt(stmt)?;
        }
        self.emit_op(Op::Return);

        #[cfg(debug_assertions)]
        {
            eprintln!("DEBUG: bytecode = {:?}", self.bytecode);
            eprintln!("DEBUG: constants = {:?}", self.constants);
        }

        Ok(CompiledFunction {
            bytecode: self.bytecode.clone(),
            constants: self.constants.clone(),
            num_locals: self.locals.len() as u16,
            num_params: 0,
            num_free: 0,
        })
    }

    fn stmt(&mut self, stmt: &Stmt) -> Result<(), Error> {
        match stmt {
            Stmt::Let { pattern, init, pos } => {
                // Handle function global registration for simple identifier patterns
                if self.func_stack.is_empty() && matches!(init, Expr::Func { .. }) {
                    if let DestructPattern::Ident(name) = pattern {
                        if !self.globals.contains_key(name) {
                            let idx = self.next_global;
                            self.next_global += 1;
                            self.globals.insert(name.clone(), (idx, false));
                        }
                    }
                }
                self.expr(init)?;
                self.compile_destruct_pattern(pattern, *pos)?;
            }
            Stmt::Mut { pattern, init, pos } => {
                self.expr(init)?;
                self.compile_destruct_pattern_mut(pattern, *pos)?;
            }
            Stmt::Assign { target, value, pos } => {
                match target {
                    AssignTarget::Ident(name) => {
                        self.check_mutability(name, *pos)?;
                        self.expr(value)?;
                        if let Some(idx) = self.locals.iter().position(|(n, _)| n == name) {
                            self.emit_op(Op::SetLocal);
                            self.emit_u16(idx as u16);
                        } else if let Some(idx) = self.free.iter().position(|(n, _)| n == name) {
                            self.emit_op(Op::SetFree);
                            self.emit_u16(idx as u16);
                        } else if let Some(&(idx, _)) = self.globals.get(name) {
                            self.emit_op(Op::SetGlobal);
                            self.emit_u16(idx as u16);
                        } else {
                            return self.undefined_var_error(name, *pos);
                        }
                    }
                    AssignTarget::Index { object, index } => {
                        self.expr(object)?;
                        self.expr(index)?;
                        self.expr(value)?;
                        self.emit_op(Op::IndexSet);
                    }
                    AssignTarget::Field { object, field } => {
                        self.expr(object)?;
                        let field_idx = self.add_constant(Value::Str(field.clone()));
                        self.emit_op(Op::Constant);
                        self.emit_u16(field_idx as u16);
                        self.expr(value)?;
                        self.emit_op(Op::SetField);
                    }
                }
            }
            Stmt::Block(stmts) => {
                for s in stmts {
                    self.stmt(s)?;
                }
            }
            Stmt::If {
                cond,
                then_blk,
                else_blk,
                ..
            } => {
                self.expr(cond)?;
                let else_jump = self.emit_jump_if_false_pop();
                self.stmt(then_blk)?;
                let end_jump = if else_blk.is_some() {
                    Some(self.emit_jump())
                } else {
                    None
                };
                self.patch_jump(else_jump);
                if let Some(else_blk) = else_blk {
                    self.stmt(else_blk)?;
                    self.patch_jump(end_jump.unwrap());
                }
            }
            Stmt::Loop { label, body, .. } => {
                let loop_start = self.bytecode.len();
                self.loop_stack
                    .push((loop_start, label.clone(), Vec::new(), Vec::new()));
                self.stmt(body)?;
                let jump_pos = self.bytecode.len();
                self.emit_op(Op::Jump);
                let offset = loop_start as i16 - jump_pos as i16;
                self.emit_i16(offset);
                let pre_cleanup_len = self.bytecode.len();
                self.emit_op(Op::Pop);
                self.emit_op(Op::Null);
                let loop_end = self.bytecode.len();

                self.patch_continue_jumps(loop_end);

                let mut any_break_has_value = false;
                if let Some((_, _, break_positions, _)) = self.loop_stack.last() {
                    let break_positions = break_positions.clone();
                    for (pos, has_value) in &break_positions {
                        if *has_value {
                            any_break_has_value = true;
                            let target = loop_end as i16;
                            let jump_instr_pos = (*pos - 1) as i16;
                            let offset = target - jump_instr_pos;
                            self.bytecode[*pos] = (offset & 0xff) as u8;
                            self.bytecode[*pos + 1] = ((offset >> 8) & 0xff) as u8;
                        } else {
                            let target = pre_cleanup_len as i16;
                            let jump_instr_pos = (*pos - 1) as i16;
                            let offset = target - jump_instr_pos;
                            self.bytecode[*pos] = (offset & 0xff) as u8;
                            self.bytecode[*pos + 1] = ((offset >> 8) & 0xff) as u8;
                        }
                    }
                }

                self.loop_stack.pop();

                if any_break_has_value {
                    self.bytecode.truncate(pre_cleanup_len);
                }
            }
            Stmt::While { cond, body, .. } => {
                let cond_pos = self.bytecode.len();
                self.loop_stack
                    .push((cond_pos, None, Vec::new(), Vec::new()));
                self.expr(cond)?;
                let jump_to_end = self.emit_jump_if_false();
                self.stmt(body)?;
                let jump_back = self.bytecode.len();
                self.emit_op(Op::Jump);
                let offset = cond_pos as i16 - jump_back as i16;
                self.emit_i16(offset);
                let pre_cleanup_len = self.bytecode.len();
                self.patch_jump(jump_to_end);
                self.emit_op(Op::Pop);
                self.emit_op(Op::Null);
                let loop_end = self.bytecode.len();

                self.patch_continue_jumps(loop_end);

                let mut any_break_has_value = false;
                if let Some((_, _, break_positions, _)) = self.loop_stack.last() {
                    let break_positions = break_positions.clone();
                    for (break_pos, has_value) in &break_positions {
                        if *has_value {
                            any_break_has_value = true;
                            let target = loop_end as i16;
                            let jump_instr_pos = (*break_pos - 1) as i16;
                            let offset = target - jump_instr_pos;
                            self.bytecode[*break_pos] = (offset & 0xff) as u8;
                            self.bytecode[*break_pos + 1] = ((offset >> 8) & 0xff) as u8;
                        } else {
                            let target = pre_cleanup_len as i16;
                            let jump_instr_pos = (*break_pos - 1) as i16;
                            let offset = target - jump_instr_pos;
                            self.bytecode[*break_pos] = (offset & 0xff) as u8;
                            self.bytecode[*break_pos + 1] = ((offset >> 8) & 0xff) as u8;
                        }
                    }
                }

                if any_break_has_value {
                    self.bytecode.truncate(pre_cleanup_len);
                }
                self.loop_stack.pop();
            }
            Stmt::For { init, cond, update, body, .. } => {
                if let Some(init) = init {
                    self.stmt(init)?;
                }
                let cond_pos = self.bytecode.len();
                self.loop_stack
                    .push((cond_pos, None, Vec::new(), Vec::new()));
                if let Some(cond) = cond {
                    self.expr(cond)?;
                    let jump_to_end = self.emit_jump_if_false();
                    self.stmt(body)?;
                    let _update_pos = self.bytecode.len();
                    if let Some(update) = update {
                        self.expr(update)?;
                        self.emit_op(Op::Pop);
                    }
                    let jump_back = self.bytecode.len();
                    self.emit_op(Op::Jump);
                    let offset = cond_pos as i16 - jump_back as i16;
                    self.emit_i16(offset);
                    let pre_cleanup_len = self.bytecode.len();
                    self.patch_jump(jump_to_end);
                    self.emit_op(Op::Pop);
                    self.emit_op(Op::Null);
                    let loop_end = self.bytecode.len();

                    self.patch_continue_jumps(loop_end);

                    let mut any_break_has_value = false;
                    if let Some((_, _, break_positions, _)) = self.loop_stack.last() {
                        let break_positions = break_positions.clone();
                        for (pos, has_value) in &break_positions {
                            if *has_value {
                                any_break_has_value = true;
                                let target = loop_end as i16;
                                let jump_instr_pos = (*pos - 1) as i16;
                                let offset = target - jump_instr_pos;
                                self.bytecode[*pos] = (offset & 0xff) as u8;
                                self.bytecode[*pos + 1] = ((offset >> 8) & 0xff) as u8;
                            } else {
                                let target = pre_cleanup_len as i16;
                                let jump_instr_pos = (*pos - 1) as i16;
                                let offset = target - jump_instr_pos;
                                self.bytecode[*pos] = (offset & 0xff) as u8;
                                self.bytecode[*pos + 1] = ((offset >> 8) & 0xff) as u8;
                            }
                        }
                    }

                    if any_break_has_value {
                        self.bytecode.truncate(pre_cleanup_len);
                    }
                } else {
                    self.stmt(body)?;
                    let _update_pos = self.bytecode.len();
                    if let Some(update) = update {
                        self.expr(update)?;
                        self.emit_op(Op::Pop);
                    }
                    let jump_back = self.bytecode.len();
                    self.emit_op(Op::Jump);
                    let offset = cond_pos as i16 - jump_back as i16;
                    self.emit_i16(offset);
                }
                self.loop_stack.pop();
            }
            Stmt::Break { label, value, pos } => {
                let loop_idx = if let Some(label) = label {
                    self.loop_stack
                        .iter()
                        .rposition(|(_, l, _, _)| l.as_deref() == Some(label))
                        .ok_or_else(|| Error::Compile {
                            pos: *pos,
                            msg: format!("undefined label '{}'", label),
                        })?
                } else {
                    self.loop_stack
                        .len()
                        .checked_sub(1)
                        .ok_or_else(|| Error::Compile {
                            pos: *pos,
                            msg: "break outside loop".to_string(),
                        })?
                };

                let has_value = value.is_some();

                if let Some(value) = value {
                    self.expr(value)?;
                }

                self.emit_op(Op::Jump);
                let break_pos = self.bytecode.len();
                self.emit_i16(0);
                self.loop_stack[loop_idx].2.push((break_pos, has_value));
            }
            Stmt::Continue { label, pos } => {
                let loop_idx = if let Some(label) = label {
                    self.loop_stack
                        .iter()
                        .rposition(|(_, l, _, _)| l.as_deref() == Some(label))
                        .ok_or_else(|| Error::Compile {
                            pos: *pos,
                            msg: format!("undefined label '{}'", label),
                        })?
                } else {
                    self.loop_stack
                        .len()
                        .checked_sub(1)
                        .ok_or_else(|| Error::Compile {
                            pos: *pos,
                            msg: "continue outside loop".to_string(),
                        })?
                };

                self.emit_op(Op::Jump);
                let continue_pos = self.bytecode.len();
                self.emit_i16(0);
                self.loop_stack[loop_idx].3.push(continue_pos);
            }
            Stmt::ForIn { var, iterable, body, pos } => {
                let arr_ident = format!("__arr_{}", pos.line);
                let idx_ident = format!("__idx_{}", pos.line);

                self.stmt(&Stmt::Let {
                    pattern: DestructPattern::Ident(arr_ident.clone()),
                    init: iterable.clone(),
                    pos: *pos,
                })?;
                self.stmt(&Stmt::Let {
                    pattern: DestructPattern::Ident(idx_ident.clone()),
                    init: Expr::Num(0.0, *pos),
                    pos: *pos,
                })?;

                let loop_start = self.bytecode.len();
                self.loop_stack
                    .push((loop_start, None, Vec::new(), Vec::new()));

                self.expr(&Expr::Ident(idx_ident.clone(), *pos))?;
                self.expr(&Expr::Ident(arr_ident.clone(), *pos))?;
                self.emit_op(Op::Index);
                self.stmt(&Stmt::Let {
                    pattern: DestructPattern::Ident(var.clone()),
                    init: Expr::Null(*pos),
                    pos: *pos,
                })?;

                self.expr(&Expr::Ident(idx_ident.clone(), *pos))?;
                self.expr(&Expr::Ident(arr_ident.clone(), *pos))?;
                self.emit_op(Op::Index);

                let continue_check = self.emit_jump_if_false();

                self.stmt(body)?;

                let _update_pos = self.bytecode.len();
                self.expr(&Expr::Ident(idx_ident.clone(), *pos))?;
                self.emit_op(Op::Constant);
                let const_idx = self.add_constant(Value::Num(1.0));
                self.emit_u16(const_idx as u16);
                self.emit_op(Op::Add);
                self.stmt(&Stmt::Assign {
                    target: AssignTarget::Ident(idx_ident.clone()),
                    value: Expr::Ident(idx_ident.clone(), *pos),
                    pos: *pos,
                })?;

                let jump_back = self.bytecode.len();
                self.emit_op(Op::Jump);
                let offset = loop_start as i16 - jump_back as i16;
                self.emit_i16(offset);

                let pre_cleanup_len = self.bytecode.len();
                self.patch_jump(continue_check);
                self.emit_op(Op::Pop);
                self.emit_op(Op::Null);

                self.patch_continue_jumps(loop_start);

                let mut any_break_has_value = false;
                if let Some((_, _, break_positions, _)) = self.loop_stack.last() {
                    let break_positions = break_positions.clone();
                    for (break_pos, has_value) in &break_positions {
                        if *has_value {
                            any_break_has_value = true;
                            let target = self.bytecode.len();
                            let jump_instr_pos = (*break_pos - 1) as i16;
                            let offset = target as i16 - jump_instr_pos;
                            self.bytecode[*break_pos] = (offset & 0xff) as u8;
                            self.bytecode[*break_pos + 1] = ((offset >> 8) & 0xff) as u8;
                        } else {
                            let target = pre_cleanup_len as i16;
                            let jump_instr_pos = (*break_pos - 1) as i16;
                            let offset = target - jump_instr_pos;
                            self.bytecode[*break_pos] = (offset & 0xff) as u8;
                            self.bytecode[*break_pos + 1] = ((offset >> 8) & 0xff) as u8;
                        }
                    }
                }

                if any_break_has_value {
                    self.bytecode.truncate(pre_cleanup_len);
                }
                self.loop_stack.pop();
            }
            Stmt::Return { value, .. } => {
                if let Some(value) = value {
                    self.expr(value)?;
                } else {
                    self.emit_op(Op::Null);
                }
                self.emit_op(Op::Return);
            }
            Stmt::Throw { value, .. } => {
                self.expr(value)?;
                self.emit_op(Op::Throw);
            }
            Stmt::Try { try_blk, catch_var, catch_blk, finally_blk, .. } => {
                // Push try handler
                self.emit_op(Op::PushTry);
                let catch_jump_offset_pos = self.bytecode.len();
                self.emit_u16(0); // Placeholder for catch handler offset
                let finally_jump_offset_pos = self.bytecode.len();
                self.emit_u16(0); // Placeholder for finally handler offset
                
                // Execute try block
                self.stmt(try_blk)?;
                
                // Pop try handler (no exception occurred)
                self.emit_op(Op::PopTry);
                
                // Jump to finally (or end if no finally)
                let skip_catch_jump = self.emit_jump();
                
                // Catch handler
                let catch_start = self.bytecode.len();
                // Patch the catch offset
                let catch_offset = catch_start - catch_jump_offset_pos;
                self.bytecode[catch_jump_offset_pos] = (catch_offset & 0xff) as u8;
                self.bytecode[catch_jump_offset_pos + 1] = ((catch_offset >> 8) & 0xff) as u8;
                
                if let Some(catch_blk) = catch_blk {
                    if let Some(var_name) = catch_var {
                        // Bind caught exception to variable
                        if !self.func_stack.is_empty() {
                            self.locals.push((var_name.clone(), false));
                        }
                        if self.func_stack.is_empty() {
                            if !self.globals.contains_key(var_name) {
                                let idx = self.next_global;
                                self.next_global += 1;
                                self.globals.insert(var_name.clone(), (idx, false));
                            }
                            let (idx, _) = self.globals[var_name];
                            self.emit_op(Op::SetGlobal);
                            self.emit_u16(idx as u16);
                        } else {
                            self.emit_op(Op::SetLocal);
                            self.emit_u16((self.locals.len() - 1) as u16);
                        }
                    }
                    self.stmt(catch_blk)?;
                } else {
                    // No catch block, just pop the exception
                    self.emit_op(Op::Pop);
                }
                
                // Jump to finally (or end)
                let skip_finally_after_catch = if finally_blk.is_some() {
                    Some(self.emit_jump())
                } else {
                    None
                };
                
                // Finally handler
                let finally_start = self.bytecode.len();
                // Patch the finally offset
                let finally_offset = finally_start - finally_jump_offset_pos;
                self.bytecode[finally_jump_offset_pos] = (finally_offset & 0xff) as u8;
                self.bytecode[finally_jump_offset_pos + 1] = ((finally_offset >> 8) & 0xff) as u8;
                
                if let Some(finally_blk) = finally_blk {
                    self.stmt(finally_blk)?;
                }
                
                // Patch jumps
                self.patch_jump(skip_catch_jump);
                if let Some(skip_finally) = skip_finally_after_catch {
                    self.patch_jump(skip_finally);
                }
            }
            Stmt::Expr(e) => {
                self.expr(e)?;
            }
        }
        Ok(())
    }

    fn expr(&mut self, expr: &Expr) -> Result<(), Error> {
        match expr {
            Expr::Null(_) => self.emit_op(Op::Null),
            Expr::Bool(b, _) => {
                if *b {
                    self.emit_op(Op::True)
                } else {
                    self.emit_op(Op::False)
                }
            }
            Expr::Num(n, _) => {
                let idx = self.add_constant(Value::Num(*n));
                self.emit_op(Op::Constant);
                self.emit_u16(idx as u16);
            }
            Expr::Str(s, _) => {
                let idx = self.add_constant(Value::Str(s.clone()));
                self.emit_op(Op::Constant);
                self.emit_u16(idx as u16);
            }
            Expr::TemplateStr(parts, _) => {
                // Concatenate all parts
                let mut first = true;
                for part in parts {
                    match part {
                        TemplatePart::Literal(s) => {
                            let idx = self.add_constant(Value::Str(s.clone()));
                            self.emit_op(Op::Constant);
                            self.emit_u16(idx as u16);
                        }
                        TemplatePart::Expr(e) => {
                            self.expr(e)?;
                        }
                    }
                    if first {
                        first = false;
                    } else {
                        self.emit_op(Op::Concat);
                    }
                }
                // If empty template string, push empty string
                if parts.is_empty() {
                    let idx = self.add_constant(Value::Str(String::new()));
                    self.emit_op(Op::Constant);
                    self.emit_u16(idx as u16);
                }
            }
            Expr::Array(elements, _) => {
                for elem in elements {
                    self.expr(elem)?;
                }
                self.emit_op(Op::Array);
                self.emit_u16(elements.len() as u16);
            }
            Expr::Index { object, index, .. } => {
                self.expr(object)?;
                self.expr(index)?;
                self.emit_op(Op::Index);
            }
            Expr::Object(fields, _) => {
                for (name, value) in fields {
                    let name_idx = self.add_constant(Value::Str(name.clone()));
                    self.emit_op(Op::Constant);
                    self.emit_u16(name_idx as u16);
                    self.expr(value)?;
                }
                self.emit_op(Op::Object);
                self.emit_u16(fields.len() as u16);
            }
            Expr::Field { object, field, .. } => {
                self.expr(object)?;
                let field_idx = self.add_constant(Value::Str(field.clone()));
                self.emit_op(Op::Constant);
                self.emit_u16(field_idx as u16);
                self.emit_op(Op::GetField);
            }
            Expr::EnumVariant { enum_name, variant, value, .. } => {
                let enum_idx = self.add_constant(Value::Str(enum_name.clone()));
                let variant_idx = self.add_constant(Value::Str(variant.clone()));
                self.emit_op(Op::Constant);
                self.emit_u16(enum_idx as u16);
                self.emit_op(Op::Constant);
                self.emit_u16(variant_idx as u16);
                if let Some(val) = value {
                    self.expr(val)?;
                } else {
                    self.emit_op(Op::Null);
                }
                self.emit_op(Op::EnumVariant);
            }
            Expr::Match { expr, arms, .. } => {
                self.expr(expr)?;
                let mut end_jumps = Vec::new();
                for arm in arms {
                    // Handle pattern matching
                    match &arm.pattern {
                        Pattern::Literal(lit) => {
                            // Duplicate the matched value for comparison
                            self.emit_op(Op::Constant);
                            let idx = self.add_constant(Value::Num(0.0));
                            self.emit_u16(idx as u16);
                            self.emit_op(Op::Index);
                            self.expr(lit)?;
                            self.emit_op(Op::Eq);
                            let guard_jump = if let Some(guard) = &arm.guard {
                                let pattern_fail_jump = self.emit_jump_if_false();
                                self.expr(guard)?;
                                let guard_fail_jump = self.emit_jump_if_false();
                                self.patch_jump(pattern_fail_jump);
                                self.emit_op(Op::Pop);
                                Some(guard_fail_jump)
                            } else {
                                Some(self.emit_jump_if_false())
                            };
                            self.emit_op(Op::Pop);
                            self.expr(&arm.body)?;
                            end_jumps.push(self.emit_jump());
                            if let Some(gj) = guard_jump {
                                self.patch_jump(gj);
                            }
                        }
                        Pattern::Wildcard => {
                            // Wildcard always matches, but check guard
                            if let Some(guard) = &arm.guard {
                                self.expr(guard)?;
                                let guard_fail_jump = self.emit_jump_if_false();
                                self.emit_op(Op::Pop);
                                self.expr(&arm.body)?;
                                end_jumps.push(self.emit_jump());
                                self.patch_jump(guard_fail_jump);
                            } else {
                                self.emit_op(Op::Pop);
                                self.expr(&arm.body)?;
                                end_jumps.push(self.emit_jump());
                            }
                        }
                        Pattern::Ident(name) => {
                            // Bind the matched value to the identifier
                            if let Some(guard) = &arm.guard {
                                // Duplicate value for guard check
                                self.emit_op(Op::Constant);
                                let idx = self.add_constant(Value::Num(0.0));
                                self.emit_u16(idx as u16);
                                self.emit_op(Op::Index);
                                self.expr(guard)?;
                                let guard_fail_jump = self.emit_jump_if_false();
                                self.emit_op(Op::Pop);
                                self.stmt(&Stmt::Let {
                                    pattern: DestructPattern::Ident(name.clone()),
                                    init: Expr::Null(Pos { line: 1, col: 1 }),
                                    pos: Pos { line: 1, col: 1 },
                                })?;
                                self.expr(&arm.body)?;
                                end_jumps.push(self.emit_jump());
                                self.patch_jump(guard_fail_jump);
                            } else {
                                self.emit_op(Op::Pop);
                                self.stmt(&Stmt::Let {
                                    pattern: DestructPattern::Ident(name.clone()),
                                    init: Expr::Null(Pos { line: 1, col: 1 }),
                                    pos: Pos { line: 1, col: 1 },
                                })?;
                                self.expr(&arm.body)?;
                                end_jumps.push(self.emit_jump());
                            }
                        }
                        Pattern::Array(elements) => {
                            // Check if value is array with correct length
                            self.emit_op(Op::Constant);
                            let idx = self.add_constant(Value::Str("len".to_string()));
                            self.emit_u16(idx as u16);
                            self.emit_op(Op::GetField);
                            self.emit_op(Op::Constant);
                            let len_idx = self.add_constant(Value::Num(elements.len() as f64));
                            self.emit_u16(len_idx as u16);
                            self.emit_op(Op::Eq);
                            let pattern_fail_jump = self.emit_jump_if_false();
                            
                            // Bind each element
                            for (i, elem_pattern) in elements.iter().enumerate() {
                                self.emit_op(Op::Constant);
                                let idx = self.add_constant(Value::Num(i as f64));
                                self.emit_u16(idx as u16);
                                self.emit_op(Op::Index);
                                match elem_pattern {
                                    Pattern::Ident(name) => {
                                        self.stmt(&Stmt::Let {
                                            pattern: DestructPattern::Ident(name.clone()),
                                            init: Expr::Null(Pos { line: 1, col: 1 }),
                                            pos: Pos { line: 1, col: 1 },
                                        })?;
                                    }
                                    Pattern::Wildcard => {
                                        self.emit_op(Op::Pop);
                                    }
                                    _ => {
                                        // Nested patterns would need more complex handling
                                        self.emit_op(Op::Pop);
                                    }
                                }
                            }
                            
                            // Check guard
                            if let Some(guard) = &arm.guard {
                                self.expr(guard)?;
                                let guard_fail_jump = self.emit_jump_if_false();
                                self.emit_op(Op::Pop);
                                self.expr(&arm.body)?;
                                end_jumps.push(self.emit_jump());
                                self.patch_jump(guard_fail_jump);
                            } else {
                                self.emit_op(Op::Pop);
                                self.expr(&arm.body)?;
                                end_jumps.push(self.emit_jump());
                            }
                            self.patch_jump(pattern_fail_jump);
                        }
                        Pattern::Object(fields) => {
                            // Bind each field
                            for (field_name, field_pattern) in fields {
                                let field_idx = self.add_constant(Value::Str(field_name.clone()));
                                self.emit_op(Op::Constant);
                                self.emit_u16(field_idx as u16);
                                self.emit_op(Op::GetField);
                                match field_pattern {
                                    Pattern::Ident(name) => {
                                        self.stmt(&Stmt::Let {
                                            pattern: DestructPattern::Ident(name.clone()),
                                            init: Expr::Null(Pos { line: 1, col: 1 }),
                                            pos: Pos { line: 1, col: 1 },
                                        })?;
                                    }
                                    Pattern::Wildcard => {
                                        self.emit_op(Op::Pop);
                                    }
                                    _ => {
                                        self.emit_op(Op::Pop);
                                    }
                                }
                            }
                            
                            // Check guard
                            if let Some(guard) = &arm.guard {
                                self.expr(guard)?;
                                let guard_fail_jump = self.emit_jump_if_false();
                                self.emit_op(Op::Pop);
                                self.expr(&arm.body)?;
                                end_jumps.push(self.emit_jump());
                                self.patch_jump(guard_fail_jump);
                            } else {
                                self.emit_op(Op::Pop);
                                self.expr(&arm.body)?;
                                end_jumps.push(self.emit_jump());
                            }
                        }
                        Pattern::EnumVariant { enum_name, variant, binding } => {
                            self.emit_op(Op::Constant);
                            let enum_idx = self.add_constant(Value::Str(enum_name.clone()));
                            self.emit_u16(enum_idx as u16);
                            self.emit_op(Op::Constant);
                            let variant_idx = self.add_constant(Value::Str(variant.clone()));
                            self.emit_u16(variant_idx as u16);
                            self.emit_op(Op::IsEnumVariant);
                            let variant_jump = self.emit_jump_if_false();

                            if let Some(binding_name) = binding {
                                self.emit_op(Op::Pop);
                                self.stmt(&Stmt::Let {
                                    pattern: DestructPattern::Ident(binding_name.clone()),
                                    init: Expr::Null(Pos { line: 1, col: 1 }),
                                    pos: Pos { line: 1, col: 1 },
                                })?;
                            } else {
                                self.emit_op(Op::Pop);
                            }
                            
                            // Check guard
                            if let Some(guard) = &arm.guard {
                                self.expr(guard)?;
                                let guard_fail_jump = self.emit_jump_if_false();
                                self.emit_op(Op::Pop);
                                self.expr(&arm.body)?;
                                end_jumps.push(self.emit_jump());
                                self.patch_jump(guard_fail_jump);
                            } else {
                                self.emit_op(Op::Pop);
                                self.expr(&arm.body)?;
                                end_jumps.push(self.emit_jump());
                            }
                            self.patch_jump(variant_jump);
                        }
                        Pattern::ResultOk { binding } => {
                            self.emit_op(Op::IsOk);
                            let result_jump = self.emit_jump_if_false();
                            self.emit_op(Op::UnwrapOk);
                            self.stmt(&Stmt::Let {
                                pattern: DestructPattern::Ident(binding.clone()),
                                init: Expr::Null(Pos { line: 1, col: 1 }),
                                pos: Pos { line: 1, col: 1 },
                            })?;
                            
                            if let Some(guard) = &arm.guard {
                                self.expr(guard)?;
                                let guard_fail_jump = self.emit_jump_if_false();
                                self.emit_op(Op::Pop);
                                self.expr(&arm.body)?;
                                end_jumps.push(self.emit_jump());
                                self.patch_jump(guard_fail_jump);
                            } else {
                                self.emit_op(Op::Pop);
                                self.expr(&arm.body)?;
                                end_jumps.push(self.emit_jump());
                            }
                            self.patch_jump(result_jump);
                        }
                        Pattern::ResultErr { binding } => {
                            self.emit_op(Op::IsErr);
                            let result_jump = self.emit_jump_if_false();
                            self.emit_op(Op::UnwrapErr);
                            self.stmt(&Stmt::Let {
                                pattern: DestructPattern::Ident(binding.clone()),
                                init: Expr::Null(Pos { line: 1, col: 1 }),
                                pos: Pos { line: 1, col: 1 },
                            })?;
                            
                            if let Some(guard) = &arm.guard {
                                self.expr(guard)?;
                                let guard_fail_jump = self.emit_jump_if_false();
                                self.emit_op(Op::Pop);
                                self.expr(&arm.body)?;
                                end_jumps.push(self.emit_jump());
                                self.patch_jump(guard_fail_jump);
                            } else {
                                self.emit_op(Op::Pop);
                                self.expr(&arm.body)?;
                                end_jumps.push(self.emit_jump());
                            }
                            self.patch_jump(result_jump);
                        }
                        Pattern::OptionSome { binding } => {
                            self.emit_op(Op::IsSome);
                            let option_jump = self.emit_jump_if_false();
                            self.emit_op(Op::UnwrapSome);
                            self.stmt(&Stmt::Let {
                                pattern: DestructPattern::Ident(binding.clone()),
                                init: Expr::Null(Pos { line: 1, col: 1 }),
                                pos: Pos { line: 1, col: 1 },
                            })?;
                            
                            if let Some(guard) = &arm.guard {
                                self.expr(guard)?;
                                let guard_fail_jump = self.emit_jump_if_false();
                                self.emit_op(Op::Pop);
                                self.expr(&arm.body)?;
                                end_jumps.push(self.emit_jump());
                                self.patch_jump(guard_fail_jump);
                            } else {
                                self.emit_op(Op::Pop);
                                self.expr(&arm.body)?;
                                end_jumps.push(self.emit_jump());
                            }
                            self.patch_jump(option_jump);
                        }
                        Pattern::OptionNone => {
                            self.emit_op(Op::IsNone);
                            let option_jump = self.emit_jump_if_false();
                            
                            if let Some(guard) = &arm.guard {
                                self.expr(guard)?;
                                let guard_fail_jump = self.emit_jump_if_false();
                                self.emit_op(Op::Pop);
                                self.expr(&arm.body)?;
                                end_jumps.push(self.emit_jump());
                                self.patch_jump(guard_fail_jump);
                            } else {
                                self.emit_op(Op::Pop);
                                self.expr(&arm.body)?;
                                end_jumps.push(self.emit_jump());
                            }
                            self.patch_jump(option_jump);
                        }
                    }
                }
                for j in end_jumps {
                    self.patch_jump(j);
                }
                self.emit_op(Op::Pop);
            }
            Expr::Result { is_ok, value, .. } => {
                self.expr(value)?;
                if *is_ok {
                    self.emit_op(Op::True);
                } else {
                    self.emit_op(Op::False);
                }
                self.emit_op(Op::MakeResult);
            }
            Expr::Option { is_some, value, .. } => {
                if *is_some {
                    if let Some(v) = value {
                        self.expr(v)?;
                    } else {
                        self.emit_op(Op::Null);
                    }
                    self.emit_op(Op::True);
                } else {
                    self.emit_op(Op::Null);
                    self.emit_op(Op::False);
                }
                self.emit_op(Op::MakeOption);
            }
            Expr::TryExpr { expr, .. } => {
                // Push try handler for error propagation
                self.emit_op(Op::PushTry);
                let catch_jump_offset_pos = self.bytecode.len();
                self.emit_u16(0); // Placeholder for catch handler offset
                let finally_jump_offset_pos = self.bytecode.len();
                self.emit_u16(0); // Placeholder for finally handler offset
                
                // Evaluate the expression
                self.expr(expr)?;
                
                // Check if Result is Ok or Option is Some
                self.emit_op(Op::IsOk);
                let is_ok_jump = self.emit_jump_if_false();
                
                // If Ok, unwrap and continue
                self.emit_op(Op::UnwrapOk);
                
                // Pop try handler
                self.emit_op(Op::PopTry);
                
                // Jump over error handling
                let success_jump = self.emit_jump();
                
                // Error handling: re-throw the error
                let catch_start = self.bytecode.len();
                let catch_offset = catch_start - catch_jump_offset_pos;
                self.bytecode[catch_jump_offset_pos] = (catch_offset & 0xff) as u8;
                self.bytecode[catch_jump_offset_pos + 1] = ((catch_offset >> 8) & 0xff) as u8;
                
                self.emit_op(Op::Throw);
                
                // Finally handler (not used for try expr, but needed for structure)
                let finally_start = self.bytecode.len();
                let finally_offset = finally_start - finally_jump_offset_pos;
                self.bytecode[finally_jump_offset_pos] = (finally_offset & 0xff) as u8;
                self.bytecode[finally_jump_offset_pos + 1] = ((finally_offset >> 8) & 0xff) as u8;
                
                // Patch jumps
                self.patch_jump(is_ok_jump);
                self.patch_jump(success_jump);
            }
            Expr::Ident(name, pos) => {
                #[cfg(debug_assertions)]
                {
                    eprintln!(
                        "DEBUG Ident: name={}, locals={:?}, globals={:?}",
                        name, self.locals, self.globals
                    );
                }
                if let Some(idx) = self.locals.iter().position(|(n, _)| n == name) {
                    #[cfg(debug_assertions)]
                    eprintln!("DEBUG Ident: using GetLocal idx={}", idx);
                    self.emit_op(Op::GetLocal);
                    self.emit_u16(idx as u16);
                } else if let Some(idx) = self.free.iter().position(|(n, _)| n == name) {
                    self.emit_op(Op::GetFree);
                    self.emit_u16(idx as u16);
                } else if let Some(&idx) = self.builtins.get(name) {
                    self.emit_op(Op::GetBuiltin);
                    self.emit_u16(idx as u16);
                } else if let Some(&(idx, _)) = self.globals.get(name) {
                    #[cfg(debug_assertions)]
                    eprintln!("DEBUG Ident: using GetGlobal idx={}", idx);
                    self.emit_op(Op::GetGlobal);
                    self.emit_u16(idx as u16);
                } else {
                    return self.undefined_var_error(name, *pos);
                }
            }
            Expr::BinOp {
                op, left, right, ..
            } => {
                if *op == BinOp::And {
                    return self.and_expr(left, right);
                }
                if *op == BinOp::Or {
                    return self.or_expr(left, right);
                }
                if *op == BinOp::Assign {
                    if let Expr::Ident(name, pos) = &**left {
                        self.check_mutability(name, *pos)?;
                        self.expr(right)?;
                        if let Some(idx) = self.locals.iter().position(|(n, _)| *n == *name) {
                            self.emit_op(Op::SetLocal);
                            self.emit_u16(idx as u16);
                        } else if let Some(idx) = self.free.iter().position(|(n, _)| *n == *name) {
                            self.emit_op(Op::SetFree);
                            self.emit_u16(idx as u16);
                        } else if let Some(&(idx, _)) = self.globals.get(name) {
                            self.emit_op(Op::SetGlobal);
                            self.emit_u16(idx as u16);
                        } else {
                            return self.undefined_var_error(name, *pos);
                        }
                        return Ok(());
                    } else if let Expr::Index { object, index, .. } = &**left {
                        self.expr(object)?;
                        self.expr(index)?;
                        self.expr(right)?;
                        self.emit_op(Op::IndexSet);
                        return Ok(());
                    } else if let Expr::Field { object, field, .. } = &**left {
                        self.expr(object)?;
                        let field_idx = self.add_constant(Value::Str(field.clone()));
                        self.emit_op(Op::Constant);
                        self.emit_u16(field_idx as u16);
                        self.expr(right)?;
                        self.emit_op(Op::SetField);
                        return Ok(());
                    }
                }
                self.expr(left)?;
                self.expr(right)?;
                self.emit_bin_op(*op);
            }
            Expr::UnaryOp { op, expr, .. } => {
                self.expr(expr)?;
                match op {
                    UnaryOp::Neg => self.emit_op(Op::Negate),
                    UnaryOp::Ref => self.emit_op(Op::Ref),
                    UnaryOp::RefMut => self.emit_op(Op::RefMut),
                    UnaryOp::Deref => self.emit_op(Op::Deref),
                }
            }
            Expr::Call { callee, args, .. } => {
                self.expr(callee)?;
                for arg in args {
                    self.expr(arg)?;
                }
                self.emit_op(Op::Call);
                self.emit_u16(args.len() as u16);
            }
            Expr::Func {
                params,
                captures,
                body,
                pos,
            } => {
                self.compile_func(params, captures, body, *pos)?;
            }
            Expr::Arrow {
                params,
                body,
                is_block,
                ..
            } => {
                if *is_block {
                    if let Expr::Func {
                        params: _,
                        captures: _,
                        body: inner_body,
                        pos: _,
                    } = &**body
                    {
                        self.compile_func(params, &[], &**inner_body, Pos { line: 1, col: 1 })?;
                    }
                } else {
                    let body_expr = (**body).clone();
                    let body_stmt = Stmt::Block(vec![Stmt::Return {
                        value: Some(body_expr),
                        pos: Pos { line: 1, col: 1 },
                    }]);
                    self.compile_func(params, &[], &body_stmt, Pos { line: 1, col: 1 })?;
                }
            }
            Expr::Pipe {
                left,
                right,
                has_placeholder,
                ..
            } => {
                if *has_placeholder {
                    #[cfg(debug_assertions)]
                    eprintln!("DEBUG: Compiling pipe with placeholder");
                    let right_expr = (**right).clone();
                    let body_stmt = Stmt::Block(vec![Stmt::Return {
                        value: Some(right_expr),
                        pos: Pos { line: 1, col: 1 },
                    }]);
                    self.compile_func(
                        &["_".to_string()],
                        &[],
                        &body_stmt,
                        Pos { line: 1, col: 1 },
                    )?;
                    self.expr(left)?;
                } else {
                    self.expr(right)?;
                    self.expr(left)?;
                }
                self.emit_op(Op::Call);
                self.emit_u16(1);
            }
        }
        Ok(())
    }

    fn and_expr(&mut self, left: &Expr, right: &Expr) -> Result<(), Error> {
        self.expr(left)?;
        let end_jump = self.emit_jump_if_false();
        self.expr(right)?;
        self.patch_jump(end_jump);
        Ok(())
    }

    fn or_expr(&mut self, left: &Expr, right: &Expr) -> Result<(), Error> {
        self.expr(left)?;
        let end_jump = self.emit_jump_if_true();
        self.expr(right)?;
        self.patch_jump(end_jump);
        Ok(())
    }

    fn compile_func(
        &mut self,
        params: &[String],
        captures: &[String],
        body: &Stmt,
        pos: Pos,
    ) -> Result<(), Error> {
        let saved_locals = self.locals.clone();
        let saved_free = self.free.clone();
        let saved_loop_stack = self.loop_stack.clone();

        self.locals = Vec::new();
        self.free = Vec::new();
        self.loop_stack = Vec::new();

        for param in params {
            self.locals.push((param.clone(), false));
        }

        let mut capture_set = HashSet::new();
        for cap in captures {
            capture_set.insert(cap.clone());
        }

        let mut vars_used = HashSet::new();
        self.find_used_vars(body, &mut vars_used);

        let mut declared = HashSet::new();
        self.find_declared_vars(body, &mut declared);

        for var in &vars_used {
            if params.contains(var) {
                continue;
            }
            if declared.contains(var) {
                continue;
            }
            if let Some((is_mut, _is_local)) = self.find_var(var, &saved_locals, &saved_free) {
                if is_mut && !capture_set.contains(var) {
                    return Err(Error::Compile {
                        pos,
                        msg: format!("mutable variable '{}' must be explicitly captured", var),
                    });
                }
                self.free.push((var.clone(), is_mut));
            }
        }

        for cap in captures {
            if !self.free.iter().any(|(n, _)| n == cap) {
                return Err(Error::Compile {
                    pos,
                    msg: format!("capture '{}' is not used in closure", cap),
                });
            }
        }

        self.func_stack.push(FuncCtx {
            locals: saved_locals.clone(),
            free: saved_free.clone(),
            captures: capture_set,
        });

        let saved_bytecode = std::mem::take(&mut self.bytecode);

        self.stmt(body)?;
        self.emit_op(Op::Return);

        let func_bytecode = std::mem::take(&mut self.bytecode);

        self.bytecode = saved_bytecode;

        let func = CompiledFunction {
            bytecode: func_bytecode,
            constants: self.constants.clone(),
            num_locals: self.locals.len() as u16,
            num_params: params.len() as u16,
            num_free: self.free.len() as u16,
        };

        let func_idx = self.add_constant(Value::Func(func));

        let free_vars = self.free.clone();
        for (name, _is_mut) in &free_vars {
            if let Some(idx) = saved_locals.iter().position(|(n, _)| n == name) {
                self.emit_op(Op::GetLocal);
                self.emit_u16(idx as u16);
            } else if let Some(idx) = saved_free.iter().position(|(n, _)| n == name) {
                self.emit_op(Op::GetFree);
                self.emit_u16(idx as u16);
            } else if let Some(&(idx, _)) = self.globals.get(name) {
                self.emit_op(Op::CaptureGlobal);
                self.emit_u16(idx as u16);
            }
        }

        self.locals = saved_locals;
        let num_free = self.free.len();
        self.free = saved_free;
        self.loop_stack = saved_loop_stack;
        self.func_stack.pop();

        self.emit_op(Op::Closure);
        self.emit_u16(func_idx as u16);
        self.emit_u16(num_free as u16);

        Ok(())
    }

    fn find_var(
        &self,
        name: &str,
        locals: &[(String, bool)],
        free: &[(String, bool)],
    ) -> Option<(bool, bool)> {
        if let Some((_, is_mut)) = locals.iter().find(|(n, _)| n == name) {
            return Some((*is_mut, true));
        }
        if let Some((_, is_mut)) = free.iter().find(|(n, _)| n == name) {
            return Some((*is_mut, false));
        }
        if let Some(&(_, is_mut)) = self.globals.get(name) {
            return Some((is_mut, false));
        }
        None
    }

    fn check_mutability(&self, name: &str, pos: Pos) -> Result<(), Error> {
        if let Some((_, is_mut)) = self.locals.iter().find(|(n, _)| n == name) {
            if !is_mut {
                return Err(Error::Compile {
                    pos,
                    msg: format!("cannot assign to immutable variable '{}'", name),
                });
            }
            return Ok(());
        }
        if let Some((_, is_mut)) = self.free.iter().find(|(n, _)| n == name) {
            if !is_mut {
                return Err(Error::Compile {
                    pos,
                    msg: format!("cannot assign to immutable variable '{}'", name),
                });
            }
            return Ok(());
        }
        if let Some(&(_, is_mut)) = self.globals.get(name) {
            if !is_mut {
                return Err(Error::Compile {
                    pos,
                    msg: format!("cannot assign to immutable variable '{}'", name),
                });
            }
            return Ok(());
        }
        Ok(())
    }

    fn find_used_vars(&self, stmt: &Stmt, vars: &mut HashSet<String>) {
        match stmt {
            Stmt::Let { pattern, init, .. } => {
                self.find_used_vars_in_expr(init, vars);
                self.find_used_vars_in_pattern(pattern, vars);
            }
            Stmt::Mut { pattern, init, .. } => {
                self.find_used_vars_in_expr(init, vars);
                self.find_used_vars_in_pattern(pattern, vars);
            }
            Stmt::Assign { target, value, .. } => {
                match target {
                    AssignTarget::Ident(name) => {
                        vars.insert(name.clone());
                    }
                    AssignTarget::Index { object, index } => {
                        self.find_used_vars_in_expr(object, vars);
                        self.find_used_vars_in_expr(index, vars);
                    }
                    AssignTarget::Field { object, .. } => {
                        self.find_used_vars_in_expr(object, vars);
                    }
                }
                self.find_used_vars_in_expr(value, vars);
            }
            Stmt::Block(stmts) => {
                for s in stmts {
                    self.find_used_vars(s, vars);
                }
            }
            Stmt::If {
                cond,
                then_blk,
                else_blk,
                ..
            } => {
                self.find_used_vars_in_expr(cond, vars);
                self.find_used_vars(then_blk, vars);
                if let Some(b) = else_blk {
                    self.find_used_vars(b, vars);
                }
            }
            Stmt::Loop { body, .. } => self.find_used_vars(body, vars),
            Stmt::While { cond, body, .. } => {
                self.find_used_vars_in_expr(cond, vars);
                self.find_used_vars(body, vars);
            }
            Stmt::For { init, cond, update, body, .. } => {
                if let Some(init) = init {
                    self.find_used_vars(init, vars);
                }
                if let Some(cond) = cond {
                    self.find_used_vars_in_expr(cond, vars);
                }
                if let Some(update) = update {
                    self.find_used_vars_in_expr(update, vars);
                }
                self.find_used_vars(body, vars);
            }
            Stmt::ForIn { iterable, body, .. } => {
                self.find_used_vars_in_expr(iterable, vars);
                self.find_used_vars(body, vars);
            }
            Stmt::Break { value, .. } => {
                if let Some(v) = value {
                    self.find_used_vars_in_expr(v, vars);
                }
            }
            Stmt::Continue { .. } => {}
            Stmt::Return { value, .. } => {
                if let Some(v) = value {
                    self.find_used_vars_in_expr(v, vars);
                }
            }
            Stmt::Throw { value, .. } => {
                self.find_used_vars_in_expr(value, vars);
            }
            Stmt::Try { try_blk, catch_blk, finally_blk, .. } => {
                self.find_used_vars(try_blk, vars);
                if let Some(c) = catch_blk {
                    self.find_used_vars(c, vars);
                }
                if let Some(f) = finally_blk {
                    self.find_used_vars(f, vars);
                }
            }
            Stmt::Expr(e) => self.find_used_vars_in_expr(e, vars),
        }
    }

    fn find_used_vars_in_expr(&self, expr: &Expr, vars: &mut HashSet<String>) {
        match expr {
            Expr::Ident(name, _) => {
                vars.insert(name.clone());
            }
            Expr::TemplateStr(parts, _) => {
                for part in parts {
                    match part {
                        TemplatePart::Literal(_) => {}
                        TemplatePart::Expr(e) => {
                            self.find_used_vars_in_expr(e, vars);
                        }
                    }
                }
            }
            Expr::Array(elements, _) => {
                for elem in elements {
                    self.find_used_vars_in_expr(elem, vars);
                }
            }
            Expr::Index { object, index, .. } => {
                self.find_used_vars_in_expr(object, vars);
                self.find_used_vars_in_expr(index, vars);
            }
            Expr::Object(fields, _) => {
                for (_, value) in fields {
                    self.find_used_vars_in_expr(value, vars);
                }
            }
            Expr::Field { object, .. } => {
                self.find_used_vars_in_expr(object, vars);
            }
            Expr::EnumVariant { value, .. } => {
                if let Some(v) = value {
                    self.find_used_vars_in_expr(v, vars);
                }
            }
            Expr::Match { expr, arms, .. } => {
                self.find_used_vars_in_expr(expr, vars);
                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        self.find_used_vars_in_expr(guard, vars);
                    }
                    self.find_used_vars_in_expr(&arm.body, vars);
                }
            }
            Expr::Result { value, .. } => {
                self.find_used_vars_in_expr(value, vars);
            }
            Expr::Option { value, .. } => {
                if let Some(v) = value {
                    self.find_used_vars_in_expr(v, vars);
                }
            }
            Expr::TryExpr { expr, .. } => {
                self.find_used_vars_in_expr(expr, vars);
            }
            Expr::BinOp { left, right, .. } => {
                self.find_used_vars_in_expr(left, vars);
                self.find_used_vars_in_expr(right, vars);
            }
            Expr::UnaryOp { expr, .. } => self.find_used_vars_in_expr(expr, vars),
            Expr::Call { callee, args, .. } => {
                self.find_used_vars_in_expr(callee, vars);
                for arg in args {
                    self.find_used_vars_in_expr(arg, vars);
                }
            }
            Expr::Func { body, .. } => self.find_used_vars(body, vars),
            Expr::Arrow { body, is_block, .. } => {
                if *is_block {
                    if let Expr::Func {
                        body: inner_body, ..
                    } = &**body
                    {
                        self.find_used_vars(inner_body, vars);
                    }
                } else {
                    self.find_used_vars_in_expr(body, vars);
                }
            }
            Expr::Pipe { left, right, .. } => {
                self.find_used_vars_in_expr(left, vars);
                self.find_used_vars_in_expr(right, vars);
            }
            _ => {}
        }
    }

    fn find_declared_vars(&self, stmt: &Stmt, declared: &mut HashSet<String>) {
        match stmt {
            Stmt::Let { pattern, .. } => {
                self.find_declared_vars_in_pattern(pattern, declared);
            }
            Stmt::Mut { pattern, .. } => {
                self.find_declared_vars_in_pattern(pattern, declared);
            }
            Stmt::Block(stmts) => {
                for s in stmts {
                    self.find_declared_vars(s, declared);
                }
            }
            Stmt::If {
                then_blk, else_blk, ..
            } => {
                self.find_declared_vars(then_blk, declared);
                if let Some(b) = else_blk {
                    self.find_declared_vars(b, declared);
                }
            }
            Stmt::Loop { body, .. } => self.find_declared_vars(body, declared),
            Stmt::While { body, .. } => self.find_declared_vars(body, declared),
            Stmt::For { init, body, .. } => {
                if let Some(init) = init {
                    self.find_declared_vars(init, declared);
                }
                self.find_declared_vars(body, declared);
            }
            Stmt::ForIn { var, body, .. } => {
                declared.insert(var.clone());
                self.find_declared_vars(body, declared);
            }
            Stmt::Try { try_blk, catch_var, catch_blk, finally_blk, .. } => {
                self.find_declared_vars(try_blk, declared);
                if let Some(var_name) = catch_var {
                    declared.insert(var_name.clone());
                }
                if let Some(c) = catch_blk {
                    self.find_declared_vars(c, declared);
                }
                if let Some(f) = finally_blk {
                    self.find_declared_vars(f, declared);
                }
            }
            _ => {}
        }
    }

    fn add_constant(&mut self, value: Value) -> usize {
        self.constants.push(value);
        self.constants.len() - 1
    }

    fn emit_op(&mut self, op: Op) {
        self.bytecode.push(op as u8);
    }

    fn emit_u16(&mut self, val: u16) {
        self.bytecode.push((val & 0xff) as u8);
        self.bytecode.push(((val >> 8) & 0xff) as u8);
    }

    fn emit_i16(&mut self, val: i16) {
        self.emit_u16(val as u16);
    }

    fn emit_bin_op(&mut self, op: BinOp) {
        match op {
            BinOp::Add => self.emit_op(Op::Add),
            BinOp::Sub => self.emit_op(Op::Sub),
            BinOp::Mul => self.emit_op(Op::Mul),
            BinOp::Div => self.emit_op(Op::Div),
            BinOp::Eq => self.emit_op(Op::Eq),
            BinOp::Neq => self.emit_op(Op::Neq),
            BinOp::Lt => self.emit_op(Op::Lt),
            BinOp::Gt => self.emit_op(Op::Gt),
            BinOp::Lte => self.emit_op(Op::Lte),
            BinOp::Gte => self.emit_op(Op::Gte),
            _ => {}
        }
    }

    fn emit_jump(&mut self) -> usize {
        self.emit_op(Op::Jump);
        let pos = self.bytecode.len();
        self.emit_i16(0);
        pos
    }

    fn emit_jump_if_false(&mut self) -> usize {
        self.emit_op(Op::JumpIfFalse);
        let pos = self.bytecode.len();
        self.emit_i16(0);
        pos
    }

    fn emit_jump_if_true(&mut self) -> usize {
        self.emit_op(Op::JumpIfTrue);
        let pos = self.bytecode.len();
        self.emit_i16(0);
        pos
    }

    fn emit_jump_if_false_pop(&mut self) -> usize {
        self.emit_op(Op::JumpIfFalsePop);
        let pos = self.bytecode.len();
        self.emit_i16(0);
        pos
    }

    fn patch_jump(&mut self, pos: usize) {
        let target = self.bytecode.len() as i16;
        let jump_instr_pos = (pos - 1) as i16;
        let offset = target - jump_instr_pos;
        #[cfg(debug_assertions)]
        {
            eprintln!("DEBUG patch_jump: pos={}, target={}, jump_instr_pos={}, offset={}, bytecode_len={}",
                pos, target, jump_instr_pos, offset, self.bytecode.len());
        }
        self.bytecode[pos] = (offset & 0xff) as u8;
        self.bytecode[pos + 1] = ((offset >> 8) & 0xff) as u8;
    }

    fn patch_continue_jumps(&mut self, target: usize) {
        if let Some((_, _, _, continue_positions)) = self.loop_stack.last() {
            let continue_positions = continue_positions.clone();
            for pos in continue_positions {
                let jump_instr_pos = (pos - 1) as i16;
                let offset = target as i16 - jump_instr_pos;
                self.bytecode[pos] = (offset & 0xff) as u8;
                self.bytecode[pos + 1] = ((offset >> 8) & 0xff) as u8;
            }
        }
    }

    fn undefined_var_error(&mut self, name: &str, pos: Pos) -> Result<(), Error> {
        let mut suggestions = Vec::new();
        let all_names: Vec<&str> = self
            .locals
            .iter()
            .map(|(n, _)| n.as_str())
            .chain(self.free.iter().map(|(n, _)| n.as_str()))
            .chain(self.globals.keys().map(|s| s.as_str()))
            .chain(self.builtins.keys().map(|s| s.as_str()))
            .collect();
        for candidate in all_names {
            let dist = levenshtein(name, candidate);
            if dist <= 2 {
                suggestions.push(candidate.to_string());
            }
        }
        suggestions.sort_by(|a, b| levenshtein(name, a).cmp(&levenshtein(name, b)));
        suggestions.truncate(3);
        let msg = if suggestions.is_empty() {
            format!("undefined variable '{}'", name)
        } else {
            format!(
                "undefined variable '{}', did you mean {}?",
                name,
                suggestions.join(", ")
            )
        };
        Err(Error::Compile { pos, msg })
    }

    fn compile_destruct_pattern(&mut self, pattern: &DestructPattern, pos: Pos) -> Result<(), Error> {
        match pattern {
            DestructPattern::Ident(name) => {
                if !self.func_stack.is_empty() {
                    if let Some(_idx) = self.locals.iter().position(|(n, _)| n == name) {
                        return Err(Error::Compile {
                            pos,
                            msg: format!("variable '{}' already declared", name),
                        });
                    }
                    self.locals.push((name.clone(), false));
                }
                if self.func_stack.is_empty() {
                    if self.builtins.contains_key(name) {
                        return Err(Error::Compile {
                            pos,
                            msg: format!("cannot shadow builtin '{}'", name),
                        });
                    }
                    if !self.globals.contains_key(name) {
                        let idx = self.next_global;
                        self.next_global += 1;
                        self.globals.insert(name.clone(), (idx, false));
                    }
                    let (idx, _) = self.globals[name];
                    self.emit_op(Op::SetGlobal);
                    self.emit_u16(idx as u16);
                } else {
                    self.emit_op(Op::SetLocal);
                    self.emit_u16((self.locals.len() - 1) as u16);
                }
            }
            DestructPattern::Array(elements) => {
                for (i, elem) in elements.iter().enumerate() {
                    // Duplicate array
                    self.emit_op(Op::Constant);
                    let idx = self.add_constant(Value::Num(i as f64));
                    self.emit_u16(idx as u16);
                    self.emit_op(Op::Index);
                    self.compile_destruct_pattern(elem, pos)?;
                }
                // Pop the array
                self.emit_op(Op::Pop);
            }
            DestructPattern::Object(fields) => {
                for (field_name, field_pattern) in fields {
                    // Get field from object
                    let field_idx = self.add_constant(Value::Str(field_name.clone()));
                    self.emit_op(Op::Constant);
                    self.emit_u16(field_idx as u16);
                    self.emit_op(Op::GetField);
                    self.compile_destruct_pattern(field_pattern, pos)?;
                }
                // Pop the object
                self.emit_op(Op::Pop);
            }
        }
        Ok(())
    }

    fn compile_destruct_pattern_mut(&mut self, pattern: &DestructPattern, pos: Pos) -> Result<(), Error> {
        match pattern {
            DestructPattern::Ident(name) => {
                if !self.func_stack.is_empty() {
                    if let Some(_idx) = self.locals.iter().position(|(n, _)| n == name) {
                        return Err(Error::Compile {
                            pos,
                            msg: format!("variable '{}' already declared", name),
                        });
                    }
                    self.locals.push((name.clone(), true));
                }
                if self.func_stack.is_empty() {
                    if self.builtins.contains_key(name) {
                        return Err(Error::Compile {
                            pos,
                            msg: format!("cannot shadow builtin '{}'", name),
                        });
                    }
                    if !self.globals.contains_key(name) {
                        let idx = self.next_global;
                        self.next_global += 1;
                        self.globals.insert(name.clone(), (idx, true));
                    }
                    let (idx, _) = self.globals[name];
                    self.emit_op(Op::SetGlobal);
                    self.emit_u16(idx as u16);
                } else {
                    self.emit_op(Op::SetLocal);
                    self.emit_u16((self.locals.len() - 1) as u16);
                }
            }
            DestructPattern::Array(elements) => {
                for (i, elem) in elements.iter().enumerate() {
                    self.emit_op(Op::Constant);
                    let idx = self.add_constant(Value::Num(i as f64));
                    self.emit_u16(idx as u16);
                    self.emit_op(Op::Index);
                    self.compile_destruct_pattern_mut(elem, pos)?;
                }
                self.emit_op(Op::Pop);
            }
            DestructPattern::Object(fields) => {
                for (field_name, field_pattern) in fields {
                    let field_idx = self.add_constant(Value::Str(field_name.clone()));
                    self.emit_op(Op::Constant);
                    self.emit_u16(field_idx as u16);
                    self.emit_op(Op::GetField);
                    self.compile_destruct_pattern_mut(field_pattern, pos)?;
                }
                self.emit_op(Op::Pop);
            }
        }
        Ok(())
    }

    fn find_declared_vars_in_pattern(&self, pattern: &DestructPattern, declared: &mut HashSet<String>) {
        match pattern {
            DestructPattern::Ident(name) => {
                declared.insert(name.clone());
            }
            DestructPattern::Array(elements) => {
                for elem in elements {
                    self.find_declared_vars_in_pattern(elem, declared);
                }
            }
            DestructPattern::Object(fields) => {
                for (_, field_pattern) in fields {
                    self.find_declared_vars_in_pattern(field_pattern, declared);
                }
            }
        }
    }

    fn find_used_vars_in_pattern(&self, pattern: &DestructPattern, vars: &mut HashSet<String>) {
        // DestructPattern doesn't use variables, it declares them
        let _ = (pattern, vars);
    }
}
