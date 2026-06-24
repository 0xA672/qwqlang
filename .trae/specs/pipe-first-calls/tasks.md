# Tasks

## Phase 1: Parser Changes

- [ ] Task 1.1: Remove `f(args)` call parsing from parser
  - Remove `Tok::LParen` handling in `call()` function
  - Remove `Tok::RParen` handling
  - Verify `primary()` no longer creates `Expr::Call`

- [ ] Task 1.2: Implement pipe with multiple arguments parsing
  - Modify pipe parsing to accept expression list before `|>`
  - Handle `x, y, z |> func` syntax
  - Store multiple values for pipeline

- [ ] Task 1.3: Update `pos()` function for removed tokens
  - Ensure all Tok variants are handled

## Phase 2: Compiler Changes

- [ ] Task 2.1: Compile pipe with multiple arguments
  - Generate code to evaluate all argument expressions
  - Call function with values on stack
  - Handle single and multiple arguments uniformly

- [ ] Task 2.2: Update find_used_vars for new Expr::Pipe structure
  - Extract variables from expression list

## Phase 3: Builtin Functions

- [ ] Task 3.1: Update builtin registration
  - Ensure all builtins work with new pipe syntax
  - Test `print`, `len`, `push`, `pop` etc.

## Phase 4: Testing

- [ ] Task 4.1: Run existing tests (will fail - breaking change expected)
- [ ] Task 4.2: Update test files to use new pipe syntax
- [ ] Task 4.3: Verify all tests pass with new syntax

## Task Dependencies
- Task 2.1 depends on Task 1.2
- Task 3.1 depends on Task 2.1
- Task 4.2 depends on Task 3.1
