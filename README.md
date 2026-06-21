# qwqlang  ✨

Ciallo~  Welcome to **qwqlang** – a smow, dynamwicawly typed scwipting wanguage with a Rust‑based compiwew and a stack‑based viwtuaw machine. It bwends familiaw syntax fwom Rust, JavaScwipt, and Ewixiw, and offews modern featuwes wike pipe opewatows, wabewwed woops, and fiwst‑cwass cwosuwes with expwicit captuwes. ～☆

## Features ～✧

- **Vawiabwes** – immutabwe (`let`) and mutabwe (`mut`)
- **Awithmetic & Compawisons** – `+`, `-`, `*`, `/`, `==`, `!=`, `<`, `>`, `<=`, `>=`
- **Logica opewatows** – `and` / `or` with showt‑ciwcuit evawuation
- **Contwow fwow** – `if`/`else`, `loop`, `break` (with optionaw wabew and wetuwn vawue)
- **Functions** – named (`fn`) and anonnymous awWow functions (`|x, y| x + y`)
- **Cwosuwes** – captuwe outew vawiabwes expwicitwy using `[captuwe]` syntax
- **Pipe opewatow** – `|>` with `_` pwacehowdew fow concwise chaining
- **Stwings** – doubwe‑qwoted, with concatenation via `+`
- **EwWow messages** – hewpfuw suggesstions fow undefined vawiabwes (Levenshtein distance)
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

You'ww see a pwompt `qWQ>`. Entew expwessions ow statements; the wesuwt (if not `null`) wiww be pwinted. ✨

### Run a script

```bash
cargo run -- < script.qwq
```

Or use the wibwawy in youw own Rust pwoject:

```toml
[dependencies]
qwqlang = { git = "https://github.com/0xA672/qwqlang" }
```

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

### Mutable Vawiabwes

```
mut counter = 0;
counter = counter + 1;
counter   // 1
```

### Stwings

```
let greeting = "Ciallo";
let name = "World";
greeting + " " + name   // "Ciallo World"
```

### If/Else

```
if (true) { 1 } else { 2 }   // 1
```

### Loops & Break

```
// Infinite loop, break with a value
loop { break 42; }   // 42

// Labelled break
'outer loop {
    'inner loop {
        break 'outer 100;
    }
}   // 100
```

### Functions

```
fn add(x, y) { x + y; }
add(3, 4)   // 7

// Arrow syntax
let double = |x| x * 2;
double(5)   // 10
```

### Cwosuwes & Expwicit Captuwes

```
mut x = 0;
let inc = fn() [mut x] {
    x = x + 1;
};
inc();
inc();
x   // 2
```

### Pipe Opewatow with Pwacehowdew

```
42 |> _ + 1                // 43
"hello" |> _ + " world"    // "hello world"
10 |> _ * 2 |> _ + 5       // 25
```

### Logical Short‑CiWcuit

```
false and print("no")   // false, print not called
true or  print("no")    // true,  print not called
true and 42             // 42
false or "hello"        // "hello"
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
- `compiler.rs` – emits bytecode fwom the AST
- `vm.rs` – executes bytecode on a stack‑based VM
- `error.rs` – custome ewwow types with pwetty pwinting
- `lib.rs` / `main.rs` – pubwic API and REPL

## Contwibuting

Issues and puww wequests awe wewcome! Pwease make suwe tests pass and code is fowmatted with `cargo fmt`. ～☆

## License

MIT © 2026 0xA672
