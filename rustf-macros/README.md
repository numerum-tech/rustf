# rustf-macros

[![crates.io](https://img.shields.io/crates/v/rustf-macros.svg)](https://crates.io/crates/rustf-macros)
[![docs.rs](https://img.shields.io/docsrs/rustf-macros)](https://docs.rs/rustf-macros)

Procedural macros powering the auto-discovery system of the
[RustF](https://crates.io/crates/rustf) web framework.

These macros scan conventional project directories at build time and generate the
wiring that would otherwise be hand-written in `mod.rs` files — giving zero runtime
overhead.

| Macro | Discovers |
|-------|-----------|
| `auto_controllers!()` | `src/controllers/*.rs` |
| `auto_models!()` | `src/models/*.rs` |
| `auto_middleware!()` | `src/middleware/*.rs` |
| `auto_workers!()` | `src/workers/*.rs` |
| `auto_events!()` | `src/events/*.rs` |
| `auto_definitions!()` | `src/definitions/*.rs` |
| `auto_modules!()` | `src/modules/*.rs` |

You normally don't depend on this crate directly — it is re-exported by `rustf`:

```rust
use rustf::prelude::*; // brings the auto_* macros into scope
```

## License

Licensed under either of Apache License 2.0 or MIT license at your option.
