pub mod ast;
pub mod borrowck;
pub mod compiler;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod type_checker;
pub mod vm;

use crate::borrowck::BorrowChecker;
use crate::compiler::Comp;
use crate::error::Error;
use crate::parser::P;
use crate::type_checker::TC;
use crate::vm::{Value, VM};

pub fn execute(code: &str) -> Result<Value, Error> {
    let mut parser = P::new(code);
    let stmts = parser.parse()?;
    eprintln!("DEBUG: AST = {:#?}", stmts);

    let mut borrow_checker = BorrowChecker::new();
    borrow_checker.check(&stmts)?;

    // Gradual type checking (non-blocking in this phase — collects errors for runtime guards)
    let mut tc = TC::new();
    let type_errors = tc.check_program(&stmts);
    if !type_errors.is_empty() {
        for err in &type_errors {
            eprintln!("TypeError: {}", err.msg);
        }
        // Note: for gradual typing we can choose to block or just warn.
        // Currently just warning — runtime guards will catch mismatches.
    }

    let mut compiler = Comp::new();
    let func = compiler.compile(&stmts).map_err(|e| e.with_input(code.to_string()))?;

    let mut vm = VM::new();
    vm.run(func).map_err(|e| e.with_input(code.to_string()))
}
