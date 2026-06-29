use crate::compiler::Op;
use crate::error::Error;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Array(Rc<RefCell<Vec<Value>>>),
    Object(Rc<RefCell<HashMap<String, Value>>>),
    Enum {
        enum_name: String,
        variant: String,
        value: Option<Box<Value>>,
    },
    Result {
        is_ok: bool,
        value: Box<Value>,
    },
    Option {
        is_some: bool,
        value: Option<Box<Value>>,
    },
    Func(CompiledFunction),
    Closure(Rc<RefCell<Closure>>),
    Builtin(fn(&mut VM, u16) -> Result<(), Error>),
    Ref(Rc<RefCell<Value>>),
    MutRef(Rc<RefCell<Value>>),
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

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"QWQBC");
        buf.push(1);
        self.serialize_into(&mut buf);
        buf
    }

    fn serialize_into(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.num_locals.to_le_bytes());
        buf.extend_from_slice(&self.num_params.to_le_bytes());
        buf.extend_from_slice(&self.num_free.to_le_bytes());
        buf.extend_from_slice(&(self.bytecode.len() as u32).to_le_bytes());
        buf.extend_from_slice(&self.bytecode);
        buf.extend_from_slice(&(self.constants.len() as u32).to_le_bytes());
        for c in &self.constants {
            c.serialize_into(buf);
        }
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, Error> {
        if data.len() < 6 || &data[..5] != b"QWQBC" {
            return Err(Error::Runtime {
            input: None,
            filename: None,
                pos: None,
                msg: "invalid bytecode file (bad magic)".to_string(),
            });
        }
        let version = data[5];
        if version != 1 {
            return Err(Error::Runtime {
            input: None,
            filename: None,
                pos: None,
                msg: format!("unsupported bytecode version: {}", version),
            });
        }
        let mut pos = 6;
        Self::deserialize_from(data, &mut pos)
    }

    fn deserialize_from(data: &[u8], pos: &mut usize) -> Result<Self, Error> {
        let num_locals = read_u16_at(data, pos)?;
        let num_params = read_u16_at(data, pos)?;
        let num_free = read_u16_at(data, pos)?;
        let bc_len = read_u32_at(data, pos)? as usize;
        if *pos + bc_len > data.len() {
            return Err(Error::Runtime {
            input: None,
            filename: None,
                pos: None,
                msg: "bytecode truncated".to_string(),
            });
        }
        let bytecode = data[*pos..*pos + bc_len].to_vec();
        *pos += bc_len;
        let num_consts = read_u32_at(data, pos)? as usize;
        let mut constants = Vec::with_capacity(num_consts);
        for _ in 0..num_consts {
            constants.push(Value::deserialize_from(data, pos)?);
        }
        Ok(CompiledFunction {
            bytecode,
            constants,
            num_locals,
            num_params,
            num_free,
        })
    }
}

fn read_u16_at(data: &[u8], pos: &mut usize) -> Result<u16, Error> {
    if *pos + 2 > data.len() {
        return Err(Error::Runtime {
            input: None,
            filename: None,
            pos: None,
            msg: "bytecode truncated".to_string(),
        });
    }
    let v = u16::from_le_bytes([data[*pos], data[*pos + 1]]);
    *pos += 2;
    Ok(v)
}

fn read_u32_at(data: &[u8], pos: &mut usize) -> Result<u32, Error> {
    if *pos + 4 > data.len() {
        return Err(Error::Runtime {
            input: None,
            filename: None,
            pos: None,
            msg: "bytecode truncated".to_string(),
        });
    }
    let v = u32::from_le_bytes([data[*pos], data[*pos + 1], data[*pos + 2], data[*pos + 3]]);
    *pos += 4;
    Ok(v)
}

#[inline]
fn read_u16(bytecode: &[u8], pos: usize) -> u16 {
    u16::from_le_bytes([bytecode[pos], bytecode[pos + 1]])
}

#[inline]
fn read_i16(bytecode: &[u8], pos: usize) -> i16 {
    read_u16(bytecode, pos) as i16
}

impl Value {
    fn serialize_into(&self, buf: &mut Vec<u8>) {
        match self {
            Value::Null => {
                buf.push(0);
            }
            Value::Bool(b) => {
                buf.push(1);
                buf.push(if *b { 1 } else { 0 });
            }
            Value::Num(n) => {
                buf.push(2);
                buf.extend_from_slice(&n.to_le_bytes());
            }
            Value::Str(s) => {
                buf.push(3);
                let bytes = s.as_bytes();
                buf.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                buf.extend_from_slice(bytes);
            }
            Value::Func(f) => {
                buf.push(4);
                f.serialize_into(buf);
            }
            Value::Closure(_) => {
                buf.push(0);
            }
            Value::Builtin(_) => {
                buf.push(0);
            }
            Value::Array(arr) => {
                buf.push(5);
                let elements = arr.borrow();
                buf.extend_from_slice(&(elements.len() as u32).to_le_bytes());
                for elem in elements.iter() {
                    elem.serialize_into(buf);
                }
            }
            Value::Object(obj) => {
                buf.push(6);
                let map = obj.borrow();
                buf.extend_from_slice(&(map.len() as u32).to_le_bytes());
                for (k, v) in map.iter() {
                    let bytes = k.as_bytes();
                    buf.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                    buf.extend_from_slice(bytes);
                    v.serialize_into(buf);
                }
            }
            Value::Enum { enum_name, variant, value } => {
                buf.push(7);
                let bytes = enum_name.as_bytes();
                buf.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                buf.extend_from_slice(bytes);
                let bytes = variant.as_bytes();
                buf.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                buf.extend_from_slice(bytes);
                if let Some(val) = value {
                    buf.push(1);
                    val.serialize_into(buf);
                } else {
                    buf.push(0);
                }
            }
            Value::Result { is_ok, value } => {
                buf.push(8);
                buf.push(if *is_ok { 1 } else { 0 });
                value.serialize_into(buf);
            }
            Value::Option { is_some, value } => {
                buf.push(9);
                buf.push(if *is_some { 1 } else { 0 });
                if let Some(val) = value {
                    val.serialize_into(buf);
                }
            }
            Value::Ref(_) | Value::MutRef(_) => {
                buf.push(0);
            }
        }
    }

