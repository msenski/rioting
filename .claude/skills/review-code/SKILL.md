---
description: Review Rust code in this project for bugs, design concerns, and Rust idioms. Use when the user asks for a code review, asks if their code is correct, or finishes implementing something and wants feedback.
argument-hint: [file-path]
allowed-tools: LSP Read
---

# Code Review

Review all changed files. If `$ARGUMENTS` is given, focus there but keep the rest of the diff as context.

## Changed files

!`git diff HEAD --name-only`

## Full diff

!`git diff HEAD`

## Context

This is a Rust 2024 home security system project. The user is learning Rust — comfortable with ownership and basic async, still building intuition for idioms and protocol-level correctness. Reviews should explain *why* something matters, not just flag it.

The project bridges several protocols: RTSP, HLS, HTTP, and ONVIF/SOAP. The ONVIF code in `src/onvif.rs` is the active area of work.

## What to check

### Bugs and correctness
- Logic errors, off-by-one, wrong conditions
- **ONVIF/SOAP specific**: namespace prefixes used without `xmlns:prefix=` declaration, wrong namespace URIs, element names that don't match the ONVIF spec, malformed XML
- **Async specific**: holding a lock across `.await`, blocking calls inside async functions (`std::thread::sleep` instead of `tokio::time::sleep`), missing `.await` on futures
- Error messages that will be useless when debugging on hardware (no context, no values)

### Things that will cause pain later
- API design that makes misuse easy (e.g. easy to call methods in the wrong order, easy to pass the wrong string where a typed value should go)
- Anything that will make multi-camera support harder (hardcoded assumptions about a single camera)
- String-typed values that should be newtypes (profile tokens, service URLs)
- Functions that do too many things and will be hard to test or extend

### Rust idioms and cleanliness
- Redundant borrows (e.g. `&&T` when `&T` suffices)
- `format!` with no interpolation (use a string literal or `String::from`)
- Unnecessary `.clone()` or `.to_string()` calls
- `unwrap()` / `expect()` in non-test code that should be `?` or a real error
- Missing or stale doc comments (especially if a public API changed but the example wasn't updated)
- Clippy-visible issues

### What NOT to flag
- Style preferences with no real tradeoff
- Hypothetical future requirements not in the plan

## Output format

Group findings by severity:

**Bug / will break at runtime** — must fix before continuing  
**Design concern** — won't break now but will cause pain later  
**Rust idiom / cleanliness** — worth fixing, explain why it matters  

For each finding: one sentence on what it is, one sentence on why it matters or what goes wrong if ignored. Keep it tight — the user can ask for more detail on anything.

Repeat any finding from a previous review that has not yet been fixed — unfixed issues stay on the list until resolved.
