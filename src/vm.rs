use crate::compiler::Op;
use crate::error::Error;
use std::fmt;
use std::rc::Rc;
use std::cell::RefCell;

#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Func(CompiledFunction),
    Closure(Rc<RefCell<Closure>>),
    Builtin(fn(&mut VM) -> Result<(), Error>),
}

#[derive(Debug, Clone)]
pub struct CompiledFunction {
    pub bytecode: Vec<u8>,
    pub constants: Vec<Value>,
    pub num_locals: u16,
    pub num_params: u16,
    pub num_free: u16,
}

impl CompiledFunction {
    pub fn new() -> Self {
        CompiledFunction {
            bytecode: Vec::new(),
            constants: Vec::new(),
            num_locals: 0,
            num_params: 0,
            num_free: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Closure {
    pub func: CompiledFunction,
    pub upvalues: Vec<Upvalue>,
}

#[derive(Debug, Clone)]
pub enum Upvalue {
    Open(usize),
    Closed(Value),
    Global(usize),  // Index into VM's globals vector
}

#[derive(Debug)]
pub struct Frame {
    pub closure: Rc<RefCell<Closure>>,
    pub ip: usize,
    pub bp: usize,
}

#[derive(Debug)]
pub struct VM {
    stack: Vec<Value>,
    call_stack: Vec<Frame>,
    globals: Vec<Value>,
    open_upvalues: Vec<Rc<RefCell<Upvalue>>>,
}

impl VM {
    pub fn new() -> Self {
        let mut vm = VM {
            stack: Vec::new(),
            call_stack: Vec::new(),
            globals: Vec::new(),
            open_upvalues: Vec::new(),
        };
        vm.globals.push(Value::Builtin(builtin_print));
        vm
    }

    pub fn run(&mut self, func: CompiledFunction) -> Result<Value, Error> {
        let closure = Rc::new(RefCell::new(Closure {
            func: func.clone(),
            upvalues: Vec::new(),
        }));
        self.call_stack.push(Frame {
            closure: closure.clone(),
            ip: 0,
            bp: 0,
        });
        self.stack.resize(func.num_locals as usize, Value::Null);
        
        loop {
            let frame_idx = self.call_stack.len() - 1;
            let ip = self.call_stack[frame_idx].ip;
            let closure = self.call_stack[frame_idx].closure.clone();
            let bp = self.call_stack[frame_idx].bp;
            let bytecode = closure.borrow().func.bytecode.clone();
            
            if ip >= bytecode.len() {
                let val = self.stack.pop().unwrap_or(Value::Null);
                return Ok(val);
            }
            
            let op = unsafe { std::mem::transmute::<u8, Op>(bytecode[ip]) };
            
            match op {
                Op::Constant => {
                    let idx = self.read_u16(&bytecode, ip + 1);
                    let val = closure.borrow().func.constants[idx as usize].clone();
                    self.stack.push(val);
                    self.call_stack[frame_idx].ip += 3;
                }
                Op::Null => {
                    self.stack.push(Value::Null);
                    self.call_stack[frame_idx].ip += 1;
                }
                Op::True => {
                    self.stack.push(Value::Bool(true));
                    self.call_stack[frame_idx].ip += 1;
                }
                Op::False => {
                    self.stack.push(Value::Bool(false));
                    self.call_stack[frame_idx].ip += 1;
                }
                Op::Add => {
                    let b = self.stack.pop().unwrap_or(Value::Null);
                    let a = self.stack.pop().unwrap_or(Value::Null);
                    eprintln!("DEBUG Add: a={:?}, b={:?}, stack={:?}", a, b, self.stack);
                    let res = match (a, b) {
                        (Value::Num(x), Value::Num(y)) => Value::Num(x + y),
                        (Value::Str(x), Value::Str(y)) => Value::Str(format!("{}{}", x, y)),
                        _ => return Err(Error::Runtime { pos: None, msg: "invalid operands for +".to_string() }),
                    };
                    self.stack.push(res);
                    self.call_stack[frame_idx].ip += 1;
                }
                Op::Sub => {
                    let b = self.stack.pop().unwrap_or(Value::Null);
                    let a = self.stack.pop().unwrap_or(Value::Null);
                    let res = match (a, b) {
                        (Value::Num(x), Value::Num(y)) => Value::Num(x - y),
                        _ => return Err(Error::Runtime { pos: None, msg: "invalid operands for -".to_string() }),
                    };
                    self.stack.push(res);
                    self.call_stack[frame_idx].ip += 1;
                }
                Op::Mul => {
                    let b = self.stack.pop().unwrap_or(Value::Null);
                    let a = self.stack.pop().unwrap_or(Value::Null);
                    let res = match (a, b) {
                        (Value::Num(x), Value::Num(y)) => Value::Num(x * y),
                        _ => return Err(Error::Runtime { pos: None, msg: "invalid operands for *".to_string() }),
                    };
                    self.stack.push(res);
                    self.call_stack[frame_idx].ip += 1;
                }
                Op::Div => {
                    let b = self.stack.pop().unwrap_or(Value::Null);
                    let a = self.stack.pop().unwrap_or(Value::Null);
                    let res = match (a, b) {
                        (Value::Num(x), Value::Num(y)) => {
                            if y == 0.0 {
                                return Err(Error::Runtime { pos: None, msg: "division by zero".to_string() });
                            }
                            Value::Num(x / y)
                        }
                        _ => return Err(Error::Runtime { pos: None, msg: "invalid operands for /".to_string() }),
                    };
                    self.stack.push(res);
                    self.call_stack[frame_idx].ip += 1;
                }
                Op::Eq => {
                    let b = self.stack.pop().unwrap_or(Value::Null);
                    let a = self.stack.pop().unwrap_or(Value::Null);
                    self.stack.push(Value::Bool(self.values_eq(&a, &b)));
                    self.call_stack[frame_idx].ip += 1;
                }
                Op::Neq => {
                    let b = self.stack.pop().unwrap_or(Value::Null);
                    let a = self.stack.pop().unwrap_or(Value::Null);
                    self.stack.push(Value::Bool(!self.values_eq(&a, &b)));
                    self.call_stack[frame_idx].ip += 1;
                }
                Op::Lt => {
                    let b = self.stack.pop().unwrap_or(Value::Null);
                    let a = self.stack.pop().unwrap_or(Value::Null);
                    let res = match (a, b) {
                        (Value::Num(x), Value::Num(y)) => Value::Bool(x < y),
                        (Value::Str(x), Value::Str(y)) => Value::Bool(x < y),
                        _ => return Err(Error::Runtime { pos: None, msg: "invalid operands for <".to_string() }),
                    };
                    self.stack.push(res);
                    self.call_stack[frame_idx].ip += 1;
                }
                Op::Gt => {
                    let b = self.stack.pop().unwrap_or(Value::Null);
                    let a = self.stack.pop().unwrap_or(Value::Null);
                    let res = match (a, b) {
                        (Value::Num(x), Value::Num(y)) => Value::Bool(x > y),
                        (Value::Str(x), Value::Str(y)) => Value::Bool(x > y),
                        _ => return Err(Error::Runtime { pos: None, msg: "invalid operands for >".to_string() }),
                    };
                    self.stack.push(res);
                    self.call_stack[frame_idx].ip += 1;
                }
                Op::Lte => {
                    let b = self.stack.pop().unwrap_or(Value::Null);
                    let a = self.stack.pop().unwrap_or(Value::Null);
                    let res = match (a, b) {
                        (Value::Num(x), Value::Num(y)) => Value::Bool(x <= y),
                        (Value::Str(x), Value::Str(y)) => Value::Bool(x <= y),
                        _ => return Err(Error::Runtime { pos: None, msg: "invalid operands for <=".to_string() }),
                    };
                    self.stack.push(res);
                    self.call_stack[frame_idx].ip += 1;
                }
                Op::Gte => {
                    let b = self.stack.pop().unwrap_or(Value::Null);
                    let a = self.stack.pop().unwrap_or(Value::Null);
                    let res = match (a, b) {
                        (Value::Num(x), Value::Num(y)) => Value::Bool(x >= y),
                        (Value::Str(x), Value::Str(y)) => Value::Bool(x >= y),
                        _ => return Err(Error::Runtime { pos: None, msg: "invalid operands for >=".to_string() }),
                    };
                    self.stack.push(res);
                    self.call_stack[frame_idx].ip += 1;
                }
                Op::Negate => {
                    let a = self.stack.pop().unwrap_or(Value::Null);
                    let res = match a {
                        Value::Num(x) => Value::Num(-x),
                        _ => return Err(Error::Runtime { pos: None, msg: "invalid operand for negation".to_string() }),
                    };
                    self.stack.push(res);
                    self.call_stack[frame_idx].ip += 1;
                }
                Op::Pop => {
                    self.stack.pop();
                    self.call_stack[frame_idx].ip += 1;
                }
                Op::GetGlobal => {
                    let idx = self.read_u16(&bytecode, ip + 1);
                    if idx as usize >= self.globals.len() {
                        return Err(Error::Runtime { pos: None, msg: "undefined global variable".to_string() });
                    }
                    let val = self.globals[idx as usize].clone();
                    eprintln!("DEBUG GetGlobal: idx={}, val={:?}", idx, val);
                    self.stack.push(val);
                    self.call_stack[frame_idx].ip += 3;
                }
                Op::SetGlobal => {
                    let idx = self.read_u16(&bytecode, ip + 1);
                    let val = self.stack.pop().unwrap_or(Value::Null);
                    eprintln!("DEBUG SetGlobal: idx={}, val={:?}", idx, val);
                    if idx as usize >= self.globals.len() {
                        self.globals.resize((idx + 1) as usize, Value::Null);
                    }
                    self.globals[idx as usize] = val;
                    self.call_stack[frame_idx].ip += 3;
                }
                Op::GetLocal => {
                    let idx = self.read_u16(&bytecode, ip + 1);
                    let stack_idx = bp + idx as usize;
                    eprintln!("DEBUG GetLocal: idx={}, bp={}, stack_idx={}, stack_len={}", idx, bp, stack_idx, self.stack.len());
                    if stack_idx >= self.stack.len() {
                        return Err(Error::Runtime { pos: None, msg: "invalid local index".to_string() });
                    }
                    self.stack.push(self.stack[stack_idx].clone());
                    self.call_stack[frame_idx].ip += 3;
                }
                Op::SetLocal => {
                    let idx = self.read_u16(&bytecode, ip + 1);
                    let val = self.stack.pop().unwrap_or(Value::Null);
                    let stack_idx = bp + idx as usize;
                    if stack_idx >= self.stack.len() {
                        return Err(Error::Runtime { pos: None, msg: "invalid local index".to_string() });
                    }
                    self.stack[stack_idx] = val;
                    self.call_stack[frame_idx].ip += 3;
                }
                Op::GetFree => {
                    let idx = self.read_u16(&bytecode, ip + 1);
                    let closure_clone = closure.clone();
                    let upvalues = &closure_clone.borrow().upvalues;
                    if idx as usize >= upvalues.len() {
                        return Err(Error::Runtime { pos: None, msg: "invalid free variable index".to_string() });
                    }
                    let val = match &upvalues[idx as usize] {
                        Upvalue::Open(slot) => self.stack[*slot].clone(),
                        Upvalue::Closed(val) => val.clone(),
                        Upvalue::Global(global_idx) => self.globals[*global_idx].clone(),
                    };
                    self.stack.push(val);
                    self.call_stack[frame_idx].ip += 3;
                }
                Op::SetFree => {
                    let idx = self.read_u16(&bytecode, ip + 1);
                    let val = self.stack.pop().unwrap_or(Value::Null);
                    let closure_clone = closure.clone();
                    let mut closure_mut = closure_clone.borrow_mut();
                    let upvalues = &mut closure_mut.upvalues;
                    if idx as usize >= upvalues.len() {
                        return Err(Error::Runtime { pos: None, msg: "invalid free variable index".to_string() });
                    }
                    match &mut upvalues[idx as usize] {
                        Upvalue::Open(slot) => {
                            self.stack[*slot] = val;
                        }
                        Upvalue::Closed(_) => {
                            upvalues[idx as usize] = Upvalue::Closed(val);
                        }
                        Upvalue::Global(global_idx) => {
                            self.globals[*global_idx] = val;
                        }
                    }
                    self.call_stack[frame_idx].ip += 3;
                }
                Op::Jump => {
                    let offset = self.read_i16(&bytecode, ip + 1);
                    self.call_stack[frame_idx].ip = ((ip as i16) + offset) as usize;
                }
                Op::JumpIfFalse => {
                    let cond = self.stack.last().unwrap_or(&Value::Null).clone();
                    let offset = self.read_i16(&bytecode, ip + 1);
                    if !self.is_truthy(&cond) {
                        // Keep the value on stack for short-circuit result
                        self.call_stack[frame_idx].ip = ((ip as i16) + offset) as usize;
                    } else {
                        // Pop the value - condition is true, continue
                        self.stack.pop();
                        self.call_stack[frame_idx].ip += 3;
                    }
                }
                Op::JumpIfTrue => {
                    let cond = self.stack.last().unwrap_or(&Value::Null).clone();
                    let offset = self.read_i16(&bytecode, ip + 1);
                    if self.is_truthy(&cond) {
                        // Keep the value on stack for short-circuit result
                        self.call_stack[frame_idx].ip = ((ip as i16) + offset) as usize;
                    } else {
                        // Pop the value - condition is false, continue
                        self.stack.pop();
                        self.call_stack[frame_idx].ip += 3;
                    }
                }
                Op::JumpIfFalsePop => {
                    let cond = self.stack.pop().unwrap_or(Value::Null);
                    let offset = self.read_i16(&bytecode, ip + 1);
                    if !self.is_truthy(&cond) {
                        self.call_stack[frame_idx].ip = ((ip as i16) + offset) as usize;
                    } else {
                        self.call_stack[frame_idx].ip += 3;
                    }
                }
                Op::Closure => {
                    let func_idx = self.read_u16(&bytecode, ip + 1);
                    let num_free = self.read_u16(&bytecode, ip + 3);
                    let constants = &closure.borrow().func.constants;
                    let func = match &constants[func_idx as usize] {
                        Value::Func(f) => f.clone(),
                        _ => return Err(Error::Runtime { pos: None, msg: "expected function in constant pool".to_string() }),
                    };
                    let mut upvalues = Vec::new();
                    // Pop captured values from the stack
                    for _ in 0..num_free {
                        let val = self.stack.pop().unwrap_or(Value::Null);
                        match val {
                            // If it's a number, check if it's a global capture marker
                            // We use a special encoding: negative numbers represent global indices
                            Value::Num(n) if n < 0.0 => {
                                let global_idx = (-n - 1.0) as usize;
                                upvalues.insert(0, Upvalue::Global(global_idx));
                            }
                            _ => {
                                upvalues.insert(0, Upvalue::Closed(val));
                            }
                        }
                    }
                    let new_closure = Rc::new(RefCell::new(Closure { func, upvalues }));
                    self.stack.push(Value::Closure(new_closure));
                    self.call_stack[frame_idx].ip += 5;
                }
                Op::Call => {
                    let num_args = self.read_u16(&bytecode, ip + 1);
                    eprintln!("DEBUG: Call - stack: {:?}", self.stack);
                    let callee_idx = self.stack.len() - num_args as usize - 1;
                    if callee_idx >= self.stack.len() {
                        return Err(Error::Runtime { pos: None, msg: "stack underflow".to_string() });
                    }
                    let callee = self.stack.remove(callee_idx);
                    eprintln!("DEBUG: Call - callee: {:?}", callee);
                    match callee {
                        Value::Closure(closure) => {
                            let func = &closure.borrow().func;
                            if func.num_params != num_args {
                                return Err(Error::Runtime { 
                                    pos: None, 
                                    msg: format!("expected {} arguments, got {}", func.num_params, num_args) 
                                });
                            }
                            let new_bp = self.stack.len() - num_args as usize;
                            self.stack.resize(new_bp + (func.num_locals as usize), Value::Null);
                            self.call_stack.push(Frame {
                                closure: closure.clone(),
                                ip: 0,
                                bp: new_bp,
                            });
                        }
                        Value::Func(func) => {
                            if func.num_params != num_args {
                                return Err(Error::Runtime { 
                                    pos: None, 
                                    msg: format!("expected {} arguments, got {}", func.num_params, num_args) 
                                });
                            }
                            let closure = Rc::new(RefCell::new(Closure {
                                func: func.clone(),
                                upvalues: Vec::new(),
                            }));
                            let new_bp = self.stack.len() - num_args as usize;
                            self.stack.resize(new_bp + (func.num_locals as usize), Value::Null);
                            self.call_stack.push(Frame {
                                closure: closure.clone(),
                                ip: 0,
                                bp: new_bp,
                            });
                        }
                        Value::Builtin(f) => {
                            f(self)?;
                        }
                        _ => return Err(Error::Runtime { pos: None, msg: "cannot call non-function".to_string() }),
                    }
                    self.call_stack[frame_idx].ip += 3;
                }
                Op::Return => {
                    let val = self.stack.pop().unwrap_or(Value::Null);
                    let frame = self.call_stack.pop().unwrap();
                    self.stack.truncate(frame.bp);
                    if self.call_stack.is_empty() {
                        return Ok(val);
                    }
                    self.stack.push(val);
                    // Don't increment ip here - Call already did it
                }
                Op::GetBuiltin => {
                    let idx = self.read_u16(&bytecode, ip + 1);
                    if idx as usize >= self.globals.len() {
                        return Err(Error::Runtime { pos: None, msg: "invalid builtin index".to_string() });
                    }
                    self.stack.push(self.globals[idx as usize].clone());
                    self.call_stack[frame_idx].ip += 3;
                }
                Op::CaptureGlobal => {
                    let idx = self.read_u16(&bytecode, ip + 1);
                    // Push a marker value that indicates this is a global capture
                    // We use negative numbers to encode the global index
                    self.stack.push(Value::Num(-(idx as f64 + 1.0)));
                    self.call_stack[frame_idx].ip += 3;
                }
            }
        }
    }

    fn read_u16(&self, bytecode: &[u8], pos: usize) -> u16 {
        ((bytecode[pos + 1] as u16) << 8) | (bytecode[pos] as u16)
    }

    fn read_i16(&self, bytecode: &[u8], pos: usize) -> i16 {
        self.read_u16(bytecode, pos) as i16
    }

    fn is_truthy(&self, val: &Value) -> bool {
        match val {
            Value::Null => false,
            Value::Bool(b) => *b,
            _ => true,
        }
    }

    fn values_eq(&self, a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(x), Value::Bool(y)) => x == y,
            (Value::Num(x), Value::Num(y)) => x == y,
            (Value::Str(x), Value::Str(y)) => x == y,
            _ => false,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Num(n) => {
                if n.trunc() == *n {
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{}", n)
                }
            }
            Value::Str(s) => write!(f, "\"{}\"", s),
            Value::Func(_) => write!(f, "<function>"),
            Value::Closure(_) => write!(f, "<closure>"),
            Value::Builtin(_) => write!(f, "<builtin>"),
        }
    }
}

fn builtin_print(vm: &mut VM) -> Result<(), Error> {
    let num_args = vm.stack.len();
    for i in 0..num_args {
        if i > 0 {
            print!(" ");
        }
        print!("{}", vm.stack[i]);
    }
    println!();
    vm.stack.clear();
    vm.stack.push(Value::Null);
    Ok(())
}