    fn deserialize_from(data: &[u8], pos: &mut usize) -> Result<Self, Error> {
        if *pos >= data.len() {
            return Err(Error::Runtime {
            input: None,
            filename: None,
                pos: None,
                msg: "bytecode truncated".to_string(),
            });
        }
        let tag = data[*pos];
        *pos += 1;
        match tag {
            0 => Ok(Value::Null),
            1 => {
                if *pos >= data.len() {
                    return Err(Error::Runtime {
            input: None,
            filename: None,
                        pos: None,
                        msg: "bytecode truncated".to_string(),
                    });
                }
                let b = data[*pos] != 0;
                *pos += 1;
                Ok(Value::Bool(b))
            }
            2 => {
                if *pos + 8 > data.len() {
                    return Err(Error::Runtime {
            input: None,
            filename: None,
                        pos: None,
                        msg: "bytecode truncated".to_string(),
                    });
                }
                let mut arr = [0u8; 8];
                arr.copy_from_slice(&data[*pos..*pos + 8]);
                *pos += 8;
                Ok(Value::Num(f64::from_le_bytes(arr)))
            }
            3 => {
                let len = read_u32_at(data, pos)? as usize;
                if *pos + len > data.len() {
                    return Err(Error::Runtime {
            input: None,
            filename: None,
                        pos: None,
                        msg: "bytecode truncated".to_string(),
                    });
                }
                let s = String::from_utf8(data[*pos..*pos + len].to_vec()).map_err(|_| {
                    Error::Runtime {
            input: None,
            filename: None,
                        pos: None,
                        msg: "invalid utf-8 in string constant".to_string(),
                    }
                })?;
                *pos += len;
                Ok(Value::Str(s))
            }
            4 => {
                let f = CompiledFunction::deserialize_from(data, pos)?;
                Ok(Value::Func(f))
            }
            5 => {
                let num_elements = read_u32_at(data, pos)? as usize;
                let mut elements = Vec::with_capacity(num_elements);
                for _ in 0..num_elements {
                    elements.push(Value::deserialize_from(data, pos)?);
                }
                Ok(Value::Array(Rc::new(RefCell::new(elements))))
            }
            6 => {
                let num_fields = read_u32_at(data, pos)? as usize;
                let mut map = HashMap::new();
                for _ in 0..num_fields {
                    let name_len = read_u32_at(data, pos)? as usize;
                    if *pos + name_len > data.len() {
                        return Err(Error::Runtime {
            input: None,
            filename: None,
                            pos: None,
                            msg: "bytecode truncated".to_string(),
                        });
                    }
                    let name = String::from_utf8(data[*pos..*pos + name_len].to_vec()).map_err(|_| {
                        Error::Runtime {
            input: None,
            filename: None,
                            pos: None,
                            msg: "invalid utf-8 in string".to_string(),
                        }
                    })?;
                    *pos += name_len;
                    let value = Value::deserialize_from(data, pos)?;
                    map.insert(name, value);
                }
                Ok(Value::Object(Rc::new(RefCell::new(map))))
            }
            7 => {
                let enum_len = read_u32_at(data, pos)? as usize;
                if *pos + enum_len > data.len() {
                    return Err(Error::Runtime {
            input: None,
            filename: None,
                        pos: None,
                        msg: "bytecode truncated".to_string(),
                    });
                }
                let enum_name = String::from_utf8(data[*pos..*pos + enum_len].to_vec()).map_err(|_| {
                    Error::Runtime {
            input: None,
            filename: None,
                        pos: None,
                        msg: "invalid utf-8 in string".to_string(),
                    }
                })?;
                *pos += enum_len;

                let variant_len = read_u32_at(data, pos)? as usize;
                if *pos + variant_len > data.len() {
                    return Err(Error::Runtime {
            input: None,
            filename: None,
                        pos: None,
                        msg: "bytecode truncated".to_string(),
                    });
                }
                let variant = String::from_utf8(data[*pos..*pos + variant_len].to_vec()).map_err(|_| {
                    Error::Runtime {
            input: None,
            filename: None,
                        pos: None,
                        msg: "invalid utf-8 in string".to_string(),
                    }
                })?;
                *pos += variant_len;

                let has_value = data[*pos];
                *pos += 1;
                let value = if has_value == 1 {
                    Some(Box::new(Value::deserialize_from(data, pos)?))
                } else {
                    None
                };

                Ok(Value::Enum {
                    enum_name,
                    variant,
                    value,
                })
            }
            8 => {
                if *pos >= data.len() {
                    return Err(Error::Runtime {
            input: None,
            filename: None,
                        pos: None,
                        msg: "bytecode truncated".to_string(),
                    });
                }
                let is_ok = data[*pos] != 0;
                *pos += 1;
                let value = Box::new(Value::deserialize_from(data, pos)?);
                Ok(Value::Result { is_ok, value })
            }
            9 => {
                if *pos >= data.len() {
                    return Err(Error::Runtime {
            input: None,
            filename: None,
                        pos: None,
                        msg: "bytecode truncated".to_string(),
                    });
                }
                let is_some = data[*pos] != 0;
                *pos += 1;
                let value = if is_some {
                    Some(Box::new(Value::deserialize_from(data, pos)?))
                } else {
                    None
                };
                Ok(Value::Option { is_some, value })
            }
            other => Err(Error::Runtime {
            input: None,
            filename: None,
                pos: None,
                msg: format!("unknown value tag in bytecode: {}", other),
            }),
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
    Global(usize),
}

#[derive(Debug)]
pub struct Frame {
    pub closure: Rc<RefCell<Closure>>,
    pub ip: usize,
    pub bp: usize,
}

#[derive(Debug)]
pub struct TryFrame {
    pub catch_ip: Option<usize>,
    pub finally_ip: Option<usize>,
    pub end_ip: usize,
    pub stack_height: usize,
}

#[derive(Debug)]
pub struct VM {
    stack: Vec<Value>,
    call_stack: Vec<Frame>,
    globals: Vec<Value>,
    open_upvalues: Vec<Rc<RefCell<Upvalue>>>,
    try_stack: Vec<TryFrame>,
    exception: Option<Value>,
    pending_return: Option<Value>,
}

impl VM {
    pub fn new() -> Self {
        let mut vm = VM {
            stack: Vec::new(),
            call_stack: Vec::new(),
            globals: Vec::new(),
            open_upvalues: Vec::new(),
            try_stack: Vec::new(),
            exception: None,
            pending_return: None,
        };
        vm.globals.push(Value::Builtin(builtin_print));
        vm.globals.push(Value::Builtin(builtin_len));
        vm.globals.push(Value::Builtin(builtin_push));
        vm.globals.push(Value::Builtin(builtin_pop));
        vm.globals.push(Value::Builtin(builtin_is_ok));
        vm.globals.push(Value::Builtin(builtin_is_err));
        vm.globals.push(Value::Builtin(builtin_is_some));
        vm.globals.push(Value::Builtin(builtin_is_none));
        vm.globals.push(Value::Builtin(builtin_unwrap));
        vm.globals.push(Value::Builtin(builtin_unwrap_or));
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
            let (ip, bp, bytecode_ptr, bytecode_len, closure_rc) = {
                let frame = self.call_stack.last().unwrap();
                let closure = frame.closure.borrow();
                let bytecode = &closure.func.bytecode;
                (
                    frame.ip,
                    frame.bp,
                    bytecode.as_ptr(),
                    bytecode.len(),
                    frame.closure.clone(),
                )
            };

            if ip >= bytecode_len {
                let val = self.stack.pop().unwrap_or(Value::Null);
                return Ok(val);
            }

            let bytecode: &[u8] = unsafe { std::slice::from_raw_parts(bytecode_ptr, bytecode_len) };
            let op = unsafe { std::mem::transmute::<u8, Op>(*bytecode_ptr.add(ip)) };

            #[cfg(debug_assertions)]
            eprintln!("DEBUG ip={}, op={:?}", ip, op);

            match op {
                Op::Constant => {
                    let idx = read_u16(bytecode, ip + 1);
                    let val = closure_rc.borrow().func.constants[idx as usize].clone();
                    self.stack.push(val);
                    self.call_stack.last_mut().unwrap().ip += 3;
                }
                Op::Null => {
                    self.stack.push(Value::Null);
                    self.call_stack.last_mut().unwrap().ip += 1;
                }
                Op::True => {
                    self.stack.push(Value::Bool(true));
                    self.call_stack.last_mut().unwrap().ip += 1;
                }
                Op::False => {
                    self.stack.push(Value::Bool(false));
                    self.call_stack.last_mut().unwrap().ip += 1;
                }
                Op::Add => {
                    let b = self.stack.pop().unwrap_or(Value::Null);
                    let a = self.stack.pop().unwrap_or(Value::Null);
                    #[cfg(debug_assertions)]
                    eprintln!("DEBUG Add: a={:?}, b={:?}, stack={:?}", a, b, self.stack);
                    let res = match (a, b) {
                        (Value::Num(x), Value::Num(y)) => Value::Num(x + y),
                        (Value::Str(x), Value::Str(y)) => Value::Str(format!("{}{}", x, y)),
                        _ => {
                            return Err(Error::Runtime {
            input: None,
            filename: None,
                                pos: None,
                                msg: "invalid operands for +".to_string(),
                            })
                        }
                    };
                    self.stack.push(res);
                    self.call_stack.last_mut().unwrap().ip += 1;
                }
                Op::Sub => {
                    let b = self.stack.pop().unwrap_or(Value::Null);
                    let a = self.stack.pop().unwrap_or(Value::Null);
                    let res = match (a, b) {
                        (Value::Num(x), Value::Num(y)) => Value::Num(x - y),
                        _ => {
                            return Err(Error::Runtime {
            input: None,
            filename: None,
                                pos: None,
                                msg: "invalid operands for -".to_string(),
                            })
                        }
                    };
                    self.stack.push(res);
                    self.call_stack.last_mut().unwrap().ip += 1;
                }
                Op::Mul => {
                    let b = self.stack.pop().unwrap_or(Value::Null);
                    let a = self.stack.pop().unwrap_or(Value::Null);
                    let res = match (a, b) {
                        (Value::Num(x), Value::Num(y)) => Value::Num(x * y),
                        _ => {
                            return Err(Error::Runtime {
            input: None,
            filename: None,
                                pos: None,
                                msg: "invalid operands for *".to_string(),
                            })
                        }
                    };
                    self.stack.push(res);
                    self.call_stack.last_mut().unwrap().ip += 1;
                }
                Op::Div => {
                    let b = self.stack.pop().unwrap_or(Value::Null);
                    let a = self.stack.pop().unwrap_or(Value::Null);
                    let res = match (a, b) {
                        (Value::Num(x), Value::Num(y)) => {
                            if y == 0.0 {
                                return Err(Error::Runtime {
            input: None,
            filename: None,
                                    pos: None,
                                    msg: "division by zero".to_string(),
                                });
                            }
                            Value::Num(x / y)
                        }
                        _ => {
                            return Err(Error::Runtime {
            input: None,
            filename: None,
                                pos: None,
                                msg: "invalid operands for /".to_string(),
                            })
                        }
                    };
                    self.stack.push(res);
                    self.call_stack.last_mut().unwrap().ip += 1;
                }
                Op::Eq => {
                    let b = self.stack.pop().unwrap_or(Value::Null);
                    let a = self.stack.pop().unwrap_or(Value::Null);
                    self.stack.push(Value::Bool(values_eq(&a, &b)));
                    self.call_stack.last_mut().unwrap().ip += 1;
                }
                Op::Neq => {
                    let b = self.stack.pop().unwrap_or(Value::Null);
                    let a = self.stack.pop().unwrap_or(Value::Null);
                    self.stack.push(Value::Bool(!values_eq(&a, &b)));
                    self.call_stack.last_mut().unwrap().ip += 1;
                }
                Op::Lt => {
                    let b = self.stack.pop().unwrap_or(Value::Null);
                    let a = self.stack.pop().unwrap_or(Value::Null);
                    let res = match (a, b) {
                        (Value::Num(x), Value::Num(y)) => Value::Bool(x < y),
                        (Value::Str(x), Value::Str(y)) => Value::Bool(x < y),
                        _ => {
                            return Err(Error::Runtime {
            input: None,
            filename: None,
                                pos: None,
                                msg: "invalid operands for <".to_string(),
                            })
                        }
                    };
                    self.stack.push(res);
                    self.call_stack.last_mut().unwrap().ip += 1;
                }
                Op::Gt => {
                    let b = self.stack.pop().unwrap_or(Value::Null);
                    let a = self.stack.pop().unwrap_or(Value::Null);
                    let res = match (a, b) {
                        (Value::Num(x), Value::Num(y)) => Value::Bool(x > y),
                        (Value::Str(x), Value::Str(y)) => Value::Bool(x > y),
                        _ => {
                            return Err(Error::Runtime {
            input: None,
            filename: None,
                                pos: None,
                                msg: "invalid operands for >".to_string(),
                            })
                        }
                    };
                    self.stack.push(res);
                    self.call_stack.last_mut().unwrap().ip += 1;
                }
                Op::Lte => {
                    let b = self.stack.pop().unwrap_or(Value::Null);
                    let a = self.stack.pop().unwrap_or(Value::Null);
                    let res = match (a, b) {
                        (Value::Num(x), Value::Num(y)) => Value::Bool(x <= y),
                        (Value::Str(x), Value::Str(y)) => Value::Bool(x <= y),
                        _ => {
                            return Err(Error::Runtime {
            input: None,
            filename: None,
                                pos: None,
                                msg: "invalid operands for <=".to_string(),
                            })
                        }
                    };
                    self.stack.push(res);
                    self.call_stack.last_mut().unwrap().ip += 1;
                }
                Op::Gte => {
                    let b = self.stack.pop().unwrap_or(Value::Null);
                    let a = self.stack.pop().unwrap_or(Value::Null);
                    let res = match (a, b) {
                        (Value::Num(x), Value::Num(y)) => Value::Bool(x >= y),
                        (Value::Str(x), Value::Str(y)) => Value::Bool(x >= y),
                        _ => {
                            return Err(Error::Runtime {
            input: None,
            filename: None,
                                pos: None,
                                msg: "invalid operands for >=".to_string(),
                            })
                        }
                    };
                    self.stack.push(res);
                    self.call_stack.last_mut().unwrap().ip += 1;
                }
                Op::Negate => {
                    let a = self.stack.pop().unwrap_or(Value::Null);
                    let res = match a {
                        Value::Num(x) => Value::Num(-x),
                        _ => {
                            return Err(Error::Runtime {
            input: None,
            filename: None,
                                pos: None,
                                msg: "invalid operand for negation".to_string(),
                            })
                        }
                    };
                    self.stack.push(res);
                    self.call_stack.last_mut().unwrap().ip += 1;
                }
                Op::Pop => {
                    self.stack.pop();
                    self.call_stack.last_mut().unwrap().ip += 1;
                }
                Op::GetGlobal => {
                    let idx = read_u16(bytecode, ip + 1);
                    if idx as usize >= self.globals.len() {
                        return Err(Error::Runtime {
            input: None,
            filename: None,
                            pos: None,
                            msg: "undefined global variable".to_string(),
                        });
                    }
                    let val = self.globals[idx as usize].clone();
                    #[cfg(debug_assertions)]
                    eprintln!("DEBUG GetGlobal: idx={}, val={:?}", idx, val);
                    self.stack.push(val);
                    self.call_stack.last_mut().unwrap().ip += 3;
                }
                Op::SetGlobal => {
                    let idx = read_u16(bytecode, ip + 1);
                    let val = self.stack.pop().unwrap_or(Value::Null);
                    #[cfg(debug_assertions)]
                    eprintln!("DEBUG SetGlobal: idx={}, val={:?}", idx, val);
                    if idx as usize >= self.globals.len() {
                        self.globals.resize((idx + 1) as usize, Value::Null);
                    }
                    self.globals[idx as usize] = val;
                    self.call_stack.last_mut().unwrap().ip += 3;
                }
                Op::GetLocal => {
                    let idx = read_u16(bytecode, ip + 1);
                    let stack_idx = bp + idx as usize;
                    #[cfg(debug_assertions)]
                    eprintln!(
                        "DEBUG GetLocal: idx={}, bp={}, stack_idx={}, stack_len={}",
                        idx,
                        bp,
                        stack_idx,
                        self.stack.len()
                    );
                    if stack_idx >= self.stack.len() {
                        return Err(Error::Runtime {
            input: None,
            filename: None,
                            pos: None,
                            msg: "invalid local index".to_string(),
                        });
                    }
                    self.stack.push(self.stack[stack_idx].clone());
                    self.call_stack.last_mut().unwrap().ip += 3;
                }
                Op::SetLocal => {
                    let idx = read_u16(bytecode, ip + 1);
                    let val = self.stack.pop().unwrap_or(Value::Null);
                    let stack_idx = bp + idx as usize;
                    if stack_idx >= self.stack.len() {
                        return Err(Error::Runtime {
            input: None,
            filename: None,
                            pos: None,
                            msg: "invalid local index".to_string(),
                        });
                    }
                    self.stack[stack_idx] = val;
                    self.call_stack.last_mut().unwrap().ip += 3;
                }
                Op::GetFree => {
                    let idx = read_u16(bytecode, ip + 1);
                    let closure_clone = closure_rc.clone();
                    let upvalues = &closure_clone.borrow().upvalues;
                    if idx as usize >= upvalues.len() {
                        return Err(Error::Runtime {
            input: None,
            filename: None,
                            pos: None,
                            msg: "invalid free variable index".to_string(),
                        });
                    }
                    let val = match &upvalues[idx as usize] {
                        Upvalue::Open(slot) => self.stack[*slot].clone(),
                        Upvalue::Closed(val) => val.clone(),
                        Upvalue::Global(global_idx) => self.globals[*global_idx].clone(),
                    };
                    self.stack.push(val);
                    self.call_stack.last_mut().unwrap().ip += 3;
                }
                Op::SetFree => {
                    let idx = read_u16(bytecode, ip + 1);
                    let val = self.stack.pop().unwrap_or(Value::Null);
                    let closure_clone = closure_rc.clone();
                    let mut closure_mut = closure_clone.borrow_mut();
                    let upvalues = &mut closure_mut.upvalues;
                    if idx as usize >= upvalues.len() {
                        return Err(Error::Runtime {
            input: None,
            filename: None,
                            pos: None,
                            msg: "invalid free variable index".to_string(),
                        });
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
                    self.call_stack.last_mut().unwrap().ip += 3;
                }
                Op::Jump => {
                    let offset = read_i16(bytecode, ip + 1);
                    let target = ((ip as i16) + offset) as usize;
                    #[cfg(debug_assertions)]
                    eprintln!("DEBUG Jump: ip={}, offset={}, target={}, bytecode_len={}", ip, offset, target, bytecode.len());
                    if target >= bytecode.len() {
                        #[cfg(debug_assertions)]
                        eprintln!("ERROR: Jump target {} >= bytecode len {}", target, bytecode.len());
                    }
                    self.call_stack.last_mut().unwrap().ip = target;
                }
                Op::JumpIfFalse => {
                    let cond = self.stack.last().unwrap_or(&Value::Null).clone();
                    let offset = read_i16(bytecode, ip + 1);
                    if !is_truthy(&cond) {
                        self.call_stack.last_mut().unwrap().ip = ((ip as i16) + offset) as usize;
                    } else {
                        self.stack.pop();
                        self.call_stack.last_mut().unwrap().ip += 3;
                    }
                }
                Op::JumpIfTrue => {
                    let cond = self.stack.last().unwrap_or(&Value::Null).clone();
                    let offset = read_i16(bytecode, ip + 1);
                    if is_truthy(&cond) {
                        self.call_stack.last_mut().unwrap().ip = ((ip as i16) + offset) as usize;
                    } else {
                        self.stack.pop();
                        self.call_stack.last_mut().unwrap().ip += 3;
                    }
                }
                Op::JumpIfFalsePop => {
                    let cond = self.stack.pop().unwrap_or(Value::Null);
                    let offset = read_i16(bytecode, ip + 1);
                    let target = ((ip as i16) + offset) as usize;
                    #[cfg(debug_assertions)]
                    eprintln!("DEBUG JumpIfFalsePop: cond={:?}, ip={}, offset={}, target={}, stack={:?}", cond, ip, offset, target, self.stack);
                    if !is_truthy(&cond) {
                        self.call_stack.last_mut().unwrap().ip = target;
                    } else {
                        self.call_stack.last_mut().unwrap().ip += 3;
                    }
                }
                Op::Closure => {
                    let func_idx = read_u16(bytecode, ip + 1);
                    let num_free = read_u16(bytecode, ip + 3);
                    let constants = &closure_rc.borrow().func.constants;
                    let func = match &constants[func_idx as usize] {
                        Value::Func(f) => f.clone(),
                        _ => {
                            return Err(Error::Runtime {
            input: None,
            filename: None,
                                pos: None,
                                msg: "expected function in constant pool".to_string(),
                            })
                        }
                    };
                    let mut upvalues = Vec::new();
                    for _ in 0..num_free {
                        let val = self.stack.pop().unwrap_or(Value::Null);
                        match val {
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
                    self.call_stack.last_mut().unwrap().ip += 5;
                }
                Op::Call => {
                    let num_args = read_u16(bytecode, ip + 1);
                    #[cfg(debug_assertions)]
                    eprintln!("DEBUG: Call - stack: {:?}", self.stack);
                    let callee_idx = self.stack.len() - num_args as usize - 1;
                    if callee_idx >= self.stack.len() {
                        return Err(Error::Runtime {
            input: None,
            filename: None,
                            pos: None,
                            msg: "stack underflow".to_string(),
                        });
                    }
                    let callee = self.stack.remove(callee_idx);
                    #[cfg(debug_assertions)]
                    eprintln!("DEBUG: Call - callee: {:?}", callee);
                    let caller_ip = ip + 3;
                    match &callee {
                        Value::Closure(closure) => {
                            let func = &closure.borrow().func;
                            if func.num_params != num_args {
                                return Err(Error::Runtime {
            input: None,
            filename: None,
                                    pos: None,
                                    msg: format!(
                                        "expected {} arguments, got {}",
                                        func.num_params, num_args
                                    ),
                                });
                            }
                            let new_bp = self.stack.len() - num_args as usize;
                            self.stack
                                .resize(new_bp + (func.num_locals as usize), Value::Null);
                            self.call_stack.push(Frame {
                                closure: closure.clone(),
                                ip: 0,
                                bp: new_bp,
                            });
                        }
                        Value::Func(func) => {
                            if func.num_params != num_args {
                                return Err(Error::Runtime {
            input: None,
            filename: None,
                                    pos: None,
                                    msg: format!(
                                        "expected {} arguments, got {}",
                                        func.num_params, num_args
                                    ),
                                });
                            }
                            let closure = Rc::new(RefCell::new(Closure {
                                func: func.clone(),
                                upvalues: Vec::new(),
                            }));
                            let new_bp = self.stack.len() - num_args as usize;
                            self.stack
                                .resize(new_bp + (func.num_locals as usize), Value::Null);
                            self.call_stack.push(Frame {
                                closure: closure.clone(),
                                ip: 0,
                                bp: new_bp,
                            });
                        }
                        Value::Builtin(f) => {
                            f(self, num_args)?;
                        }
                        _ => {
                            return Err(Error::Runtime {
            input: None,
            filename: None,
                                pos: None,
                                msg: "cannot call non-function".to_string(),
                            })
                        }
                    }
                    match callee {
                        Value::Builtin(_) => {
                            self.call_stack.last_mut().unwrap().ip = caller_ip;
                        }
                        _ => {
                            let caller_frame_idx = self.call_stack.len() - 2;
                            self.call_stack[caller_frame_idx].ip = caller_ip;
                        }
                    }
                }
                Op::Return => {
                    let val = self.stack.pop().unwrap_or(Value::Null);
                    
                    // Check for finally blocks - if any exist, run them before returning
                    if !self.try_stack.is_empty() {
                        // There are finally blocks - store the return value
                        self.pending_return = Some(val.clone());
                        
                        // Jump to finally (keep frame on stack - CompleteReturn will pop it)
                        let try_frame = self.try_stack.pop().unwrap();
                        let jump_target = try_frame.finally_ip.unwrap_or(try_frame.end_ip);
                        
                        self.call_stack.last_mut().unwrap().ip = jump_target;
                        
                        // Restore stack height and push return value for finally to access
                        self.stack.truncate(try_frame.stack_height);
                        self.stack.push(val);
                    } else {
                        // No finally blocks, do normal return
                        let frame = self.call_stack.pop().unwrap();
                        self.stack.truncate(frame.bp);
                        if self.call_stack.is_empty() {
                            return Ok(val);
                        }
                        self.stack.push(val);
                    }
                }
                Op::CompleteReturn => {
                    // Called at end of finally block to complete the pending return
                    if let Some(val) = self.pending_return.take() {
                        // Check if there are more finally blocks to run
                        if !self.try_stack.is_empty() {
                            // More finally blocks - store value and jump to next
                            self.pending_return = Some(val.clone());
                            let try_frame = self.try_stack.pop().unwrap();
                            self.call_stack.last_mut().unwrap().ip = try_frame.finally_ip.unwrap_or(try_frame.end_ip);
                            self.stack.truncate(try_frame.stack_height);
                            self.stack.push(val);
                        } else {
                            // No more finally blocks - pop frame and complete the return
                            let frame = self.call_stack.pop().unwrap();
                            self.stack.truncate(frame.bp);
                            
                            if self.call_stack.is_empty() {
                                return Ok(val);
                            }
                            self.stack.push(val);
                        }
                    } else {
                        // No pending return - just continue normally
                        self.call_stack.last_mut().unwrap().ip += 1;
                    }
                }
                Op::GetBuiltin => {
                    let idx = read_u16(bytecode, ip + 1);
                    if idx as usize >= self.globals.len() {
                        return Err(Error::Runtime {
            input: None,
            filename: None,
                            pos: None,
                            msg: "invalid builtin index".to_string(),
                        });
                    }
                    self.stack.push(self.globals[idx as usize].clone());
                    self.call_stack.last_mut().unwrap().ip += 3;
                }
                Op::CaptureGlobal => {
                    let idx = read_u16(bytecode, ip + 1);
                    self.stack.push(Value::Num(-(idx as f64 + 1.0)));
                    self.call_stack.last_mut().unwrap().ip += 3;
                }
                Op::Array => {
                    let count = read_u16(bytecode, ip + 1) as usize;
                    let mut elements = Vec::with_capacity(count);
                    for _ in 0..count {
                        elements.push(self.stack.pop().unwrap_or(Value::Null));
                    }
                    elements.reverse();
                    self.stack.push(Value::Array(Rc::new(RefCell::new(elements))));
                    self.call_stack.last_mut().unwrap().ip += 3;
                }
                Op::Object => {
                    let count = read_u16(bytecode, ip + 1) as usize;
                    let mut map = HashMap::new();
                    let mut values = Vec::with_capacity(count * 2);
                    for _ in 0..count * 2 {
                        let value = self.stack.pop().unwrap_or(Value::Null);
                        values.push(value);
                    }
                    values.reverse();
                    for i in 0..count {
                        let name_idx = i * 2;
                        let val_idx = i * 2 + 1;
                        if name_idx < values.len() && val_idx < values.len() {
                            if let Value::Str(name) = values[name_idx].clone() {
                                map.insert(name, values[val_idx].clone());
                            }
                        }
                    }
                    self.stack.push(Value::Object(Rc::new(RefCell::new(map))));
                    self.call_stack.last_mut().unwrap().ip += 3;
                }
                Op::Index => {
                    let index = self.stack.pop().unwrap_or(Value::Null);
                    let object = self.stack.pop().unwrap_or(Value::Null);
                    match (object, index) {
                        (Value::Array(arr), Value::Num(idx)) => {
                            let idx = idx as usize;
                            let elements = arr.borrow();
                            if idx < elements.len() {
                                self.stack.push(elements[idx].clone());
                            } else {
                                self.stack.push(Value::Null);
                            }
                        }
                        (Value::Str(s), Value::Num(idx)) => {
                            let idx = idx as usize;
                            if idx < s.len() {
                                self.stack.push(Value::Str(s.chars().nth(idx).unwrap().to_string()));
                            } else {
                                self.stack.push(Value::Null);
                            }
                        }
                        (Value::Object(obj), Value::Str(key)) => {
                            let map = obj.borrow();
                            if let Some(val) = map.get(&key) {
                                self.stack.push(val.clone());
                            } else {
                                self.stack.push(Value::Null);
                            }
                        }
                        _ => {
                            return Err(Error::Runtime {
            input: None,
            filename: None,
                                pos: None,
                                msg: "invalid indexing operation".to_string(),
                            })
                        }
                    }
                    self.call_stack.last_mut().unwrap().ip += 1;
                }
                Op::IndexSet => {
                    let value = self.stack.pop().unwrap_or(Value::Null);
                    let index = self.stack.pop().unwrap_or(Value::Null);
                    let object = self.stack.pop().unwrap_or(Value::Null);
                    match (object, index, value) {
                        (Value::Array(arr), Value::Num(idx), val) => {
                            let idx = idx as usize;
                            let mut elements = arr.borrow_mut();
                            if idx < elements.len() {
                                elements[idx] = val;
                            } else if idx == elements.len() {
                                elements.push(val);
                            }
                            self.stack.push(Value::Null);
                        }
                        (Value::Object(obj), Value::Str(key), val) => {
                            obj.borrow_mut().insert(key, val);
                            self.stack.push(Value::Null);
                        }
                        _ => {
                            return Err(Error::Runtime {
            input: None,
            filename: None,
                                pos: None,
                                msg: "invalid index assignment".to_string(),
                            })
                        }
                    }
                    self.call_stack.last_mut().unwrap().ip += 1;
                }
                Op::GetField => {
                    let field = self.stack.pop().unwrap_or(Value::Null);
                    let object = self.stack.pop().unwrap_or(Value::Null);
                    if let (Value::Object(obj), Value::Str(field_name)) = (object, field) {
                        let map = obj.borrow();
                        if let Some(value) = map.get(&field_name) {
                            self.stack.push(value.clone());
                        } else {
                            self.stack.push(Value::Null);
                        }
                    } else {
                        return Err(Error::Runtime {
            input: None,
            filename: None,
                            pos: None,
                            msg: "invalid field access".to_string(),
                        });
                    }
                    self.call_stack.last_mut().unwrap().ip += 1;
                }
                Op::SetField => {
                    let value = self.stack.pop().unwrap_or(Value::Null);
                    let field = self.stack.pop().unwrap_or(Value::Null);
                    let object = self.stack.pop().unwrap_or(Value::Null);
                    if let (Value::Object(obj), Value::Str(field_name)) = (object, field) {
                        let mut map = obj.borrow_mut();
                        map.insert(field_name, value);
                        self.stack.push(Value::Null);
                    } else {
                        return Err(Error::Runtime {
            input: None,
            filename: None,
                            pos: None,
                            msg: "invalid field assignment".to_string(),
                        });
                    }
                    self.call_stack.last_mut().unwrap().ip += 1;
                }
                Op::EnumVariant => {
                    let value = self.stack.pop().unwrap_or(Value::Null);
                    let variant = self.stack.pop().unwrap_or(Value::Null);
                    let enum_name = self.stack.pop().unwrap_or(Value::Null);
                    if let (Value::Str(enum_name), Value::Str(variant)) = (enum_name, variant) {
                        let enum_val = Value::Enum {
                            enum_name,
                            variant,
                            value: Some(Box::new(value)),
                        };
                        self.stack.push(enum_val);
                    } else {
                        return Err(Error::Runtime {
            input: None,
            filename: None,
                            pos: None,
                            msg: "invalid enum variant".to_string(),
                        });
                    }
                    self.call_stack.last_mut().unwrap().ip += 1;
                }
                Op::IsEnumVariant => {
                    let variant = self.stack.pop().unwrap_or(Value::Null);
                    let enum_name = self.stack.pop().unwrap_or(Value::Null);
                    let value = self.stack.pop().unwrap_or(Value::Null);
                    if let (Value::Enum { enum_name: e, variant: v, .. }, Value::Str(enum_name), Value::Str(variant)) =
                        (&value, enum_name, variant)
                    {
                        self.stack.push(Value::Bool(*e == enum_name && *v == variant));
                    } else {
                        self.stack.push(Value::Bool(false));
                    }
                    self.call_stack.last_mut().unwrap().ip += 1;
                }
                Op::GetEnumValue => {
                    let value = self.stack.pop().unwrap_or(Value::Null);
                    if let Value::Enum { value: v, .. } = value {
                        if let Some(val) = v {
                            self.stack.push(*val);
                        } else {
                            self.stack.push(Value::Null);
                        }
                    } else {
                        return Err(Error::Runtime {
            input: None,
            filename: None,
                            pos: None,
                            msg: "expected enum value".to_string(),
                        });
                    }
                    self.call_stack.last_mut().unwrap().ip += 1;
                }
                Op::Ref => {
                    let val = self.stack.pop().unwrap_or(Value::Null);
                    self.stack.push(Value::Ref(Rc::new(RefCell::new(val))));
                    self.call_stack.last_mut().unwrap().ip += 1;
                }
                Op::RefMut => {
                    let val = self.stack.pop().unwrap_or(Value::Null);
                    self.stack.push(Value::MutRef(Rc::new(RefCell::new(val))));
                    self.call_stack.last_mut().unwrap().ip += 1;
                }
                Op::Deref => {
                    let val = self.stack.pop().unwrap_or(Value::Null);
                    match val {
                        Value::Ref(r) | Value::MutRef(r) => {
                            self.stack.push(r.borrow().clone());
                        }
                        _ => {
                            self.stack.push(val);
                        }
                    }
                    self.call_stack.last_mut().unwrap().ip += 1;
                }
                Op::IsOk => {
                    let val = self.stack.pop().unwrap_or(Value::Null);
                    match val {
                        Value::Result { is_ok, .. } => {
                            self.stack.push(Value::Bool(is_ok));
                        }
                        _ => {
                            return Err(Error::Runtime {
            input: None,
            filename: None,
                                pos: None,
                                msg: "is_ok expects Result value".to_string(),
                            })
                        }
                    }
                    self.call_stack.last_mut().unwrap().ip += 1;
                }
                Op::IsErr => {
                    let val = self.stack.pop().unwrap_or(Value::Null);
                    match val {
                        Value::Result { is_ok, .. } => {
                            self.stack.push(Value::Bool(!is_ok));
                        }
                        _ => {
                            return Err(Error::Runtime {
            input: None,
            filename: None,
                                pos: None,
                                msg: "is_err expects Result value".to_string(),
                            })
                        }
                    }
                    self.call_stack.last_mut().unwrap().ip += 1;
                }
                Op::IsSome => {
                    let val = self.stack.pop().unwrap_or(Value::Null);
                    match val {
                        Value::Option { is_some, .. } => {
                            self.stack.push(Value::Bool(is_some));
                        }
                        _ => {
                            return Err(Error::Runtime {
            input: None,
            filename: None,
                                pos: None,
                                msg: "is_some expects Option value".to_string(),
                            })
                        }
                    }
                    self.call_stack.last_mut().unwrap().ip += 1;
                }
                Op::IsNone => {
                    let val = self.stack.pop().unwrap_or(Value::Null);
                    match val {
                        Value::Option { is_some, .. } => {
                            self.stack.push(Value::Bool(!is_some));
                        }
                        _ => {
                            return Err(Error::Runtime {
            input: None,
            filename: None,
                                pos: None,
                                msg: "is_none expects Option value".to_string(),
                            })
                        }
                    }
                    self.call_stack.last_mut().unwrap().ip += 1;
                }
                Op::UnwrapOk => {
                    let val = self.stack.pop().unwrap_or(Value::Null);
                    match val {
                        Value::Result { is_ok, value } => {
                            if is_ok {
                                self.stack.push(*value);
                            } else {
                                let err_val = *value.clone();
                                self.exception = Some(*value);
                                return Err(Error::Runtime {
            input: None,
            filename: None,
                                    pos: None,
                                    msg: format!("unwrap called on Err value: {}", err_val),
                                });
                            }
                        }
                        _ => {
                            return Err(Error::Runtime {
            input: None,
            filename: None,
                                pos: None,
                                msg: "unwrap expects Result value".to_string(),
                            })
                        }
                    }
                    self.call_stack.last_mut().unwrap().ip += 1;
                }
                Op::UnwrapErr => {
                    let val = self.stack.pop().unwrap_or(Value::Null);
                    match val {
                        Value::Result { is_ok, value } => {
                            if !is_ok {
                                self.stack.push(*value);
                            } else {
                                let ok_val = *value.clone();
                                self.exception = Some(*value);
                                return Err(Error::Runtime {
            input: None,
            filename: None,
                                    pos: None,
                                    msg: format!("unwrap_err called on Ok value: {}", ok_val),
                                });
                            }
                        }
                        _ => {
                            return Err(Error::Runtime {
            input: None,
            filename: None,
                                pos: None,
                                msg: "unwrap_err expects Result value".to_string(),
                            })
                        }
                    }
                    self.call_stack.last_mut().unwrap().ip += 1;
                }
                Op::UnwrapSome => {
                    let val = self.stack.pop().unwrap_or(Value::Null);
                    match val {
                        Value::Option { is_some, value } => {
                            if is_some {
                                if let Some(v) = value {
                                    self.stack.push(*v);
                                } else {
                                    self.stack.push(Value::Null);
                                }
                            } else {
                                return Err(Error::Runtime {
            input: None,
            filename: None,
                                    pos: None,
                                    msg: "unwrap called on None value".to_string(),
                                });
                            }
                        }
                        _ => {
                            return Err(Error::Runtime {
            input: None,
            filename: None,
                                pos: None,
                                msg: "unwrap expects Option value".to_string(),
                            })
                        }
                    }
                    self.call_stack.last_mut().unwrap().ip += 1;
                }
                Op::MakeResult => {
                    let is_ok_byte = bytecode[ip + 1];
                    let is_ok = is_ok_byte != 0;
                    let val = self.stack.pop().unwrap_or(Value::Null);
                    self.stack.push(Value::Result {
                        is_ok,
                        value: Box::new(val),
                    });
                    self.call_stack.last_mut().unwrap().ip += 2;
                }
                Op::MakeOption => {
                    let is_some_byte = bytecode[ip + 1];
                    let is_some = is_some_byte != 0;
                    if is_some {
                        let val = self.stack.pop().unwrap_or(Value::Null);
                        self.stack.push(Value::Option {
                            is_some,
                            value: Some(Box::new(val)),
                        });
                    } else {
                        self.stack.push(Value::Option {
                            is_some: false,
                            value: None,
                        });
                    }
                    self.call_stack.last_mut().unwrap().ip += 2;
                }
                Op::Throw => {
                    let val = self.stack.pop().unwrap_or(Value::Null);
                    self.exception = Some(val.clone());
                    // Find the nearest try block to handle the exception
                    if let Some(try_frame) = self.try_stack.pop() {
                        self.stack.truncate(try_frame.stack_height);
                        if let Some(finally_ip) = try_frame.finally_ip {
                            self.call_stack.last_mut().unwrap().ip = finally_ip;
                        } else if let Some(catch_ip) = try_frame.catch_ip {
                            self.stack.push(val);
                            self.call_stack.last_mut().unwrap().ip = catch_ip;
                        } else {
                            return Err(Error::Runtime {
            input: None,
            filename: None,
                                pos: None,
                                msg: "unhandled exception".to_string(),
                            });
                        }
                    } else {
                        return Err(Error::Runtime {
            input: None,
            filename: None,
                            pos: None,
                            msg: format!("unhandled exception: {}", val),
                        });
                    }
                }
                Op::PushTry => {
                    let catch_offset = read_i16(bytecode, ip + 1);
                    let finally_offset = read_i16(bytecode, ip + 3);
                    let end_offset = read_i16(bytecode, ip + 5);
                    
                    let catch_ip = if catch_offset != 0 {
                        Some(((ip as i16) + catch_offset) as usize)
                    } else {
                        None
                    };
                    let finally_ip = if finally_offset != 0 {
                        Some(((ip as i16) + finally_offset) as usize)
                    } else {
                        None
                    };
                    let end_ip = ((ip as i16) + end_offset) as usize;
                    
                    self.try_stack.push(TryFrame {
                        catch_ip,
                        finally_ip,
                        end_ip,
                        stack_height: self.stack.len(),
                    });
                    self.call_stack.last_mut().unwrap().ip += 7;
                }
                Op::PopTry => {
                    self.try_stack.pop();
                    self.call_stack.last_mut().unwrap().ip += 1;
                }
                Op::Concat => {
                    let b = self.stack.pop().unwrap_or(Value::Null);
                    let a = self.stack.pop().unwrap_or(Value::Null);
                    match (a, b) {
                        (Value::Str(x), Value::Str(y)) => {
                            self.stack.push(Value::Str(format!("{}{}", x, y)));
                        }
                        _ => {
                            return Err(Error::Runtime {
            input: None,
            filename: None,
                                pos: None,
                                msg: "concat expects string operands".to_string(),
                            })
                        }
                    }
                    self.call_stack.last_mut().unwrap().ip += 1;
                }
                // Runtime type guards (gradual typing)
                Op::CheckNum => {
                    let val = self.stack.last().unwrap_or(&Value::Null).clone();
                    if !matches!(val, Value::Num(_)) {
                        return Err(Error::Runtime {
            input: None,
            filename: None,
                            pos: None,
                            msg: format!("TypeError: expected Num, got {:?}", val),
                        });
                    }
                    self.call_stack.last_mut().unwrap().ip += 1;
                }
                Op::CheckStr => {
                    let val = self.stack.last().unwrap_or(&Value::Null).clone();
                    if !matches!(val, Value::Str(_)) {
                        return Err(Error::Runtime {
            input: None,
            filename: None,
                            pos: None,
                            msg: format!("TypeError: expected Str, got {:?}", val),
                        });
                    }
                    self.call_stack.last_mut().unwrap().ip += 1;
                }
                Op::CheckBool => {
                    let val = self.stack.last().unwrap_or(&Value::Null).clone();
                    if !matches!(val, Value::Bool(_)) {
                        return Err(Error::Runtime {
            input: None,
            filename: None,
                            pos: None,
                            msg: format!("TypeError: expected Bool, got {:?}", val),
                        });
                    }
                    self.call_stack.last_mut().unwrap().ip += 1;
                }
                Op::ListComp => {
                    let func_idx = read_u16(bytecode, ip + 1) as usize;
                    let iterable = self.stack.pop().unwrap_or(Value::Null);
                    let array = match iterable {
                        Value::Array(arr) => arr,
                        _ => {
                            return Err(Error::Runtime {
            input: None,
            filename: None,
                                pos: None,
                                msg: "list comprehension requires an array".to_string(),
                            })
                        }
                    };
                    let elements = array.borrow().clone();
                    let mut result = Vec::new();
                    let func_val = closure_rc.borrow().func.constants[func_idx].clone();

                    for elem in &elements {
                        let func = match &func_val {
                            Value::Func(f) => {
                                let num_free = f.num_free as usize;
                                let mut upvalues = Vec::with_capacity(num_free);
                                for _ in 0..num_free {
                                    let uv_val = self.stack.pop().unwrap_or(Value::Null);
                                    if let Value::Num(n) = uv_val {
                                        if n < 0.0 {
                                            let global_idx = -(n + 1.0) as usize;
                                            upvalues.push(Upvalue::Global(global_idx));
                                        } else {
                                            upvalues.push(Upvalue::Closed(uv_val));
                                        }
                                    } else {
                                        upvalues.push(Upvalue::Closed(uv_val));
                                    }
                                }
                                upvalues.reverse();
                                Rc::new(RefCell::new(Closure {
                                    func: f.clone(),
                                    upvalues,
                                }))
                            }
                            Value::Closure(c) => c.clone(),
                            _ => {
                                return Err(Error::Runtime {
            input: None,
            filename: None,
                                    pos: None,
                                    msg: "expected function in list comprehension".to_string(),
                                })
                            }
                        };

                        if func.borrow().func.num_params != 1 {
                            return Err(Error::Runtime {
            input: None,
            filename: None,
                                pos: None,
                                msg: "list comprehension function expects 1 argument".to_string(),
                            });
                        }

                        let new_bp = self.stack.len();
                        self.stack.push(elem.clone());
                        let num_locals = func.borrow().func.num_locals as usize;
                        self.stack.resize(new_bp + num_locals, Value::Null);

                        self.call_stack.push(Frame {
                            closure: func.clone(),
                            ip: 0,
                            bp: new_bp,
                        });

                        loop {
                            let (fip, fbp, fbytecode_ptr, fbytecode_len, fclosure_rc) = {
                                let fframe = self.call_stack.last().unwrap();
                                let fclosure = fframe.closure.borrow();
                                let fbytecode = &fclosure.func.bytecode;
                                (
                                    fframe.ip,
                                    fframe.bp,
                                    fbytecode.as_ptr(),
                                    fbytecode.len(),
                                    fframe.closure.clone(),
                                )
                            };

                            let fbytecode: &[u8] = unsafe { std::slice::from_raw_parts(fbytecode_ptr, fbytecode_len) };

                            if fip >= fbytecode_len {
                                self.stack.truncate(fbp);
                                self.call_stack.pop();
                                self.stack.push(Value::Null);
                                break;
                            }

                            let fop = unsafe { std::mem::transmute::<u8, Op>(*fbytecode_ptr.add(fip)) };

                            match fop {
                                Op::Return => {
                                    let val = self.stack.pop().unwrap_or(Value::Null);
                                    let frame = self.call_stack.pop().unwrap();
                                    self.stack.truncate(frame.bp);
                                    self.stack.push(val);
                                    break;
                                }
                                Op::Constant => {
                                    let idx = read_u16(fbytecode, fip + 1);
                                    let val = fclosure_rc.borrow().func.constants[idx as usize].clone();
                                    self.stack.push(val);
                                    self.call_stack.last_mut().unwrap().ip += 3;
                                }
                                Op::GetLocal => {
                                    let idx = read_u16(fbytecode, fip + 1);
                                    let stack_idx = fbp + idx as usize;
                                    self.stack.push(self.stack[stack_idx].clone());
                                    self.call_stack.last_mut().unwrap().ip += 3;
                                }
                                Op::SetLocal => {
                                    let idx = read_u16(fbytecode, fip + 1);
                                    let val = self.stack.pop().unwrap_or(Value::Null);
                                    let stack_idx = fbp + idx as usize;
                                    self.stack[stack_idx] = val;
                                    self.call_stack.last_mut().unwrap().ip += 3;
                                }
                                Op::Null => {
                                    self.stack.push(Value::Null);
                                    self.call_stack.last_mut().unwrap().ip += 1;
                                }
                                Op::True => {
                                    self.stack.push(Value::Bool(true));
                                    self.call_stack.last_mut().unwrap().ip += 1;
                                }
                                Op::False => {
                                    self.stack.push(Value::Bool(false));
                                    self.call_stack.last_mut().unwrap().ip += 1;
                                }
                                Op::Negate => {
                                    let a = self.stack.pop().unwrap_or(Value::Null);
                                    match a {
                                        Value::Num(x) => self.stack.push(Value::Num(-x)),
                                        _ => self.stack.push(Value::Null),
                                    }
                                    self.call_stack.last_mut().unwrap().ip += 1;
                                }
                                Op::Add => {
                                    let b = self.stack.pop().unwrap_or(Value::Null);
                                    let a = self.stack.pop().unwrap_or(Value::Null);
                                    let res = match (a, b) {
                                        (Value::Num(x), Value::Num(y)) => Value::Num(x + y),
                                        (Value::Str(x), Value::Str(y)) => Value::Str(format!("{}{}", x, y)),
                                        _ => Value::Null,
                                    };
                                    self.stack.push(res);
                                    self.call_stack.last_mut().unwrap().ip += 1;
                                }
                                Op::Sub => {
                                    let b = self.stack.pop().unwrap_or(Value::Null);
                                    let a = self.stack.pop().unwrap_or(Value::Null);
                                    let res = match (a, b) {
                                        (Value::Num(x), Value::Num(y)) => Value::Num(x - y),
                                        _ => Value::Null,
                                    };
                                    self.stack.push(res);
                                    self.call_stack.last_mut().unwrap().ip += 1;
                                }
                                Op::Mul => {
                                    let b = self.stack.pop().unwrap_or(Value::Null);
                                    let a = self.stack.pop().unwrap_or(Value::Null);
                                    let res = match (a, b) {
                                        (Value::Num(x), Value::Num(y)) => Value::Num(x * y),
                                        _ => Value::Null,
                                    };
                                    self.stack.push(res);
                                    self.call_stack.last_mut().unwrap().ip += 1;
                                }
                                Op::Div => {
                                    let b = self.stack.pop().unwrap_or(Value::Null);
                                    let a = self.stack.pop().unwrap_or(Value::Null);
                                    let res = match (a, b) {
                                        (Value::Num(x), Value::Num(y)) => {
                                            if y == 0.0 { Value::Null } else { Value::Num(x / y) }
                                        }
                                        _ => Value::Null,
                                    };
                                    self.stack.push(res);
                                    self.call_stack.last_mut().unwrap().ip += 1;
                                }
                                Op::Pop => {
                                    self.stack.pop();
                                    self.call_stack.last_mut().unwrap().ip += 1;
                                }
                                Op::Eq => {
                                    let b = self.stack.pop().unwrap_or(Value::Null);
                                    let a = self.stack.pop().unwrap_or(Value::Null);
                                    self.stack.push(Value::Bool(values_eq(&a, &b)));
                                    self.call_stack.last_mut().unwrap().ip += 1;
                                }
                                Op::Neq => {
                                    let b = self.stack.pop().unwrap_or(Value::Null);
                                    let a = self.stack.pop().unwrap_or(Value::Null);
                                    self.stack.push(Value::Bool(!values_eq(&a, &b)));
                                    self.call_stack.last_mut().unwrap().ip += 1;
                                }
                                Op::Lt => {
                                    let b = self.stack.pop().unwrap_or(Value::Null);
                                    let a = self.stack.pop().unwrap_or(Value::Null);
                                    let res = match (a, b) {
                                        (Value::Num(x), Value::Num(y)) => Value::Bool(x < y),
                                        _ => Value::Bool(false),
                                    };
                                    self.stack.push(res);
                                    self.call_stack.last_mut().unwrap().ip += 1;
                                }
                                Op::Gt => {
                                    let b = self.stack.pop().unwrap_or(Value::Null);
                                    let a = self.stack.pop().unwrap_or(Value::Null);
                                    let res = match (a, b) {
                                        (Value::Num(x), Value::Num(y)) => Value::Bool(x > y),
                                        _ => Value::Bool(false),
                                    };
                                    self.stack.push(res);
                                    self.call_stack.last_mut().unwrap().ip += 1;
                                }
                                Op::GetField => {
                                    let field = self.stack.pop().unwrap_or(Value::Null);
                                    let object = self.stack.pop().unwrap_or(Value::Null);
                                    if let (Value::Object(obj), Value::Str(field_name)) = (object, field) {
                                        let map = obj.borrow();
                                        if let Some(value) = map.get(&field_name) {
                                            self.stack.push(value.clone());
                                        } else {
                                            self.stack.push(Value::Null);
                                        }
                                    } else {
                                        self.stack.push(Value::Null);
                                    }
                                    self.call_stack.last_mut().unwrap().ip += 1;
                                }
                                Op::Index => {
                                    let index = self.stack.pop().unwrap_or(Value::Null);
                                    let object = self.stack.pop().unwrap_or(Value::Null);
                                    match (object, index) {
                                        (Value::Array(arr), Value::Num(idx)) => {
                                            let idx = idx as usize;
                                            let elems = arr.borrow();
                                            if idx < elems.len() {
                                                self.stack.push(elems[idx].clone());
                                            } else {
                                                self.stack.push(Value::Null);
                                            }
                                        }
                                        (Value::Str(s), Value::Num(idx)) => {
                                            let idx = idx as usize;
                                            if idx < s.len() {
                                                self.stack.push(Value::Str(s.chars().nth(idx).unwrap().to_string()));
                                            } else {
                                                self.stack.push(Value::Null);
                                            }
                                        }
                                        (Value::Object(obj), Value::Str(key)) => {
                                            let map = obj.borrow();
                                            if let Some(val) = map.get(&key) {
                                                self.stack.push(val.clone());
                                            } else {
                                                self.stack.push(Value::Null);
                                            }
                                        }
                                        _ => self.stack.push(Value::Null),
                                    }
                                    self.call_stack.last_mut().unwrap().ip += 1;
                                }
                                Op::GetFree => {
                                    let idx = read_u16(fbytecode, fip + 1);
                                    let upvalues = &fclosure_rc.borrow().upvalues;
                                    if idx as usize >= upvalues.len() {
                                        self.stack.push(Value::Null);
                                    } else {
                                        let val = match &upvalues[idx as usize] {
                                            Upvalue::Open(slot) => self.stack[*slot].clone(),
                                            Upvalue::Closed(val) => val.clone(),
                                            Upvalue::Global(global_idx) => self.globals[*global_idx].clone(),
                                        };
                                        self.stack.push(val);
                                    }
                                    self.call_stack.last_mut().unwrap().ip += 3;
                                }
                                Op::CaptureGlobal => {
                                    let idx = read_u16(fbytecode, fip + 1);
                                    self.stack.push(Value::Num(-(idx as f64 + 1.0)));
                                    self.call_stack.last_mut().unwrap().ip += 3;
                                }
                                Op::Concat => {
                                    let b = self.stack.pop().unwrap_or(Value::Null);
                                    let a = self.stack.pop().unwrap_or(Value::Null);
                                    match (a, b) {
                                        (Value::Str(x), Value::Str(y)) => {
                                            self.stack.push(Value::Str(format!("{}{}", x, y)));
                                        }
                                        _ => self.stack.push(Value::Null),
                                    }
                                    self.call_stack.last_mut().unwrap().ip += 1;
                                }
                                _ => {
                                    self.call_stack.last_mut().unwrap().ip += 1;
                                }
                            }
                        }

                        let res = self.stack.pop().unwrap_or(Value::Null);
                        result.push(res);
                    }
                    self.stack.push(Value::Array(Rc::new(RefCell::new(result))));
                    self.call_stack.last_mut().unwrap().ip += 3;
                }
            }
        }
    }
}

#[inline]
fn is_truthy(val: &Value) -> bool {
    match val {
        Value::Null => false,
        Value::Bool(b) => *b,
        _ => true,
    }
}

#[inline]
fn values_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Num(x), Value::Num(y)) => x == y,
        (Value::Str(x), Value::Str(y)) => x == y,
        (Value::Array(x), Value::Array(y)) => {
            let xb = x.borrow();
            let yb = y.borrow();
            if xb.len() != yb.len() {
                return false;
            }
            for (xv, yv) in xb.iter().zip(yb.iter()) {
                if !values_eq(xv, yv) {
                    return false;
                }
            }
            true
        }
        (Value::Object(x), Value::Object(y)) => {
            let xm = x.borrow();
            let ym = y.borrow();
            if xm.len() != ym.len() {
                return false;
            }
            for (k, xv) in xm.iter() {
                if let Some(yv) = ym.get(k) {
                    if !values_eq(xv, yv) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            true
        }
        (Value::Enum { enum_name: xn, variant: xv, value: xval },
         Value::Enum { enum_name: yn, variant: yv, value: yval }) => {
            if xn != yn || xv != yv {
                return false;
            }
            match (xval, yval) {
                (Some(x), Some(y)) => values_eq(x, y),
                (None, None) => true,
                _ => false,
            }
        }
        (Value::Result { is_ok: x_ok, value: xval },
         Value::Result { is_ok: y_ok, value: yval }) => {
            x_ok == y_ok && values_eq(xval, yval)
        }
        (Value::Option { is_some: x_some, value: xval },
         Value::Option { is_some: y_some, value: yval }) => {
            if x_some != y_some {
                return false;
            }
            match (xval, yval) {
                (Some(x), Some(y)) => values_eq(x, y),
                (None, None) => true,
                _ => false,
            }
        }
        (Value::Ref(x), Value::Ref(y)) | (Value::MutRef(x), Value::MutRef(y)) => {
            Rc::ptr_eq(x, y)
        }
        _ => false,
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
            Value::Array(arr) => {
                let elements = arr.borrow();
                write!(f, "[")?;
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", elem)?;
                }
                write!(f, "]")
            }
            Value::Object(obj) => {
                let map = obj.borrow();
                write!(f, "{{")?;
                for (i, (k, v)) in map.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
            Value::Enum { enum_name, variant, value } => {
                if let Some(val) = value {
                    write!(f, "{}::{}({})", enum_name, variant, val)
                } else {
                    write!(f, "{}::{}", enum_name, variant)
                }
            }
            Value::Result { is_ok, value } => {
                if *is_ok {
                    write!(f, "Ok({})", value)
                } else {
                    write!(f, "Err({})", value)
                }
            }
            Value::Option { is_some, value } => {
                if *is_some {
                    if let Some(val) = value {
                        write!(f, "Some({})", val)
                    } else {
                        write!(f, "Some")
                    }
                } else {
                    write!(f, "None")
                }
            }
            Value::Func(_) => write!(f, "<function>"),
            Value::Closure(_) => write!(f, "<closure>"),
            Value::Builtin(_) => write!(f, "<builtin>"),
            Value::Ref(r) => write!(f, "&{}", r.borrow()),
            Value::MutRef(r) => write!(f, "&mut {}", r.borrow()),
        }
    }
}

fn builtin_print(vm: &mut VM, num_args: u16) -> Result<(), Error> {
    let n = num_args as usize;
    let start = vm.stack.len() - n;
    for i in 0..n {
        if i > 0 {
            print!(" ");
        }
        print!("{}", vm.stack[start + i]);
    }
    println!();
    vm.stack.truncate(start);
    vm.stack.push(Value::Null);
    Ok(())
}

fn builtin_len(vm: &mut VM, num_args: u16) -> Result<(), Error> {
    if num_args != 1 {
        return Err(Error::Runtime {
            input: None,
            filename: None,
            pos: None,
            msg: format!("len expects 1 argument, got {}", num_args),
        });
    }
    let val = vm.stack.pop().unwrap_or(Value::Null);
    let len = match val {
        Value::Str(s) => s.len() as f64,
        Value::Array(arr) => arr.borrow().len() as f64,
        _ => {
            return Err(Error::Runtime {
            input: None,
            filename: None,
                pos: None,
                msg: "len expects string or array".to_string(),
            })
        }
    };
    vm.stack.push(Value::Num(len));
    Ok(())
}

fn builtin_push(vm: &mut VM, num_args: u16) -> Result<(), Error> {
    if num_args != 2 {
        return Err(Error::Runtime {
            input: None,
            filename: None,
            pos: None,
            msg: format!("push expects 2 arguments, got {}", num_args),
        });
    }
    let val = vm.stack.pop().unwrap_or(Value::Null);
    let arr = vm.stack.pop().unwrap_or(Value::Null);
    match arr {
        Value::Array(array) => {
            array.borrow_mut().push(val);
            vm.stack.push(Value::Null);
        }
        _ => {
            return Err(Error::Runtime {
            input: None,
            filename: None,
                pos: None,
                msg: "push expects array as first argument".to_string(),
            })
        }
    }
    Ok(())
}

fn builtin_pop(vm: &mut VM, num_args: u16) -> Result<(), Error> {
    if num_args != 1 {
        return Err(Error::Runtime {
            input: None,
            filename: None,
            pos: None,
            msg: format!("pop expects 1 argument, got {}", num_args),
        });
    }
    let arr = vm.stack.pop().unwrap_or(Value::Null);
    match arr {
        Value::Array(array) => {
            if let Some(val) = array.borrow_mut().pop() {
                vm.stack.push(val);
            } else {
                vm.stack.push(Value::Null);
            }
        }
        _ => {
            return Err(Error::Runtime {
            input: None,
            filename: None,
                pos: None,
                msg: "pop expects array as argument".to_string(),
            })
        }
    }
    Ok(())
}

fn builtin_is_ok(vm: &mut VM, num_args: u16) -> Result<(), Error> {
    if num_args != 1 {
        return Err(Error::Runtime {
            input: None,
            filename: None,
            pos: None,
            msg: format!("is_ok expects 1 argument, got {}", num_args),
        });
    }
    let val = vm.stack.pop().unwrap_or(Value::Null);
    match val {
        Value::Result { is_ok, .. } => {
            vm.stack.push(Value::Bool(is_ok));
        }
        _ => {
            return Err(Error::Runtime {
            input: None,
            filename: None,
                pos: None,
                msg: "is_ok expects Result value".to_string(),
            })
        }
    }
    Ok(())
}

fn builtin_is_err(vm: &mut VM, num_args: u16) -> Result<(), Error> {
    if num_args != 1 {
        return Err(Error::Runtime {
            input: None,
            filename: None,
            pos: None,
            msg: format!("is_err expects 1 argument, got {}", num_args),
        });
    }
    let val = vm.stack.pop().unwrap_or(Value::Null);
    match val {
        Value::Result { is_ok, .. } => {
            vm.stack.push(Value::Bool(!is_ok));
        }
        _ => {
            return Err(Error::Runtime {
            input: None,
            filename: None,
                pos: None,
                msg: "is_err expects Result value".to_string(),
            })
        }
    }
    Ok(())
}

fn builtin_is_some(vm: &mut VM, num_args: u16) -> Result<(), Error> {
    if num_args != 1 {
        return Err(Error::Runtime {
            input: None,
            filename: None,
            pos: None,
            msg: format!("is_some expects 1 argument, got {}", num_args),
        });
    }
    let val = vm.stack.pop().unwrap_or(Value::Null);
    match val {
        Value::Option { is_some, .. } => {
            vm.stack.push(Value::Bool(is_some));
        }
        _ => {
            return Err(Error::Runtime {
            input: None,
            filename: None,
                pos: None,
                msg: "is_some expects Option value".to_string(),
            })
        }
    }
    Ok(())
}

fn builtin_is_none(vm: &mut VM, num_args: u16) -> Result<(), Error> {
    if num_args != 1 {
        return Err(Error::Runtime {
            input: None,
            filename: None,
            pos: None,
            msg: format!("is_none expects 1 argument, got {}", num_args),
        });
    }
    let val = vm.stack.pop().unwrap_or(Value::Null);
    match val {
        Value::Option { is_some, .. } => {
            vm.stack.push(Value::Bool(!is_some));
        }
        _ => {
            return Err(Error::Runtime {
            input: None,
            filename: None,
                pos: None,
                msg: "is_none expects Option value".to_string(),
            })
        }
    }
    Ok(())
}

fn builtin_unwrap(vm: &mut VM, num_args: u16) -> Result<(), Error> {
    if num_args != 1 {
        return Err(Error::Runtime {
            input: None,
            filename: None,
            pos: None,
            msg: format!("unwrap expects 1 argument, got {}", num_args),
        });
    }
    let val = vm.stack.pop().unwrap_or(Value::Null);
    match val {
        Value::Result { is_ok, value } => {
            if is_ok {
                vm.stack.push(*value);
            } else {
                let err_val = *value.clone();
                vm.exception = Some(*value);
                return Err(Error::Runtime {
            input: None,
            filename: None,
                    pos: None,
                    msg: format!("unwrap called on Err value: {}", err_val),
                });
            }
        }
        Value::Option { is_some, value } => {
            if is_some {
                if let Some(v) = value {
                    vm.stack.push(*v);
                } else {
                    vm.stack.push(Value::Null);
                }
            } else {
                return Err(Error::Runtime {
            input: None,
            filename: None,
                    pos: None,
                    msg: "unwrap called on None value".to_string(),
                });
            }
        }
        _ => {
            return Err(Error::Runtime {
            input: None,
            filename: None,
                pos: None,
                msg: "unwrap expects Result or Option value".to_string(),
            })
        }
    }
    Ok(())
}

fn builtin_unwrap_or(vm: &mut VM, num_args: u16) -> Result<(), Error> {
    if num_args != 2 {
        return Err(Error::Runtime {
            input: None,
            filename: None,
            pos: None,
            msg: format!("unwrap_or expects 2 arguments, got {}", num_args),
        });
    }
    let default = vm.stack.pop().unwrap_or(Value::Null);
    let val = vm.stack.pop().unwrap_or(Value::Null);
    match val {
        Value::Result { is_ok, value } => {
            if is_ok {
                vm.stack.push(*value);
            } else {
                vm.stack.push(default);
            }
        }
        Value::Option { is_some, value } => {
            if is_some {
                if let Some(v) = value {
                    vm.stack.push(*v);
                } else {
                    vm.stack.push(Value::Null);
                }
            } else {
                vm.stack.push(default);
            }
        }
        _ => {
            return Err(Error::Runtime {
            input: None,
            filename: None,
                pos: None,
                msg: "unwrap_or expects Result or Option value".to_string(),
            })
        }
    }
    Ok(())
}
