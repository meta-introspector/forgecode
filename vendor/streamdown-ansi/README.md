# streamdown-ansi

ANSI escape codes and terminal utilities for the [streamdown](https://crates.io/crates/streamdown) streaming markdown renderer.

## Overview

This crate provides terminal-related utilities:

- **ANSI escape sequences** - Colors, styles, cursor control
- **Unicode width handling** - Proper display width calculation
- **Terminal detection** - Capability detection and fallbacks
- **String utilities** - ANSI-aware string manipulation

## Usage

```toml
[dependencies]
streamdown-ansi = "0.1"
```

```rust
use streamdown_ansi::{AnsiStyle, Color};

let style = AnsiStyle::new()
    .foreground(Color::Red)
    .bold();

println!("{}Error!{}", style.start(), style.end());
```

## Part of Streamdown

This is a component of [streamdown-rs](https://github.com/fed-stew/streamdown-rs), a streaming markdown renderer for modern terminals.

## License

MIT
