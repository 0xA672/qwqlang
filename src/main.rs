use std::fs;
use std::io::{self, BufRead, Write};
use std::process;

mod ast;
mod compiler;
mod error;
mod lexer;
mod parser;
mod vm;

use crate::compiler::Comp;
use crate::error::Error;
use crate::parser::P;
use crate::vm::{CompiledFunction, Value, VM};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 2 {
        run_cli(&args);
        return;
    }
    run_repl();
}

fn run_cli(args: &[String]) {
    // Usage:
    //   qwq <file.qwq>               -- interpret source file
    //   qwq <file.qwqc>              -- run compiled bytecode
    //   qwq -c <file.qwq> [out]      -- compile to bytecode
    //   qwq compile <file.qwq> [out] -- compile to bytecode
    //   qwq run <file.qwq|file.qwqc> -- interpret/run
    match args[1].as_str() {
        "-c" | "--compile" | "compile" => {
            if args.len() < 3 {
                eprintln!("error: missing input file");
                process::exit(1);
            }
            let input = &args[2];
            let output = if args.len() > 3 {
                args[3].clone()
            } else {
                default_compile_output(input)
            };
            if let Err(e) = compile_file(input, &output) {
                eprintln!("{}", e);
                process::exit(1);
            }
            println!("compiled {} -> {}", input, output);
        }
        "run" | "-r" | "--run" => {
            if args.len() < 3 {
                eprintln!("error: missing input file");
                process::exit(1);
            }
            if let Err(e) = run_any_file(&args[2]) {
                eprintln!("{}", e);
                process::exit(1);
            }
        }
        _ => {
            // Treat as a file path
            if let Err(e) = run_any_file(&args[1]) {
                eprintln!("{}", e);
                process::exit(1);
            }
        }
    }
}

fn default_compile_output(input: &str) -> String {
    if let Some(base) = input.strip_suffix(".qwq") {
        format!("{}.qwqc", base)
    } else {
        format!("{}.qwqc", input)
    }
}

fn compile_file(input: &str, output: &str) -> Result<(), Error> {
    let code = fs::read_to_string(input).map_err(|e| Error::Runtime {
        pos: None,
        msg: format!("cannot read '{}': {}", input, e),
    })?;
    let mut parser = P::new(&code);
    let stmts = parser.parse()?;
    let mut compiler = Comp::new();
    let func = compiler.compile(&stmts)?;
    let bytes = func.to_bytes();
    fs::write(output, bytes).map_err(|e| Error::Runtime {
        pos: None,
        msg: format!("cannot write '{}': {}", output, e),
    })?;
    Ok(())
}

fn run_any_file(path: &str) -> Result<(), Error> {
    let bytes = fs::read(path).map_err(|e| Error::Runtime {
        pos: None,
        msg: format!("cannot read '{}': {}", path, e),
    })?;
    if path.ends_with(".qwqc") || bytes.starts_with(b"QWQBC") {
        let func = CompiledFunction::from_bytes(&bytes)?;
        let mut vm = VM::new();
        vm.run(func)?;
    } else {
        let code = String::from_utf8(bytes).map_err(|e| Error::Runtime {
            pos: None,
            msg: format!("invalid utf-8 in '{}': {}", path, e),
        })?;
        run_source(&code)?;
    }
    Ok(())
}

fn run_source(code: &str) -> Result<Value, Error> {
    let mut parser = P::new(code);
    let stmts = parser.parse()?;
    let mut compiler = Comp::new();
    let func = compiler.compile(&stmts)?;
    let mut vm = VM::new();
    vm.run(func)
}

fn run_repl() {
    let mut vm = VM::new();
    let mut compiler = Comp::new();
    let mut input = String::new();
    let stdin = io::stdin();

    loop {
        print!("qwq> ");
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

        let result = run_code(&input, &mut compiler, &mut vm);

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

fn run_code(code: &str, compiler: &mut Comp, vm: &mut VM) -> Result<Value, Error> {
    let mut parser = P::new(code);
    let stmts = parser.parse()?;

    compiler.reset();
    let func = compiler.compile(&stmts)?;

    vm.run(func)
}

pub fn execute(code: &str) -> Result<Value, Error> {
    run_source(code)
}
