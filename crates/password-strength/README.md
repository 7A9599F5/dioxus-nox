# dioxus-nox-password-strength

Pure-logic password strength assessment with optional Dioxus hook.

## Features

- `assess_password_strength` — pure function, zero Dioxus dependency
- Custom check functions for extensibility
- Default checks: length 8+, length 12+, uppercase, number, special char
- Optional `dioxus` feature for reactive `use_password_strength` hook

## Usage (no Dioxus)

```rust
use dioxus_nox_password_strength::*;

let result = assess_password_strength_default("MyP@ss1");
println!("Score: {}/4 ({})", result.score, result.label);
```
