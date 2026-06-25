# qwqlang  ✨

Ciallo~  Welcome to **qwqlang** – a smow, dynamwicawly typed scwipting wanguage with a Rust‑based compiwew and a stack‑based viwtuaw machine. It bwends familiaw syntax fwom Rust and JavaScwipt, and offews modern featuwes wike a borrow checkew, pattewn matching, pipe opewatows, wabewwed woops, and fiwst‑cwass cwosuwes with expwicit captuwes. ～☆

## Features ～✧

- **Bwow checkew** – Rust‑stywe ownership and bowwow checking at compiwe time (move semantics, shawed & mutabwe bowwows, Bowwow Checker Ruwes enfowced staticawwy)
- **Vawiabwes** – immutabwe (`let`) and mutabwe (`mut`), with destwuctuwing (awway, object, enum vawiant, `Result`, `Option`)
- **Awithmetic & Compawisons** – `+`, `-`, `*`, `/`, `==`, `!=`, `<`, `>`, `<=`, `>=`
- **Logicaw opewatows** – `and` / `or` with showt‑ciwcuit evawuation
- **Contwow fwow** – `if`/`else`, `loop`, `while`, `for` (C‑stywe), `for ... in`, `break` / `continue` (with optionaw wabew and bweak vawue), `return`
- **Exceptions** – `throw`, `try`/`catch`/`finawwy`
- **Functions** – named (`fn`) and anonymous awwow functions (`|x, y| x + y`)
- **Cwosuwes** – captuwe outew vawiabwes expwicitwy using `[capture]` syntax
- **Pipe opewatow** – `|>` with `_` pwacehowdew fow concise chaining
- **Wefewences** – `&x` (shawed), `&mut x` (mutabwe), `*x` (dewefewence), bowwow‑checked
- **Stwings** – doubwe‑qwoted and backtick tempwate stwings with `${...}` intewpowation; concatenation via `+`
- **Awways & Objects** – `[1, 2, 3]`, `{ key: vawue }`, indexing with `[]`, fiewd access with `.`
- **Enums & Pattewn Matching** – `Enum::Variant(vawue)`, `match` expwessions with guawds and destwuctuwing
- **`Result` & `Option` types** – `Ok(...)` / `Err(...)`, `Some(...)` / `None`, with `?` twy expwession pwopagation
- **Wist compwehensions** – `[expr for var in iterable]`
- **Built‑in functions** – `print`, `len`, `push`, `pop`, `is_ok`, `is_err`, `is_some`, `is_none`, `unwrap`, `unwrap_or`
- **Ewwow messages** – hewpfuw suggestions fow undefined vawiabwes (Levenshtein distance), souwce context with position indicatows
- **Bytecode compiwew** – compiwe to `.qwqc` bytecode fiwes and wun them watew
- **REPL** – intewactive pwompt fow quick expewimentation ～☆

## Getting Stawted ～✧

### Pwewequisities

