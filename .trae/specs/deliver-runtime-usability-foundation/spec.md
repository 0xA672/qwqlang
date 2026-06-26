# Runtime Usability Foundation Spec

## Why
The language already covers most core expression and control-flow features, but larger scripts still run into a few practical gaps: no file-based modules, runtime errors lose precise execution context, closures require manual capture boilerplate, and pipe ergonomics break down for multi-argument calls. This change defines the first engineering slice that makes the language easier to use for real programs without jumping straight to advanced optimization work.

## What Changes
- Add a file-based `import` system with deterministic module resolution and module caching.
- Preserve source spans through compilation so VM runtime errors can report exact file/line locations and stack traces.
- Infer closure captures automatically from free-variable analysis instead of requiring manual capture lists.
- Extend pipe lowering so `lhs |> f` becomes `f(lhs)` and `lhs |> f(a, b)` becomes `f(lhs, a, b)`.
- Reuse compiled globals and imported modules across REPL submissions.
- Ignore `.trae/` in Git defaults so private workspace metadata is not committed by accident.

## Impact
- Affected specs: module loading, runtime diagnostics, closures, pipe expressions, REPL behavior, repository hygiene
- Affected code: `src/lexer.rs`, `src/parser.rs`, `src/ast.rs`, `src/compiler.rs`, `src/vm.rs`, `src/error.rs`, `src/main.rs`, `src/lib.rs`, `tests/integration.rs`, `.gitignore`

## ADDED Requirements
### Requirement: File-Based Imports
The system SHALL load modules from source files, cache loaded modules during a process, and expose imported bindings to the caller.

#### Scenario: Import a sibling module
- **WHEN** a script imports another file from the same project
- **THEN** the imported module is compiled once, evaluated once, and its exported bindings are available to the importing file

#### Scenario: Reuse an already loaded module
- **WHEN** the same module is imported multiple times during one CLI or REPL session
- **THEN** the runtime reuses the cached module result instead of recompiling and reevaluating it

### Requirement: Runtime Source Diagnostics
The system SHALL preserve source positions through bytecode generation and use them in runtime failures and stack traces.

#### Scenario: Report a runtime failure location
- **WHEN** VM execution raises a runtime error
- **THEN** the error output includes the source file and line for the failing instruction

#### Scenario: Report a call stack
- **WHEN** a runtime error crosses one or more function calls
- **THEN** the error output includes the call stack with source locations for each frame

### Requirement: Automatic Closure Capture
The system SHALL infer closure captures from referenced outer variables, including mutable captures.

#### Scenario: Capture an outer immutable binding
- **WHEN** a closure reads an outer local variable
- **THEN** the compiler captures it automatically without requiring a capture list

#### Scenario: Capture an outer mutable binding
- **WHEN** a closure mutates an outer `mut` variable
- **THEN** the compiler captures the mutable upvalue automatically and preserves the current mutation semantics

### Requirement: Multi-Argument Pipe Calls
The system SHALL allow pipe expressions to target bare functions and function calls with additional arguments.

#### Scenario: Pipe into a bare function
- **WHEN** a user writes `value |> f`
- **THEN** the expression evaluates as `f(value)`

#### Scenario: Pipe into a function call with extra arguments
- **WHEN** a user writes `value |> f(a, b)`
- **THEN** the expression evaluates as `f(value, a, b)`

### Requirement: REPL State Reuse
The REPL SHALL preserve compiled globals, functions, and imported modules across sequential submissions in one session.

#### Scenario: Reuse a previous definition
- **WHEN** a user defines a function or global in one REPL input and references it in a later input
- **THEN** the later input resolves and executes against the existing compiled state

## MODIFIED Requirements
### Requirement: Closure Capture Syntax
The system SHALL no longer require explicit capture lists for closures. Existing explicit capture syntax may remain accepted for compatibility, but capture selection must be driven by compiler analysis.

### Requirement: Repository Metadata Safety
The project SHALL ignore `.trae/` by default so specification and workspace metadata stay out of public commits unless a maintainer intentionally overrides that behavior.

## REMOVED Requirements
### Requirement: Mandatory Explicit Capture Lists
**Reason**: Requiring manual capture declarations adds boilerplate and makes closures less ergonomic than the rest of the language.
**Migration**: Existing closures continue to work, but users no longer need to write capture lists for ordinary closure usage.
