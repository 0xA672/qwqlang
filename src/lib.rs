pub mod ast;
pub mod borrowck;
pub mod compiler;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod vm;

use crate::borrowck::BorrowChecker;
use crate::compiler::Comp;
use crate::error::Error;
use crate::parser::P;
use crate::vm::{Value, VM};

pub fn execute(code: &str) -> Result<Value, Error> {
    let mut parser = P::new(code);
    let stmts = parser.parse()?;
    eprintln!("DEBUG: AST = {:#?}", stmts);

    let mut borrow_checker = BorrowChecker::new();
    borrow_checker.check(&stmts)?;

    let mut compiler = Comp::new();
    let func = compiler.compile(&stmts).map_err(|e| e.with_input(code.to_string()))?;

    let mut vm = VM::new();
    vm.run(func).map_err(|e| e.with_input(code.to_string()))
}