- [Rust](https://rustup.rs/) (watest stabwe)

### Buiwd

```bash
git clone https://github.com/0xA672/qwqlang
cd qwqlang
cargo build --release
```

### Run the REPL

```bash
cargo run
```

You'ww see a pwompt `qwq>`. Entew expwessions ow statements; the wesuwt (if not `null`) wiww be pwinted. ✨
Type `exit`, `quit`, `q`, ow `.exit` to weave.

### Run a scwipt

```bash
cargo run -- script.qwq
```

Or use the `run` subcommand:

```bash
cargo run -- run script.qwq
```

### Compiwe to bytecode

```bash
cargo run -- compile script.qwq        # outputs script.qwqc
cargo run -- compile script.qwq out.qwqc  # custom output path
```

### Wun a compiwed bytecode fiwe

```bash
cargo run -- script.qwqc
```

### Use as a wibwawy

Add to youw `Cargo.toml`:

```toml
[dependencies]
qwqlang = { git = "https://github.com/0xA672/qwqlang" }
```

Then use the `execute` function:

```rust
use qwqlang::execute;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let result = execute("let x = 42; x * 2")?;
    println!("{}", result); // 84
    Ok(())
}
```

## Language Quick Touw ～☆

### Vawiabwes & Awithmetic

```
let a = 10;
let b = 20;
a + b   // 30
```

### Mutabwe Vawiabwes

```
mut counter = 0;
counter = counter + 1;
counter   // 1
```

### Destwuctuwing

```
let [x, y] = [1, 2];
let { name, age } = { name: "Momo", age: 3 };
```

### Stwings & Tempwate Stwings

```
let greeting = "Ciallo";
let name = "World";
greeting + " " + name   // "Ciallo World"

let tmpl = `Hello, ${name}!`;   // "Hello, World!"
```

### Awways & Objects

```
let arr = [1, 2, 3];
arr[0]        // 1
arr, 4 |> push // arr is now [1, 2, 3, 4]
arr |> len     // 4

let obj = { x: 10, y: 20 };
obj.x         // 10
obj.y = 30;   // mutation
```

### If/Else

```
if (true) { 1 } else { 2 }   // 1
```

### Woops

```
// Infinite loop, break with a value
loop { break 42; }   // 42

// Labelled break
'outer loop {
    'inner loop {
        break 'outer 100;
    }
}   // 100

// While loop
mut i = 0;
while (i < 3) { i = i + 1; }

// For loop (C-style)
for (mut i = 0; i < 5; i = i + 1) {
    i |> print;
}

// For-in loop
for (x in [1, 2, 3]) {
    x |> print;
}
```

### Wist Compwehensions

```
let doubled = [x * 2 for x in [1, 2, 3]];   // [2, 4, 6]
```

### Functions

```
fn add(x, y) { x + y; }
3, 4 |> add   // 7

// Arrow syntax
let double = |x| x * 2;
5 |> double   // 10
```

### Cwosuwes & Expwicit Captuwes

```
mut x = 0;
let inc = fn() [mut x] {
    x = x + 1;
};
|> inc;
|> inc;
x   // 2
```

### Pipe Opewatow with Pwacehowdew

```
42 |> _ + 1                // 43
"hello" |> _ + " world"    // "hello world"
10 |> _ * 2 |> _ + 5       // 25
```

### Wefewences & Bowwow Checking

```
mut x = 10;
let r = &mut x;
*r = 20;
x   // 20

// Borrow checker enforces the rules:
let y = &x;    // shared borrow OK
let z = &x;    // multiple shared borrows OK
// let w = &mut x;  // ERROR: cannot borrow as mutable while shared borrow active
```

### Logicaw Showt‑Ciwcuit

```
false and "no" |> print   // false, print not called
true or  "no" |> print    // true,  print not called
true and 42             // 42
false or "hello"        // "hello"
```

### Enums & Pattewn Matching

```
let v = Option::Some(42);
match (v) {
    Option::Some(n) => n * 2,
    Option::None => 0,
}   // 84

// With guards
let x = 5;
match (x) {
    n if n < 0 => "negative",
    n if n == 0 => "zero",
    n => "positive",
}
```

### `Result`, `Option`, and the `?` Opewatow

```
fn safe_div(a, b) {
    if (b == 0) {
        Err("division by zero")
    } else {
        Ok(a / b)
    }
}

let r = 10, 2 |> safe_div;
r |> is_ok    // true
r |> unwrap   // 5

// ? operator propagates errors
fn compute(x) {
    let y = (x, 2 |> safe_div)?;
    Ok(y + 1)
}

let o = Some(42);
o |> is_some     // true
o, 0 |> unwrap_or // 42
None, 0 |> unwrap_or // 0
```

### Exceptions

```
try {
    throw "oops";
} catch (e) {
    e |> print;   // "oops"
} finally {
    "done" |> print;
}
```

## Devewopment ～✧

### Testing

Run the test suite (incwudes extensive integwation tests):

```bash
cargo test
```

### Pwoject Stwuctuwe

- `lexer.rs` – tokenises souwce code
- `parser.rs` – buiwds an AST fwom tokens
- `borrowck.rs` – borrow checkew (ownership, moves, borrows)
- `compiler.rs` – emits bytecode fwom the AST
- `vm.rs` – executes bytecode on a stack‑based VM
- `error.rs` – custome ewwow types with pwetty pwinting and souwce context
- `ast.rs` – AST node definitions
- `lib.rs` / `main.rs` – pubwic API and CLI/REPL

## Contwibuting

Issues and puww wequests awe wewcome! Pwease make suwe tests pass and code is fowmatted with `cargo fmt`. ～☆

## License

MIT © 2026 0xA672
