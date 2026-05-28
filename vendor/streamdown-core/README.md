# streamdown-core

Core types, traits, and error definitions for the [streamdown](https://crates.io/crates/streamdown) streaming markdown renderer.

## Overview

This crate provides the foundational types used across all streamdown crates:

- **Event types** - Markdown parsing events (headings, code blocks, lists, etc.)
- **State management** - Parser and renderer state tracking
- **Error types** - Unified error handling across the library
- **Core traits** - Common interfaces for extensibility

## Usage

```toml
[dependencies]
streamdown-core = "0.1"
```

This crate is primarily intended for internal use by other streamdown crates. For most use cases, depend on the main `streamdown` crate instead.

## Part of Streamdown

This is a component of [streamdown-rs](https://github.com/fed-stew/streamdown-rs), a streaming markdown renderer for modern terminals.

## License

MIT
