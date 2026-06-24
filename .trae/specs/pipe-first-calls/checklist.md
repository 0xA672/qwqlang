# Checklist

## Parser Changes
- [x] Parser no longer accepts `f(args)` syntax
- [x] `x, y, z |> func` parses correctly
- [x] Single argument pipe `x |> f` still works
- [x] Chain pipe `a |> b |> c` still works
- [x] Method access `obj.field` still works
- [x] Function definition `fn foo() {}` still works
- [x] Nullary pipe `|> f` works for zero-argument functions

## Compiler Changes
- [x] Multi-argument pipe compiles to correct bytecode
- [x] Stack layout correct for multi-argument calls
- [x] All existing operations (arithmetic, etc.) still work

## Builtin Functions
- [x] `print` works: `"hello" |> print`
- [x] `len` works: `arr |> len`
- [x] `push` works: `arr, val |> push`
- [x] `pop` works: `arr |> pop`

## Integration Tests
- [x] All tests updated to pipe syntax
- [x] All tests pass
- [x] No syntax errors in test files

## Backward Compatibility
- [x] Control flow unchanged: `while`, `for`, `loop`, `if`
- [x] Object/array literals unchanged
- [x] Function definition unchanged
