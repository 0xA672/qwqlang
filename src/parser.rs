use crate::ast::{AssignTarget, BinOp, DestructPattern, Expr, MatchArm, Pattern, Pos, Stmt, TemplatePart, Type, UnaryOp};
use crate::error::Error;
use crate::lexer::{Lex, Tok};
use std::collections::HashMap;

#[derive(Debug)]
pub struct P<'a> {
    lex: Lex<'a>,
    cur: Tok,
    labels: HashMap<String, Pos>,
    loop_stack: Vec<Option<String>>,
    input: &'a str,
    // When true, the parser is inside an array/object/paren container where
    // commas separate elements, so `pipe()` must NOT treat `,` as a multi-arg
    // pipe separator.
    in_container: bool,
    // Lookahead buffer for 1-token peek.
    lookahead: Option<Tok>,
}

fn peek_next_two(scan: &mut Lex) -> (Tok, Tok) {
    let first = scan.next();
    let second = scan.next();
    (first, second)
}

impl<'a> P<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut lex = Lex::new(input);
        let cur = lex.next();
        P {
            lex,
            cur,
            labels: HashMap::new(),
            loop_stack: Vec::new(),
            input,
            in_container: false,
            lookahead: None,
        }
    }

    pub fn parse(&mut self) -> Result<Vec<Stmt>, Error> {
        let mut stmts = Vec::new();
        while !matches!(self.cur, Tok::Eof(_)) {
            stmts.push(self.stmt()?);
        }
        Ok(stmts)
    }

    fn stmt(&mut self) -> Result<Stmt, Error> {
        let tok = self.cur.clone();
        match tok {
            Tok::Let(pos) => self.let_stmt(pos),
            Tok::Mut(pos) => self.mut_stmt(pos),
            Tok::If(pos) => self.if_expr(pos),
            Tok::Loop(pos) => self.loop_stmt(pos),
            Tok::While(pos) => self.while_stmt(pos),
            Tok::For(pos) => self.for_stmt(pos),
            Tok::Label(name, pos) => self.label_loop(&name, pos),
            Tok::Return(pos) => self.return_stmt(pos),
            Tok::Break(pos) => self.break_stmt(pos),
            Tok::Continue(pos) => self.continue_stmt(pos),
            Tok::Match(pos) => self.match_stmt(pos),
            Tok::Throw(pos) => self.throw_stmt(pos),
            Tok::Try(pos) => self.try_stmt(pos),
            Tok::LBrace(_) => {
                // Disambiguate: `{` could be a block or an object literal.
                // Object literal if next token is `Str` (string key dict).
                if matches!(self.peek_next(), Tok::Str(_, _)) {
                    let e = self.expr()?;
                    let tok2 = self.cur.clone();
                    match tok2 {
                        Tok::Semicolon(_) => {
                            self.consume();
                            Ok(Stmt::Expr(e))
                        }
                        _ => Ok(Stmt::Expr(e)),
                    }
                } else {
                    self.block()
                }
            }
            Tok::Semicolon(pos) => {
                self.consume();
                Ok(Stmt::Expr(Expr::Null(pos)))
            }
            Tok::Fn(pos) => self.fn_stmt(pos),
            Tok::PipeSingle(_) => {
                let e = self.expr()?;
                Ok(Stmt::Expr(e))
            }
            _ => {
                let e = self.expr()?;
                let tok2 = self.cur.clone();
                match tok2 {
                    Tok::Semicolon(_) => {
                        self.consume();
                        Ok(Stmt::Expr(e))
                    }
                    Tok::Assign(pos) => {
                        let target = self.expr_to_assign_target(e)?;
                        self.consume();
                        let value = self.expr()?;
                        self.consume_semicolon()?;
                        Ok(Stmt::Assign { target, value, pos })
                    }
                    _ => Ok(Stmt::Expr(e)),
                }
            }
        }
    }

    fn let_stmt(&mut self, pos: Pos) -> Result<Stmt, Error> {
        self.consume();
        let pattern = self.destruct_pattern()?;
        let type_anno = self.parse_opt_type();
        self.expect_assign()?;
        let init = self.expr()?;
        self.consume_semicolon()?;
        Ok(Stmt::Let { pattern, type_anno, init, pos })
    }

    fn mut_stmt(&mut self, pos: Pos) -> Result<Stmt, Error> {
        self.consume();
        let pattern = self.destruct_pattern()?;
        let type_anno = self.parse_opt_type();
        self.expect_assign()?;
        let init = self.expr()?;
        self.consume_semicolon()?;
        Ok(Stmt::Mut { pattern, type_anno, init, pos })
    }

    fn destruct_pattern(&mut self) -> Result<DestructPattern, Error> {
        let tok = self.cur.clone();
        match tok {
            Tok::LBracket(_) => {
                self.consume();
                let mut elements = Vec::new();
                if !matches!(self.cur, Tok::RBracket(_)) {
                    elements.push(self.destruct_pattern()?);
                    while let Tok::Comma(_) = self.cur.clone() {
                        self.consume();
                        if matches!(self.cur, Tok::RBracket(_)) {
                            break;
                        }
                        elements.push(self.destruct_pattern()?);
                    }
                }
                self.expect_rbracket()?;
                Ok(DestructPattern::Array(elements))
            }
            Tok::LBrace(_) => {
                self.consume();
                let mut fields = Vec::new();
                if !matches!(self.cur, Tok::RBrace(_)) {
                    let name = self.expect_ident()?;
                    let pattern = if matches!(self.cur, Tok::Colon(_)) {
                        self.consume();
                        self.destruct_pattern()?
                    } else {
                        DestructPattern::Ident(name.clone())
                    };
                    fields.push((name, pattern));
                    while let Tok::Comma(_) = self.cur.clone() {
                        self.consume();
                        if matches!(self.cur, Tok::RBrace(_)) {
                            break;
                        }
                        let name = self.expect_ident()?;
                        let pattern = if matches!(self.cur, Tok::Colon(_)) {
                            self.consume();
                            self.destruct_pattern()?
                        } else {
                            DestructPattern::Ident(name.clone())
                        };
                        fields.push((name, pattern));
                    }
                }
                self.expect_rbrace()?;
                Ok(DestructPattern::Object(fields))
            }
            Tok::Ident(name, _) => {
                self.consume();
                Ok(DestructPattern::Ident(name))
            }
            _ => Err(Error::Syntax {
            filename: None,
                pos: self.pos(),
                msg: format!("expected destruct pattern, got {}", tok.describe()),
                input: self.input.to_string(),
            }),
        }
    }

    fn expr_to_assign_target(&self, expr: Expr) -> Result<AssignTarget, Error> {
        match expr {
            Expr::Ident(name, _) => Ok(AssignTarget::Ident(name)),
            Expr::Index { object, index, .. } => Ok(AssignTarget::Index { object, index }),
            Expr::Field { object, field, .. } => Ok(AssignTarget::Field { object, field }),
            _ => Err(Error::Compile {
            input: None,
            filename: None,
                pos: expr.pos(),
                msg: "cannot assign to this expression".to_string(),
            }),
        }
    }

    fn fn_stmt(&mut self, pos: Pos) -> Result<Stmt, Error> {
        self.consume();
        let name = self.expect_ident()?;
        self.expect_lparen()?;
        let mut params = Vec::new();
        if !matches!(self.cur, Tok::RParen(_)) {
            params.push((self.expect_ident()?, None));
            while let Tok::Comma(_) = self.cur.clone() {
                self.consume();
                params.push((self.expect_ident()?, None));
            }
        }
        self.expect_rparen()?;
        let mut captures = Vec::new();
        if let Tok::LBracket(_) = self.cur.clone() {
            self.consume();
            if !matches!(self.cur, Tok::RBracket(_)) {
                captures.push(self.parse_capture()?);
                while let Tok::Comma(_) = self.cur.clone() {
                    self.consume();
                    captures.push(self.parse_capture()?);
                }
            }
            self.expect_rbracket()?;
        }
        let body = Box::new(self.block()?);
        self.consume_semicolon()?;
        Ok(Stmt::Let {
            pattern: DestructPattern::Ident(name),
            type_anno: None,
            init: Expr::Func {
                params,
                ret_type: None,
                captures,
                body,
                pos,
            },
            pos,
        })
    }

    fn if_expr(&mut self, pos: Pos) -> Result<Stmt, Error> {
        self.consume();
        self.expect_lparen()?;
        let cond = self.expr()?;
        self.expect_rparen()?;
        let then_blk = Box::new(self.block()?);
        let else_blk = if matches!(self.cur, Tok::Else(_)) {
            self.consume();
            Some(Box::new(self.block()?))
        } else {
            None
        };
        Ok(Stmt::If {
            cond,
            then_blk,
            else_blk,
            pos,
        })
    }

    fn label_loop(&mut self, name: &str, pos: Pos) -> Result<Stmt, Error> {
        self.consume();
        self.expect_loop()?;
        self.labels.insert(name.to_string(), pos);
        self.loop_stack.push(Some(name.to_string()));
        let body = Box::new(self.block()?);
        self.loop_stack.pop();
        Ok(Stmt::Loop {
            label: Some(name.to_string()),
            body,
            pos,
        })
    }

    fn loop_stmt(&mut self, pos: Pos) -> Result<Stmt, Error> {
        self.consume();
        self.loop_stack.push(None);
        let body = Box::new(self.block()?);
        self.loop_stack.pop();
        Ok(Stmt::Loop {
            label: None,
            body,
            pos,
        })
    }

    fn while_stmt(&mut self, pos: Pos) -> Result<Stmt, Error> {
        self.consume();
        self.expect_lparen()?;
        let cond = self.expr()?;
        self.expect_rparen()?;
        self.loop_stack.push(None);
        let body = Box::new(self.block()?);
        self.loop_stack.pop();
        Ok(Stmt::While {
            cond,
            body,
            pos,
        })
    }

    fn for_stmt(&mut self, pos: Pos) -> Result<Stmt, Error> {
        self.consume();

        // Support both `for (let/mut var in iterable) { body }` and
        // `for var in iterable { body }` (no parens, no let/mut).
        let has_paren = matches!(self.cur, Tok::LParen(_));
        if has_paren {
            self.consume();
        }

        if matches!(self.cur, Tok::Let(_)) || matches!(self.cur, Tok::Mut(_)) {
            self.consume();
        }

        // Check if this looks like a for-in loop: identifier followed by 'in'
        if matches!(self.cur, Tok::Ident(_, _)) {
            let saved = self.cur.clone();
            self.consume();
            if matches!(self.cur, Tok::In(_)) {
                let var = match saved {
                    Tok::Ident(name, _) => name,
                    _ => unreachable!(),
                };
                self.consume();
                let iterable = self.expr()?;
                if has_paren {
                    self.expect_rparen()?;
                }
                self.loop_stack.push(None);
                let body = Box::new(self.block()?);
                self.loop_stack.pop();
                return Ok(Stmt::ForIn {
                    var,
                    iterable,
                    body,
                    pos,
                });
            } else {
                // Not a for-in, backtrack
                self.cur = saved;
            }
        }

        if !has_paren {
            return Err(Error::Compile {
            input: None,
            filename: None,
                pos,
                msg: "expected 'for var in iterable' or 'for (;cond;update)'".to_string(),
            });
        }

        let init = if matches!(self.cur, Tok::Semicolon(_)) {
            self.consume();
            None
        } else {
            return Err(Error::Compile {
            input: None,
            filename: None,
                pos,
                msg: "for loop init must be a semicolon".to_string(),
            });
        };
        let cond = if matches!(self.cur, Tok::Semicolon(_)) {
            self.consume();
            None
        } else {
            Some(self.expr()?)
        };
        self.expect_semicolon()?;
        let update = if matches!(self.cur, Tok::RParen(_)) {
            None
        } else {
            Some(self.expr()?)
        };
        self.expect_rparen()?;
        self.loop_stack.push(None);
        let body = Box::new(self.block()?);
        self.loop_stack.pop();
        Ok(Stmt::For {
            init,
            cond,
            update,
            body,
            pos,
        })
    }

    fn break_stmt(&mut self, pos: Pos) -> Result<Stmt, Error> {
        self.consume();
        let tok = self.cur.clone();
        let label = if let Tok::Label(name, _) = tok {
            let label = name;
            self.consume();
            if !self.loop_stack.iter().any(|l| l.as_deref() == Some(&label)) {
                return Err(Error::Compile {
            input: None,
            filename: None,
                    pos,
                    msg: format!("undefined label '{}'", label),
                });
            }
            Some(label)
        } else {
            None
        };
        let value = if !matches!(self.cur, Tok::Semicolon(_) | Tok::RBrace(_) | Tok::Eof(_)) {
            Some(self.expr()?)
        } else {
            None
        };
        self.consume_semicolon()?;
        if self.loop_stack.is_empty() && label.is_none() {
            return Err(Error::Compile {
            input: None,
            filename: None,
                pos,
                msg: "break outside loop".to_string(),
            });
        }
        Ok(Stmt::Break { label, value, pos })
    }

    fn return_stmt(&mut self, pos: Pos) -> Result<Stmt, Error> {
        self.consume();
        let value = if !matches!(self.cur, Tok::Semicolon(_) | Tok::RBrace(_) | Tok::Eof(_)) {
            Some(self.expr()?)
        } else {
            None
        };
        self.consume_semicolon()?;
        Ok(Stmt::Return { value, pos })
    }

    fn continue_stmt(&mut self, pos: Pos) -> Result<Stmt, Error> {
        self.consume();
        let label = if let Tok::Label(name, _) = self.cur.clone() {
            let label = name;
            self.consume();
            if !self.loop_stack.iter().any(|l| l.as_deref() == Some(&label)) {
                return Err(Error::Compile {
            input: None,
            filename: None,
                    pos,
                    msg: format!("undefined label '{}'", label),
                });
            }
            Some(label)
        } else {
            None
        };
        self.consume_semicolon()?;
        if self.loop_stack.is_empty() && label.is_none() {
            return Err(Error::Compile {
            input: None,
            filename: None,
                pos,
                msg: "continue outside loop".to_string(),
            });
        }
        Ok(Stmt::Continue { label, pos })
    }

    fn match_stmt(&mut self, pos: Pos) -> Result<Stmt, Error> {
        self.consume();
        self.expect_lparen()?;
        let expr = self.expr()?;
        self.expect_rparen()?;
        self.expect_lbrace()?;
        let mut arms = Vec::new();
        while !matches!(self.cur, Tok::RBrace(_) | Tok::Eof(_)) {
            let pattern = self.pattern()?;
            let guard = if matches!(self.cur, Tok::If(_)) {
                self.consume();
                Some(self.expr()?)
            } else {
                None
            };
            self.expect_fat_arrow()?;
            let body = self.expr()?;
            self.consume_semicolon()?;
            arms.push(MatchArm { pattern, guard, body });
        }
        self.expect_rbrace()?;
        Ok(Stmt::Expr(Expr::Match {
            expr: Box::new(expr),
            arms,
            pos,
        }))
    }

    fn throw_stmt(&mut self, pos: Pos) -> Result<Stmt, Error> {
        self.consume();
        let value = self.expr()?;
        self.consume_semicolon()?;
        Ok(Stmt::Throw { value, pos })
    }

    fn try_stmt(&mut self, pos: Pos) -> Result<Stmt, Error> {
        self.consume();
        let try_blk = Box::new(self.block()?);
        let (catch_var, catch_blk) = if matches!(self.cur, Tok::Catch(_)) {
            self.consume();
            let var = if matches!(self.cur, Tok::LParen(_)) {
                self.consume();
                let name = self.expect_ident()?;
                self.expect_rparen()?;
                Some(name)
            } else {
                None
            };
            let blk = Box::new(self.block()?);
            (var, Some(blk))
        } else {
            (None, None)
        };
        let finally_blk = if matches!(self.cur, Tok::Finally(_)) {
            self.consume();
            Some(Box::new(self.block()?))
        } else {
            None
        };
        Ok(Stmt::Try {
            try_blk,
            catch_var,
            catch_blk,
            finally_blk,
            pos,
        })
    }

    fn expect_lbrace(&mut self) -> Result<(), Error> {
        if matches!(self.cur, Tok::LBrace(_)) {
            self.consume();
            Ok(())
        } else {
            Err(Error::Syntax {
            filename: None,
                pos: self.pos(),
                msg: format!("expected '{{', found {}", self.cur.describe()),
                input: self.input.to_string(),
            })
        }
    }

    fn pattern(&mut self) -> Result<Pattern, Error> {
        let tok = self.cur.clone();
        match tok {
            Tok::Ident(name, _) if name == "_" => {
                self.consume();
                Ok(Pattern::Wildcard)
            }
            Tok::Ok(_) => {
                self.consume();
                self.expect_lparen()?;
                let binding = self.expect_ident()?;
                self.expect_rparen()?;
                Ok(Pattern::ResultOk { binding })
            }
            Tok::Err(_) => {
                self.consume();
                self.expect_lparen()?;
                let binding = self.expect_ident()?;
                self.expect_rparen()?;
                Ok(Pattern::ResultErr { binding })
            }
            Tok::Some(_) => {
                self.consume();
                self.expect_lparen()?;
                let binding = self.expect_ident()?;
                self.expect_rparen()?;
                Ok(Pattern::OptionSome { binding })
            }
            Tok::None(_) => {
                self.consume();
                Ok(Pattern::OptionNone)
            }
            Tok::Ident(name, _) => {
                self.consume();
                if matches!(self.cur, Tok::DoubleColon(_)) {
                    self.consume();
                    let variant = self.expect_ident()?;
                    if matches!(self.cur, Tok::LParen(_)) {
                        self.consume();
                        let binding = self.expect_ident()?;
                        self.expect_rparen()?;
                        Ok(Pattern::EnumVariant {
                            enum_name: name,
                            variant,
                            binding: Some(binding),
                        })
                    } else {
                        Ok(Pattern::EnumVariant {
                            enum_name: name,
                            variant,
                            binding: None,
                        })
                    }
                } else {
                    Ok(Pattern::Ident(name))
                }
            }
            Tok::LBracket(_) => {
                self.consume();
                let mut elements = Vec::new();
                if !matches!(self.cur, Tok::RBracket(_)) {
                    elements.push(self.pattern()?);
                    while let Tok::Comma(_) = self.cur.clone() {
                        self.consume();
                        if matches!(self.cur, Tok::RBracket(_)) {
                            break;
                        }
                        elements.push(self.pattern()?);
                    }
                }
                self.expect_rbracket()?;
                Ok(Pattern::Array(elements))
            }
            Tok::LBrace(_) => {
                self.consume();
                let mut fields = Vec::new();
                if !matches!(self.cur, Tok::RBrace(_)) {
                    let name = self.expect_ident()?;
                    let pattern = if matches!(self.cur, Tok::Colon(_)) {
                        self.consume();
                        self.pattern()?
                    } else {
                        Pattern::Ident(name.clone())
                    };
                    fields.push((name, pattern));
                    while let Tok::Comma(_) = self.cur.clone() {
                        self.consume();
                        if matches!(self.cur, Tok::RBrace(_)) {
                            break;
                        }
                        let name = self.expect_ident()?;
                        let pattern = if matches!(self.cur, Tok::Colon(_)) {
                            self.consume();
                            self.pattern()?
                        } else {
                            Pattern::Ident(name.clone())
                        };
                        fields.push((name, pattern));
                    }
                }
                self.expect_rbrace()?;
                Ok(Pattern::Object(fields))
            }
            Tok::True(pos) => {
                self.consume();
                Ok(Pattern::Literal(Expr::Bool(true, pos)))
            }
            Tok::False(pos) => {
                self.consume();
                Ok(Pattern::Literal(Expr::Bool(false, pos)))
            }
            Tok::Null(pos) => {
                self.consume();
                Ok(Pattern::Literal(Expr::Null(pos)))
            }
            Tok::Num(n, pos) => {
                self.consume();
                Ok(Pattern::Literal(Expr::Num(n, pos)))
            }
            Tok::Str(s, pos) => {
                self.consume();
                Ok(Pattern::Literal(Expr::Str(s, pos)))
            }
            _ => Err(Error::Syntax {
            filename: None,
                pos: self.pos(),
                msg: format!("unexpected token in pattern: {}", tok.describe()),
                input: self.input.to_string(),
            }),
        }
    }

    fn block(&mut self) -> Result<Stmt, Error> {
        self.consume();
        let mut stmts = Vec::new();
        while !matches!(self.cur, Tok::RBrace(_) | Tok::Eof(_)) {
            stmts.push(self.stmt()?);
        }
        self.expect_rbrace()?;
        Ok(Stmt::Block(stmts))
    }

    fn expr(&mut self) -> Result<Expr, Error> {
        self.or()
    }

    fn or(&mut self) -> Result<Expr, Error> {
        let mut left = self.and()?;
        while let Tok::Or(pos) = self.cur.clone() {
            self.consume();
            let right = self.and()?;
            left = Expr::BinOp {
                op: BinOp::Or,
                left: Box::new(left),
                right: Box::new(right),
                pos,
            };
        }
        Ok(left)
    }

    fn and(&mut self) -> Result<Expr, Error> {
        let mut left = self.assign()?;
        while let Tok::And(pos) = self.cur.clone() {
            self.consume();
            let right = self.assign()?;
            left = Expr::BinOp {
                op: BinOp::And,
                left: Box::new(left),
                right: Box::new(right),
                pos,
            };
        }
        Ok(left)
    }

    fn assign(&mut self) -> Result<Expr, Error> {
        let left = self.pipe()?;
        if let Tok::Assign(pos) = self.cur.clone() {
            self.consume();
            let right = self.assign()?;
            Ok(Expr::BinOp {
                op: BinOp::Assign,
                left: Box::new(left),
                right: Box::new(right),
                pos,
            })
        } else {
            Ok(left)
        }
    }

    fn pipe(&mut self) -> Result<Expr, Error> {
        // Handle nullary pipe: |> f
        if matches!(self.cur, Tok::Pipe(_)) {
            let pos = match &self.cur {
                Tok::Pipe(p) => *p,
                _ => unreachable!(),
            };
            self.consume();
            let func = self.equality()?;
            let has_placeholder = self.has_placeholder(&func);

            // Check for chained nullary pipe: |> f |> g
            if matches!(self.cur, Tok::Pipe(_)) {
                let first = Expr::Pipe {
                    args: vec![],
                    func: Box::new(func),
                    has_placeholder,
                    pos,
                };
                self.consume();
                let remaining = self.pipe()?;
                let remaining_has_placeholder = self.has_placeholder(&remaining);
                return Ok(Expr::Pipe {
                    args: vec![first],
                    func: Box::new(remaining),
                    has_placeholder: remaining_has_placeholder,
                    pos: Pos { line: 1, col: 1 },
                });
            }

            return Ok(Expr::Pipe {
                args: vec![],
                func: Box::new(func),
                has_placeholder,
                pos,
            });
        }

        let first = self.equality()?;

        match self.cur.clone() {
            Tok::Pipe(pos) => {
                let mut args = vec![first];
                self.consume();

                while matches!(self.cur, Tok::Comma(_)) {
                    self.consume();
                    args.push(self.equality()?);
                }

                let func = self.equality()?;
                let has_placeholder = self.has_placeholder(&func);

                // `x |> f()` should mean `f(x)`: if func is an empty-arg
                // call (Pipe{args:[], func:g}), fold our args into it.
                let (args, func) = match func {
                    Expr::Pipe { args: inner_args, func: inner_func, .. } if inner_args.is_empty() && !self.has_placeholder(&inner_func) => {
                        (args, *inner_func)
                    }
                    other => (args, other),
                };

                let pipe_expr = Expr::Pipe {
                    args,
                    func: Box::new(func),
                    has_placeholder,
                    pos,
                };

                if matches!(self.cur, Tok::Pipe(_)) {
                    self.consume();
                    let remaining = self.pipe()?;
                    let remaining_has_placeholder = self.has_placeholder(&remaining);
                    return Ok(Expr::Pipe {
                        args: vec![pipe_expr],
                        func: Box::new(remaining),
                        has_placeholder: remaining_has_placeholder,
                        pos: Pos { line: 1, col: 1 },
                    });
                }

                Ok(pipe_expr)
            }
            Tok::Comma(_) => {
                // Inside array/object/paren containers, commas separate
                // elements and must NOT be consumed as pipe arg separators.
                if self.in_container {
                    return Ok(first);
                }
                // Multi-argument pipe: a, b, c |> f
                let mut args = vec![first];
                self.consume();
                args.push(self.equality()?);

                while matches!(self.cur, Tok::Comma(_)) {
                    self.consume();
                    args.push(self.equality()?);
                }

                if let Tok::Pipe(pos) = self.cur.clone() {
                    self.consume();
                    let func = self.equality()?;
                    let has_placeholder = self.has_placeholder(&func);

                    let pipe_expr = Expr::Pipe {
                        args,
                        func: Box::new(func),
                        has_placeholder,
                        pos,
                    };

                    if matches!(self.cur, Tok::Pipe(_)) {
                        self.consume();
                        let remaining = self.pipe()?;
                        let remaining_has_placeholder = self.has_placeholder(&remaining);
                        return Ok(Expr::Pipe {
                            args: vec![pipe_expr],
                            func: Box::new(remaining),
                            has_placeholder: remaining_has_placeholder,
                            pos: Pos { line: 1, col: 1 },
                        });
                    }

                    return Ok(pipe_expr);
                }

                // No pipe follows the comma-list: not a valid expression form.
                // Return the first element and let the caller decide what to do
                // with the remaining commas (typically a syntax error upstream).
                if args.len() == 1 {
                    Ok(args.into_iter().next().unwrap())
                } else {
                    Ok(args.remove(0))
                }
            }
            _ => Ok(first),
        }
    }

    fn has_placeholder(&self, e: &Expr) -> bool {
        match e {
            Expr::Ident(name, _) => name == "_",
            Expr::BinOp { left, right, .. } => {
                self.has_placeholder(left) || self.has_placeholder(right)
            }
            Expr::UnaryOp { expr, .. } => self.has_placeholder(expr),
            Expr::Pipe { func, .. } => self.has_placeholder(func),
            Expr::Func { body, .. } => self.has_placeholder_in_stmt(body),
            Expr::Arrow { body, .. } => self.has_placeholder(body),
            Expr::TemplateStr(parts, _) => parts.iter().any(|p| {
                if let TemplatePart::Expr(expr) = p {
                    self.has_placeholder(expr)
                } else {
                    false
                }
            }),
            Expr::Result { value, .. } => self.has_placeholder(value),
            Expr::Option { value, .. } => value.as_ref().map_or(false, |v| self.has_placeholder(v)),
            Expr::TryExpr { expr, .. } => self.has_placeholder(expr),
            Expr::Array(elements, _) => elements.iter().any(|e| self.has_placeholder(e)),
            Expr::Object(fields, _) => fields.iter().any(|(_, v)| self.has_placeholder(v)),
            Expr::Index { object, index, .. } => {
                self.has_placeholder(object) || self.has_placeholder(index)
            }
            Expr::Field { object, .. } => self.has_placeholder(object),
            Expr::Match { arms, .. } => arms.iter().any(|arm| self.has_placeholder(&arm.body)),
            _ => false,
        }
    }

    fn has_placeholder_in_stmt(&self, s: &Stmt) -> bool {
        match s {
            Stmt::Let { init, .. } => self.has_placeholder(init),
            Stmt::Mut { init, .. } => self.has_placeholder(init),
            Stmt::Assign { value, .. } => self.has_placeholder(value),
            Stmt::Block(stmts) => stmts.iter().any(|s| self.has_placeholder_in_stmt(s)),
            Stmt::If {
                cond,
                then_blk,
                else_blk,
                ..
            } => {
                self.has_placeholder(cond)
                    || self.has_placeholder_in_stmt(then_blk)
                    || else_blk
                        .as_ref()
                        .map_or(false, |b| self.has_placeholder_in_stmt(b))
            }
            Stmt::Loop { body, .. } => self.has_placeholder_in_stmt(body),
            Stmt::While { cond, body, .. } => {
                self.has_placeholder(cond) || self.has_placeholder_in_stmt(body)
            }
            Stmt::For { init, cond, update, body, .. } => {
                let init_has = init.as_ref().map_or(false, |i| self.has_placeholder_in_stmt(i));
                let cond_has = cond.as_ref().map_or(false, |c| self.has_placeholder(c));
                let update_has = update.as_ref().map_or(false, |u| self.has_placeholder(u));
                init_has || cond_has || update_has || self.has_placeholder_in_stmt(body)
            }
            Stmt::ForIn { iterable, body, .. } => {
                self.has_placeholder(iterable) || self.has_placeholder_in_stmt(body)
            }
            Stmt::Break { value, .. } => value.as_ref().map_or(false, |v| self.has_placeholder(v)),
            Stmt::Continue { .. } => false,
            Stmt::Return { value, .. } => value.as_ref().map_or(false, |v| self.has_placeholder(v)),
            Stmt::Throw { value, .. } => self.has_placeholder(value),
            Stmt::Try { try_blk, catch_blk, finally_blk, .. } => {
                self.has_placeholder_in_stmt(try_blk)
                    || catch_blk.as_ref().map_or(false, |b| self.has_placeholder_in_stmt(b))
                    || finally_blk.as_ref().map_or(false, |b| self.has_placeholder_in_stmt(b))
            }
            Stmt::Expr(e) => self.has_placeholder(e),
        }
    }

    fn equality(&mut self) -> Result<Expr, Error> {
        let mut left = self.comparison()?;
        loop {
            let tok = self.cur.clone();
            match tok {
                Tok::Eq(pos) => {
                    self.consume();
                    let right = self.comparison()?;
                    left = Expr::BinOp {
                        op: BinOp::Eq,
                        left: Box::new(left),
                        right: Box::new(right),
                        pos,
                    };
                }
                Tok::Neq(pos) => {
                    self.consume();
                    let right = self.comparison()?;
                    left = Expr::BinOp {
                        op: BinOp::Neq,
                        left: Box::new(left),
                        right: Box::new(right),
                        pos,
                    };
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn comparison(&mut self) -> Result<Expr, Error> {
        let mut left = self.additive()?;
        loop {
            let tok = self.cur.clone();
            match tok {
                Tok::Lt(pos) => {
                    self.consume();
                    let right = self.additive()?;
                    left = Expr::BinOp {
                        op: BinOp::Lt,
                        left: Box::new(left),
                        right: Box::new(right),
                        pos,
                    };
                }
                Tok::Gt(pos) => {
                    self.consume();
                    let right = self.additive()?;
                    left = Expr::BinOp {
                        op: BinOp::Gt,
                        left: Box::new(left),
                        right: Box::new(right),
                        pos,
                    };
                }
                Tok::Lte(pos) => {
                    self.consume();
                    let right = self.additive()?;
                    left = Expr::BinOp {
                        op: BinOp::Lte,
                        left: Box::new(left),
                        right: Box::new(right),
                        pos,
                    };
                }
                Tok::Gte(pos) => {
                    self.consume();
                    let right = self.additive()?;
                    left = Expr::BinOp {
                        op: BinOp::Gte,
                        left: Box::new(left),
                        right: Box::new(right),
                        pos,
                    };
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn additive(&mut self) -> Result<Expr, Error> {
        let mut left = self.multiplicative()?;
        loop {
            let tok = self.cur.clone();
            match tok {
                Tok::Add(pos) => {
                    self.consume();
                    let right = self.multiplicative()?;
                    left = Expr::BinOp {
                        op: BinOp::Add,
                        left: Box::new(left),
                        right: Box::new(right),
                        pos,
                    };
                }
                Tok::Sub(pos) => {
                    self.consume();
                    let right = self.multiplicative()?;
                    left = Expr::BinOp {
                        op: BinOp::Sub,
                        left: Box::new(left),
                        right: Box::new(right),
                        pos,
                    };
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn multiplicative(&mut self) -> Result<Expr, Error> {
        let mut left = self.unary()?;
        loop {
            let tok = self.cur.clone();
            match tok {
                Tok::Mul(pos) => {
                    self.consume();
                    let right = self.unary()?;
                    left = Expr::BinOp {
                        op: BinOp::Mul,
                        left: Box::new(left),
                        right: Box::new(right),
                        pos,
                    };
                }
                Tok::Div(pos) => {
                    self.consume();
                    let right = self.unary()?;
                    left = Expr::BinOp {
                        op: BinOp::Div,
                        left: Box::new(left),
                        right: Box::new(right),
                        pos,
                    };
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<Expr, Error> {
        let tok = self.cur.clone();
        match tok {
            Tok::Sub(pos) => {
                self.consume();
                let expr = self.unary()?;
                Ok(Expr::UnaryOp {
                    op: UnaryOp::Neg,
                    expr: Box::new(expr),
                    pos,
                })
            }
            Tok::Ref(pos) => {
                self.consume();
                let is_mut = matches!(self.cur, Tok::Mut(_));
                if is_mut {
                    self.consume();
                }
                let expr = self.unary()?;
                Ok(Expr::UnaryOp {
                    op: if is_mut { UnaryOp::RefMut } else { UnaryOp::Ref },
                    expr: Box::new(expr),
                    pos,
                })
            }
            Tok::Mul(pos) => {
                self.consume();
                let expr = self.unary()?;
                Ok(Expr::UnaryOp {
                    op: UnaryOp::Deref,
                    expr: Box::new(expr),
                    pos,
                })
            }
            _ => self.call(),
        }
    }

    fn try_parse_first_arg(&mut self) -> Option<Expr> {
        let tok = self.cur.clone();
        match tok {
            Tok::Str(s, pos) => {
                self.consume();
                Some(Expr::Str(s, pos))
            }
            Tok::Num(n, pos) => {
                self.consume();
                Some(Expr::Num(n, pos))
            }
            Tok::True(pos) => {
                self.consume();
                Some(Expr::Bool(true, pos))
            }
            Tok::False(pos) => {
                self.consume();
                Some(Expr::Bool(false, pos))
            }
            Tok::Null(pos) => {
                self.consume();
                Some(Expr::Null(pos))
            }
            Tok::LBracket(_) => {
                let arr = self.array_literal().ok()?;
                Some(arr)
            }
            Tok::LBrace(_) => {
                let obj = self.object_literal().ok()?;
                Some(obj)
            }
            Tok::Ident(_, _) => {
                let arg = self.equality().ok()?;
                Some(arg)
            }
            _ => None,
        }
    }

    fn call(&mut self) -> Result<Expr, Error> {
        let mut callee = self.primary()?;
        loop {
            let tok = self.cur.clone();
            match tok {
                Tok::LBracket(pos) => {
                    self.consume();
                    let prev = self.in_container;
                    self.in_container = true;
                    let index = self.expr()?;
                    self.in_container = prev;
                    self.expect_rbracket()?;
                    callee = Expr::Index {
                        object: Box::new(callee),
                        index: Box::new(index),
                        pos,
                    };
                }
                Tok::Dot(pos) => {
                    self.consume();
                    let field = self.expect_ident()?;
                    callee = Expr::Field {
                        object: Box::new(callee),
                        field,
                        pos,
                    };
                    if let Some(first_arg) = self.try_parse_first_arg() {
                        callee = Expr::Pipe {
                            args: vec![first_arg],
                            func: Box::new(callee),
                            has_placeholder: false,
                            pos,
                        };
                    }
                }
                Tok::LParen(pos) => {
                    // Function call suffix: f(args...)
                    self.consume();
                    let mut args = Vec::new();
                    let prev = self.in_container;
                    self.in_container = true;
                    if !matches!(self.cur, Tok::RParen(_)) {
                        args.push(self.expr()?);
                        while matches!(self.cur, Tok::Comma(_)) {
                            self.consume();
                            args.push(self.expr()?);
                        }
                    }
                    self.in_container = prev;
                    self.expect_rparen()?;
                    callee = Expr::Pipe {
                        args,
                        func: Box::new(callee),
                        has_placeholder: false,
                        pos,
                    };
                }
                Tok::DoubleColon(pos) => {
                    self.consume();
                    let variant = self.expect_ident()?;
                    if let Expr::Ident(name, _) = callee {
                        callee = Expr::EnumVariant {
                            enum_name: name,
                            variant,
                            value: None,
                            pos,
                        };
                    } else {
                        return Err(Error::Syntax {
            filename: None,
                            pos,
                            msg: format!("expected identifier before '::', found {}", self.cur.describe()),
                            input: self.input.to_string(),
                        });
                    }
                }
                _ => break,
            }
        }
        Ok(callee)
    }

    fn primary(&mut self) -> Result<Expr, Error> {
        let tok = self.cur.clone();
        match tok {
            Tok::Null(pos) => {
                self.consume();
                Ok(Expr::Null(pos))
            }
            Tok::True(pos) => {
                self.consume();
                Ok(Expr::Bool(true, pos))
            }
            Tok::False(pos) => {
                self.consume();
                Ok(Expr::Bool(false, pos))
            }
            Tok::Num(n, pos) => {
                self.consume();
                Ok(Expr::Num(n, pos))
            }
            Tok::Str(s, pos) => {
                self.consume();
                Ok(Expr::Str(s, pos))
            }
            Tok::TemplateStr(s, pos) => {
                self.consume();
                self.parse_template_string(s, pos)
            }
            Tok::Ok(pos) => {
                self.consume();
                self.expect_lparen()?;
                let value = self.expr()?;
                self.expect_rparen()?;
                Ok(Expr::Result {
                    is_ok: true,
                    value: Box::new(value),
                    pos,
                })
            }
            Tok::Err(pos) => {
                self.consume();
                self.expect_lparen()?;
                let value = self.expr()?;
                self.expect_rparen()?;
                Ok(Expr::Result {
                    is_ok: false,
                    value: Box::new(value),
                    pos,
                })
            }
            Tok::Some(pos) => {
                self.consume();
                self.expect_lparen()?;
                let value = self.expr()?;
                self.expect_rparen()?;
                Ok(Expr::Option {
                    is_some: true,
                    value: Some(Box::new(value)),
                    pos,
                })
            }
            Tok::None(pos) => {
                self.consume();
                Ok(Expr::Option {
                    is_some: false,
                    value: None,
                    pos,
                })
            }
            Tok::Ident(name, pos) => {
                self.consume();
                Ok(Expr::Ident(name, pos))
            }
            Tok::LBracket(_) => self.array_literal(),
            Tok::LBrace(_) => self.object_literal(),
            Tok::LParen(_) => {
                self.consume();
                let prev = self.in_container;
                self.in_container = true;
                let e = self.expr()?;
                self.in_container = prev;
                self.expect_rparen()?;
                Ok(e)
            }
            Tok::Fn(pos) => self.func(pos),
            Tok::PipeSingle(pos) => self.arrow(pos),
            Tok::Question(pos) => {
                self.consume();
                let expr = self.unary()?;
                Ok(Expr::TryExpr {
                    expr: Box::new(expr),
                    pos,
                })
            }
            _ => Err(Error::Syntax {
            filename: None,
                pos: self.pos(),
                msg: format!("unexpected token {}", tok.describe()),
                input: self.input.to_string(),
            }),
        }
    }

    fn parse_template_string(&mut self, raw: String, pos: Pos) -> Result<Expr, Error> {
        let mut parts = Vec::new();
        let mut current_literal = String::new();
        let mut chars = raw.chars().peekable();
        
        while let Some(c) = chars.next() {
            if c == '$' {
                if let Some(&'{') = chars.peek() {
                    chars.next(); // consume '{'
                    // Save current literal if non-empty
                    if !current_literal.is_empty() {
                        parts.push(TemplatePart::Literal(current_literal.clone()));
                        current_literal.clear();
                    }
                    // Parse the expression inside ${}
                    // We need to extract the expression text until we find '}'
                    let mut expr_text = String::new();
                    let mut brace_count = 1;
                    while let Some(ec) = chars.next() {
                        if ec == '{' {
                            brace_count += 1;
                            expr_text.push(ec);
                        } else if ec == '}' {
                            brace_count -= 1;
                            if brace_count == 0 {
                                break;
                            }
                            expr_text.push(ec);
                        } else {
                            expr_text.push(ec);
                        }
                    }
                    // Parse the expression
                    let expr = self.parse_template_expr(&expr_text)?;
                    parts.push(TemplatePart::Expr(expr));
                } else {
                    current_literal.push(c);
                }
            } else {
                current_literal.push(c);
            }
        }
        
        if !current_literal.is_empty() {
            parts.push(TemplatePart::Literal(current_literal));
        }
        
        Ok(Expr::TemplateStr(parts, pos))
    }

    fn parse_template_expr(&mut self, text: &str) -> Result<Expr, Error> {
        // Create a temporary parser for the expression
        let mut temp_parser = P::new(text);
        temp_parser.expr()
    }

    fn array_literal(&mut self) -> Result<Expr, Error> {
        let pos = self.pos();
        self.consume();

        if !matches!(self.cur, Tok::RBracket(_)) {
            let prev = self.in_container;
            self.in_container = true;
            let first_expr = self.expr()?;
            self.in_container = prev;

            if matches!(self.cur, Tok::For(_)) {
                self.consume();
                let var = self.expect_ident()?;

                if !matches!(self.cur, Tok::In(_)) {
                    return Err(Error::Syntax {
            filename: None,
                        pos: self.pos(),
                        msg: format!("expected 'in' after 'for' in list comprehension, found {}", self.cur.describe()),
                        input: self.input.to_string(),
                    });
                }
                self.consume();
                let iterable = self.expr()?;

                self.expect_rbracket()?;

                return Ok(Expr::ListComp {
                    element: Box::new(first_expr),
                    var,
                    iterable: Box::new(iterable),
                    pos,
                });
            }

            let mut elements = vec![first_expr];
            while matches!(self.cur, Tok::Comma(_)) {
                self.consume();
                let prev = self.in_container;
                self.in_container = true;
                let e = self.expr()?;
                self.in_container = prev;
                elements.push(e);
            }
            self.expect_rbracket()?;
            return Ok(Expr::Array(elements, pos));
        }

        self.expect_rbracket()?;
        Ok(Expr::Array(Vec::new(), pos))
    }

    fn object_literal(&mut self) -> Result<Expr, Error> {
        let pos = self.pos();
        self.consume();
        let mut fields = Vec::new();
        if !matches!(self.cur, Tok::RBrace(_)) {
            // Support both {ident: expr} and {"key" = expr}
            let name = if matches!(self.cur, Tok::Str(_, _)) {
                let Tok::Str(s, _) = self.cur.clone() else { unreachable!() };
                self.consume();
                s
            } else {
                self.expect_ident()?
            };
            // Accept either ':' or '=' as separator
            if matches!(self.cur, Tok::Colon(_)) {
                self.consume();
            } else if matches!(self.cur, Tok::Assign(_)) {
                self.consume();
            } else {
                return Err(Error::Syntax {
                    filename: None,
                    pos: self.pos(),
                    msg: format!("expected ':' or '=' in object literal, found {}", self.cur.describe()),
                    input: self.input.to_string(),
                });
            }
            let prev = self.in_container;
            self.in_container = true;
            let value = self.expr()?;
            self.in_container = prev;
            fields.push((name, value));
            while matches!(self.cur, Tok::Comma(_)) {
                self.consume();
                let name = if matches!(self.cur, Tok::Str(_, _)) {
                    let Tok::Str(s, _) = self.cur.clone() else { unreachable!() };
                    self.consume();
                    s
                } else {
                    self.expect_ident()?
                };
                if matches!(self.cur, Tok::Colon(_)) {
                    self.consume();
                } else if matches!(self.cur, Tok::Assign(_)) {
                    self.consume();
                } else {
                    return Err(Error::Syntax {
                        filename: None,
                        pos: self.pos(),
                        msg: format!("expected ':' or '=' in object literal, found {}", self.cur.describe()),
                        input: self.input.to_string(),
                    });
                }
                let prev = self.in_container;
                self.in_container = true;
                let value = self.expr()?;
                self.in_container = prev;
                fields.push((name, value));
            }
        }
        self.expect_rbrace()?;
        Ok(Expr::Object(fields, pos))
    }

    fn func(&mut self, pos: Pos) -> Result<Expr, Error> {
        self.consume();
        self.expect_lparen()?;
        let mut params = Vec::new();
        if !matches!(self.cur, Tok::RParen(_)) {
            let name = self.expect_ident()?;
            let ty = self.parse_opt_type();
            params.push((name, ty));
            while let Tok::Comma(_) = self.cur.clone() {
                self.consume();
                let name = self.expect_ident()?;
                let ty = self.parse_opt_type();
                params.push((name, ty));
            }
        }
        self.expect_rparen()?;
        let ret_type = self.parse_opt_type();
        let mut captures = Vec::new();
        if let Tok::LBracket(_) = self.cur.clone() {
            self.consume();
            if !matches!(self.cur, Tok::RBracket(_)) {
                captures.push(self.parse_capture()?);
                while let Tok::Comma(_) = self.cur.clone() {
                    self.consume();
                    captures.push(self.parse_capture()?);
                }
            }
            self.expect_rbracket()?;
        }
        let body = Box::new(self.block()?);
        Ok(Expr::Func {
            params,
            ret_type,
            captures,
            body,
            pos,
        })
    }

    fn parse_capture(&mut self) -> Result<String, Error> {
        if let Tok::Mut(_) = self.cur.clone() {
            self.consume();
        }
        self.expect_ident()
    }

    fn arrow(&mut self, pos: Pos) -> Result<Expr, Error> {
        self.consume();
        let mut params = Vec::new();
        if !matches!(self.cur, Tok::PipeSingle(_)) {
            let name = self.expect_ident()?;
            let ty = self.parse_opt_type();
            params.push((name, ty));
            while let Tok::Comma(_) = self.cur.clone() {
                self.consume();
                let name = self.expect_ident()?;
                let ty = self.parse_opt_type();
                params.push((name, ty));
            }
        }
        if let Tok::PipeSingle(_) = self.cur.clone() {
            self.consume();
        } else {
            return Err(Error::Syntax {
            filename: None,
                pos: self.pos(),
                msg: format!("expected '|>' after arrow function parameters, found {}", self.cur.describe()),
                input: self.input.to_string(),
            });
        }
        let ret_type = self.parse_opt_type();
        let (body, is_block) = if let Tok::LBrace(_) = self.cur.clone() {
            let body = self.block()?;
            let body_expr = Expr::Func {
                params: Vec::new(),
                ret_type: None,
                captures: Vec::new(),
                body: Box::new(body),
                pos,
            };
            (Box::new(body_expr), true)
        } else {
            let expr = self.expr()?;
            (Box::new(expr), false)
        };
        Ok(Expr::Arrow {
            params,
            ret_type,
            body,
            is_block,
            pos,
        })
    }

    fn consume(&mut self) {
        if let Some(saved) = self.lookahead.take() {
            self.cur = saved;
        } else {
            self.cur = self.lex.next();
        }
    }

    /// Peek at the next token without consuming it.
    fn peek_next(&mut self) -> Tok {
        if self.lookahead.is_none() {
            self.lookahead = Some(self.lex.next());
        }
        self.lookahead.as_ref().unwrap().clone()
    }

    fn expect_ident(&mut self) -> Result<String, Error> {
        if let Tok::Ident(name, _) = self.cur.clone() {
            let name = name;
            self.consume();
            Ok(name)
        } else {
            Err(Error::Syntax {
            filename: None,
                pos: self.pos(),
                msg: format!("expected identifier, found {}", self.cur.describe()),
                input: self.input.to_string(),
            })
        }
    }

    fn expect_assign(&mut self) -> Result<(), Error> {
        if matches!(self.cur, Tok::Assign(_)) {
            self.consume();
            Ok(())
        } else {
            Err(Error::Syntax {
            filename: None,
                pos: self.pos(),
                msg: format!("expected '=', found {}", self.cur.describe()),
                input: self.input.to_string(),
            })
        }
    }

    fn expect_lparen(&mut self) -> Result<(), Error> {
        if matches!(self.cur, Tok::LParen(_)) {
            self.consume();
            Ok(())
        } else {
            Err(Error::Syntax {
            filename: None,
                pos: self.pos(),
                msg: format!("expected '(', found {}", self.cur.describe()),
                input: self.input.to_string(),
            })
        }
    }

    fn expect_rparen(&mut self) -> Result<(), Error> {
        if matches!(self.cur, Tok::RParen(_)) {
            self.consume();
            Ok(())
        } else {
            Err(Error::Syntax {
            filename: None,
                pos: self.pos(),
                msg: format!("expected ')', found {}", self.cur.describe()),
                input: self.input.to_string(),
            })
        }
    }

    fn expect_semicolon(&mut self) -> Result<(), Error> {
        if matches!(self.cur, Tok::Semicolon(_)) {
            self.consume();
            Ok(())
        } else {
            Err(Error::Syntax {
            filename: None,
                pos: self.pos(),
                msg: format!("expected ';', found {}", self.cur.describe()),
                input: self.input.to_string(),
            })
        }
    }

    fn expect_rbrace(&mut self) -> Result<(), Error> {
        if matches!(self.cur, Tok::RBrace(_)) {
            self.consume();
            Ok(())
        } else {
            Err(Error::Syntax {
            filename: None,
                pos: self.pos(),
                msg: format!("expected '}}', found {}", self.cur.describe()),
                input: self.input.to_string(),
            })
        }
    }

    fn expect_colon(&mut self) -> Result<(), Error> {
        if matches!(self.cur, Tok::Colon(_)) {
            self.consume();
            Ok(())
        } else {
            Err(Error::Syntax {
            filename: None,
                pos: self.pos(),
                msg: format!("expected ':', found {}", self.cur.describe()),
                input: self.input.to_string(),
            })
        }
    }

    /// Parse an optional `: Type` annotation.
    fn parse_opt_type(&mut self) -> Option<Type> {
        if matches!(self.cur, Tok::Colon(_)) {
            self.consume();
            self.parse_type().ok()
        } else {
            None
        }
    }

    /// Parse a type expression: Num | Str | Bool | Null | Dyn | [Type] | {Type} | identifier
    fn parse_type(&mut self) -> Result<Type, Error> {
        let pos = self.pos();
        match &self.cur.clone() {
            Tok::Ident(name, _) => {
                let s = name.clone();
                self.consume();
                match s.as_str() {
                    "Num" => Ok(Type::Num),
                    "Str" => Ok(Type::Str),
                    "Bool" => Ok(Type::Bool),
                    "Null" => Ok(Type::Null),
                    "Dyn" | "?" => Ok(Type::Dyn),
                    _ => Ok(Type::Named(s)),
                }
            }
            Tok::LBracket(_) => {
                self.consume();
                let inner = self.parse_type()?;
                self.expect_rbracket()?;
                Ok(Type::Array(Box::new(inner)))
            }
            Tok::LBrace(_) => {
                self.consume();
                let inner = self.parse_type()?;
                self.expect_rbrace()?;
                Ok(Type::Object(Box::new(inner)))
            }
            _ => Err(Error::Syntax {
                filename: None,
                pos,
                msg: format!("expected type, found {}", self.cur.describe()),
                input: self.input.to_string(),
            }),
        }
    }

    fn expect_rbracket(&mut self) -> Result<(), Error> {
        if matches!(self.cur, Tok::RBracket(_)) {
            self.consume();
            Ok(())
        } else {
            Err(Error::Syntax {
            filename: None,
                pos: self.pos(),
                msg: format!("expected ']', found {}", self.cur.describe()),
                input: self.input.to_string(),
            })
        }
    }

    fn expect_loop(&mut self) -> Result<(), Error> {
        if matches!(self.cur, Tok::Loop(_)) {
            self.consume();
            Ok(())
        } else {
            Err(Error::Syntax {
            filename: None,
                pos: self.pos(),
                msg: format!("expected 'loop', found {}", self.cur.describe()),
                input: self.input.to_string(),
            })
        }
    }

    fn expect_fat_arrow(&mut self) -> Result<(), Error> {
        if matches!(self.cur, Tok::FatArrow(_)) {
            self.consume();
            Ok(())
        } else {
            Err(Error::Syntax {
            filename: None,
                pos: self.pos(),
                msg: format!("expected '=>', found {}", self.cur.describe()),
                input: self.input.to_string(),
            })
        }
    }

    fn consume_semicolon(&mut self) -> Result<(), Error> {
        if matches!(self.cur, Tok::Semicolon(_)) {
            self.consume();
        }
        Ok(())
    }

    fn pos(&self) -> Pos {
        match &self.cur {
            Tok::Let(p)
            | Tok::Mut(p)
            | Tok::Fn(p)
            | Tok::If(p)
            | Tok::Else(p)
            | Tok::True(p)
            | Tok::False(p)
            | Tok::Null(p)
            | Tok::Return(p)
            | Tok::And(p)
            | Tok::Or(p)
            | Tok::Loop(p)
            | Tok::Break(p)
            | Tok::Continue(p)
            | Tok::While(p)
            | Tok::For(p)
            | Tok::In(p)
            | Tok::Match(p)
            | Tok::Enum(p)
            | Tok::Struct(p)
            | Tok::Throw(p)
            | Tok::Try(p)
            | Tok::Catch(p)
            | Tok::Finally(p)
            | Tok::Ok(p)
            | Tok::Err(p)
            | Tok::Some(p)
            | Tok::None(p)
            | Tok::Ident(_, p)
            | Tok::Label(_, p)
            | Tok::Num(_, p)
            | Tok::Str(_, p)
            | Tok::TemplateStr(_, p)
            | Tok::Dollar(p)
            | Tok::Spread(p)
            | Tok::Add(p)
            | Tok::Sub(p)
            | Tok::Mul(p)
            | Tok::Div(p)
            | Tok::Eq(p)
            | Tok::Neq(p)
            | Tok::Lt(p)
            | Tok::Gt(p)
            | Tok::Lte(p)
            | Tok::Gte(p)
            | Tok::Assign(p)
            | Tok::Pipe(p)
            | Tok::PipeSingle(p)
            | Tok::LParen(p)
            | Tok::RParen(p)
            | Tok::LBrace(p)
            | Tok::RBrace(p)
            | Tok::Semicolon(p)
            | Tok::Comma(p)
            | Tok::LBracket(p)
            | Tok::RBracket(p)
            | Tok::Ref(p)
            | Tok::Dot(p)
            | Tok::Colon(p)
            | Tok::DoubleColon(p)
            | Tok::FatArrow(p)
            | Tok::Question(p)
            | Tok::Eof(p) => *p,
        }
    }
}

trait ExprPos {
    fn pos(&self) -> Pos;
}

impl ExprPos for Expr {
    fn pos(&self) -> Pos {
        match self {
            Expr::Null(p)
            | Expr::Bool(_, p)
            | Expr::Num(_, p)
            | Expr::Str(_, p)
            | Expr::TemplateStr(_, p)
            | Expr::Ident(_, p)
            | Expr::Array(_, p) => *p,
            Expr::Index { pos, .. }
            | Expr::Object(_, pos)
            | Expr::Field { pos, .. }
            | Expr::EnumVariant { pos, .. }
            | Expr::Match { pos, .. }
            | Expr::Result { pos, .. }
            | Expr::Option { pos, .. }
            | Expr::TryExpr { pos, .. }
            | Expr::BinOp { pos, .. }
            | Expr::UnaryOp { pos, .. }
            | Expr::Func { pos, .. }
            | Expr::Arrow { pos, .. }
            | Expr::Pipe { pos, .. }
            | Expr::ListComp { pos, .. } => *pos,
        }
    }
}
