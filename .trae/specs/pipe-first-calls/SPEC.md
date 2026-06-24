# Pipe-First Call Syntax Spec

## Why
qwq already has first-class pipe operator `|>`. Users want to simplify function calls to use pipe syntax exclusively, removing traditional `f(x)` call syntax.

## What Changes
- **BREAKING**: Remove `f(args)` function call syntax entirely
- Add pipe with multiple arguments: `x, y, z |> f` equivalent to `f(x, y, z)`
- Control flow statements (`while`, `for`, `loop`, `if`) remain unchanged
- Method access `obj.field` remains unchanged
- Function definition `fn foo() {}` remains unchanged

## Impact
- Affected specs: All function call sites
- Affected code: parser.rs, compiler.rs

## ADDED Requirements

### Requirement: Pipe with Multiple Arguments
The system SHALL support multiple arguments in pipe expressions.

#### Scenario: Multi-argument pipe
- **WHEN** user writes `x, y, z |> func`
- **THEN** system compiles to equivalent of `func(x, y, z)`

#### Scenario: Chain with multi-argument
- **WHEN** user writes `a, b |> f |> c, d |> g`
- **THEN** system compiles to equivalent of `g(f(a, b), c, d)`

### Requirement: Preserve Existing Syntax
The system SHALL preserve these existing syntaxes unchanged:
- Function definition: `fn foo(a, b) { a + b }`
- Method access: `obj.field`, `arr.length`
- Control flow: `while`, `for`, `loop`, `if`, `match`
- Object/array literals: `{a: 1}`, `[1, 2, 3]`

## MODIFIED Requirements

### Requirement: Function Call
**Old**: `func(arg1, arg2)` syntax for function calls
**New**: Only `args |> func` syntax for function calls
**Migration**: Convert all `f(x)` to `x |> f`, `f(x, y)` to `x, y |> f`

## REMOVED Requirements

### Requirement: Traditional Call Syntax
**Reason**: Moving to pipe-first design philosophy
**Migration**: All function calls must use pipe syntax

## Examples

```qwq
// OLD (removed)
add(1, 2)
print("hello")
map(arr, fn(x) { x * 2 })

// NEW
1, 2 |> add
"hello" |> print
arr |> map fn(x) { x * 2 }

// Chain
1, 2 |> add |> print
a, b, c |> f |> g, h |> k
```
