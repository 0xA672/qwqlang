pub mod ast;
pub mod compiler;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod vm;

use crate::compiler::Comp;
use crate::error::Error;
use crate::parser::P;
use crate::vm::{Value, VM};

pub fn execute(code: &str) -> Result<Value, Error> {
    let mut parser = P::new(code);
    let stmts = parser.parse()?;
    eprintln!("DEBUG: AST = {:#?}", stmts);

    let mut compiler = Comp::new();
    let func = compiler.compile(&stmts)?;

    let mut vm = VM::new();
    vm.run(func)
}
