use std::io::{self, BufRead, Write};

mod ast;
mod lexer;
mod parser;
mod compiler;
mod vm;
mod error;

use crate::compiler::Comp;
use crate::parser::P;
use crate::vm::{VM, Value};
use crate::error::Error;

fn main() {
    let mut vm = VM::new();
    let mut input = String::new();
    let stdin = io::stdin();
    
    loop {
        print!("qWQ> ");
        io::stdout().flush().unwrap();
        
        let line = stdin.lock().lines().next().unwrap().unwrap();
        input.push_str(&line);
        input.push('\n');
        
        let exit_commands = ["exit", "quit", "q", ".exit"];
        if exit_commands.contains(&line.trim()) {
            break;
        }
        
        if !is_balanced(&input) {
            continue;
        }
        
        let result = run_code(&input, &mut vm);
        
        match result {
            Ok(val) => {
                if !matches!(val, Value::Null) {
                    println!("→ {}", val);
                }
            }
            Err(e) => {
                eprintln!("{}", e);
            }
        }
        
        input.clear();
    }
}

fn is_balanced(s: &str) -> bool {
    let mut parens = 0;
    let mut braces = 0;
    let mut in_string = false;
    let mut escape = false;
    
    for c in s.chars() {
        if escape {
            escape = false;
            continue;
        }
        
        if c == '\\' && in_string {
            escape = true;
            continue;
        }
        
        if c == '"' {
            in_string = !in_string;
            continue;
        }
        
        if !in_string {
            match c {
                '(' => parens += 1,
                ')' => parens -= 1,
                '{' => braces += 1,
                '}' => braces -= 1,
                _ => {}
            }
        }
        
        if parens < 0 || braces < 0 {
            return false;
        }
    }
    
    parens == 0 && braces == 0
}

fn run_code(code: &str, vm: &mut VM) -> Result<Value, Error> {
    let mut parser = P::new(code);
    let stmts = parser.parse()?;
    
    let mut compiler = Comp::new();
    let func = compiler.compile(&stmts)?;
    
    vm.run(func)
}

pub fn execute(code: &str) -> Result<Value, Error> {
    let mut vm = VM::new();
    run_code(code, &mut vm)
}
