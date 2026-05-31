# streamdown-parser

Streaming markdown parser for the [streamdown](https://crates.io/crates/streamdown) terminal renderer.

## Overview

A line-oriented streaming markdown parser designed for real-time rendering:

- **Streaming design** - Parses line-by-line as content arrives
- **LLM-friendly** - Handles partial/incomplete markdown gracefully
- **Full CommonMark support** - Headings, code blocks, lists, tables, etc.
- **Think blocks** - Special `<think>` tag support for LLM output

## Usage

```toml
[dependencies]
streamdown-parser = "0.1"
```

```rust
use streamdown_parser::Parser;

let mut parser = Parser::new();

for line in markdown.lines() {
    for event in parser.parse_line(line) {
        // Handle parsing events
        match event {
            Event::Heading { level, text } => { /* ... */ }
            Event::CodeBlock { language, code } => { /* ... */ }
            // ...
        }
    }
}
```

## Features

- Headings (ATX style)
- Fenced code blocks with language detection
- Ordered and unordered lists (nested)
- Tables with alignment
- Blockquotes
- Inline formatting (bold, italic, code, links)
- Think blocks for LLM reasoning

## Part of Streamdown

This is a component of [streamdown-rs](https://github.com/fed-stew/streamdown-rs), a streaming markdown renderer for modern terminals.

## License

MIT
