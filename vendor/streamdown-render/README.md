# streamdown-render

Terminal rendering engine for the [streamdown](https://crates.io/crates/streamdown) streaming markdown renderer.

## Overview

The core rendering engine that transforms parsed markdown into beautiful terminal output:

- **Box drawing** - Unicode borders for code blocks and tables
- **Syntax highlighting** - Integrated code highlighting
- **Responsive layout** - Adapts to terminal width
- **Streaming output** - Renders incrementally as content arrives
- **Clipboard integration** - OSC 52 support for code blocks

## Usage

```toml
[dependencies]
streamdown-render = "0.1"
```

```rust
use streamdown_parser::Parser;
use streamdown_render::Renderer;

let mut output = Vec::new();
let mut parser = Parser::new();

{
    let mut renderer = Renderer::new(&mut output, 80);

    for line in markdown.lines() {
        for event in parser.parse_line(line) {
            renderer.render_event(&event)?;
        }
    }
}

print!("{}", String::from_utf8(output)?);
```

## Features

- Beautiful headings with proper centering
- Syntax-highlighted code blocks with borders
- Unicode box-drawing tables
- Nested list rendering with bullets
- Blockquote styling
- Think block rendering for LLM output

## Part of Streamdown

This is a component of [streamdown-rs](https://github.com/fed-stew/streamdown-rs), a streaming markdown renderer for modern terminals.

## License

MIT
