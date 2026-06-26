# Tasks
- [ ] Task 1: Add file-based imports and module caching.
  - [ ] Define source-file resolution, module cache keys, export behavior, and cycle handling.
  - [ ] Implement import parsing, compilation, and runtime loading for CLI and library execution.
  - [ ] Validate with focused integration tests that import sibling modules more than once.

- [ ] Task 2: Preserve runtime source positions and stack traces.
  - [ ] Thread source spans from AST nodes into emitted bytecode or instruction metadata.
  - [ ] Surface file/line information and call stacks in VM runtime errors and thrown exceptions.
  - [ ] Validate with failing execution tests and one manual CLI check.

- [ ] Task 3: Replace required explicit closure capture with automatic inference.
  - [ ] Implement free-variable analysis for locals, upvalues, and globals.
  - [ ] Preserve current mutable capture behavior without requiring manual `[captures]`.
  - [ ] Validate with nested and mutable closure tests.

- [ ] Task 4: Extend pipe lowering for bare and multi-argument calls.
  - [ ] Support `lhs |> f` as `f(lhs)`.
  - [ ] Support `lhs |> f(a, b)` as `f(lhs, a, b)`.
  - [ ] Validate placeholder interactions and existing pipe behavior remain intact.

- [ ] Task 5: Reuse compiled state in the REPL.
  - [ ] Preserve globals, functions, and imported modules across sequential submissions.
  - [ ] Avoid recompiling unchanged imported modules during one REPL session.
  - [ ] Validate with sequential REPL scenarios that reuse prior definitions.

- [ ] Task 6: Protect private workspace metadata from accidental commits.
  - [ ] Add `.trae/` to `.gitignore`.
  - [ ] Verify spec files are ignored by default after the change.

- [ ] Task 7: Verify and stabilize the combined change.
  - [ ] Run focused parser, compiler, VM, and REPL tests for the new behavior.
  - [ ] Run the full `cargo test` suite.
  - [ ] Update `checklist.md` with the verification results.

# Task Dependencies
- Task 5 depends on Task 1.
- Task 7 depends on Tasks 1-6.
