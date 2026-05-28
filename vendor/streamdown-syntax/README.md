# streamdown-syntax

Syntax highlighting for the [streamdown](https://crates.io/crates/streamdown) streaming markdown renderer.

## Overview

Provides syntax highlighting via [syntect](https://github.com/trishume/syntect):

- **Language detection** - Automatic language identification
- **Theme support** - Customizable color themes
- **ANSI output** - Terminal-ready highlighted output
- **Wide language support** - 100+ programming languages

## Usage

```toml
[dependencies]
streamdown-syntax = "0.1"
```

```rust
use streamdown_syntax::Highlighter;

let highlighter = Highlighter::new();
let highlighted = highlighter.highlight("fn main() {}", "rust");
println!("{}", highlighted);
```

## Supported Languages

All languages supported by syntect, including:
- Rust, Python, JavaScript, TypeScript
- Go, C, C++, Java, Kotlin
- Ruby, PHP, Swift, Scala
- HTML, CSS, JSON, YAML, TOML
- Bash, SQL, Markdown, and many more

## Part of Streamdown

This is a component of [streamdown-rs](https://github.com/fed-stew/streamdown-rs), a streaming markdown renderer for modern terminals.

## License

MIT
