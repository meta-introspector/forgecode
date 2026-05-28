# streamdown-config

Configuration loading and management for the [streamdown](https://crates.io/crates/streamdown) streaming markdown renderer.

## Overview

This crate handles configuration:

- **Config file loading** - TOML configuration from standard paths
- **Style computation** - HSV-based color theme generation
- **Default values** - Sensible defaults for all settings
- **Runtime configuration** - Programmatic configuration API

## Configuration Paths

Configuration is loaded from (in order of precedence):
1. `$XDG_CONFIG_HOME/streamdown/config.toml`
2. `~/.config/streamdown/config.toml`
3. `~/.streamdown.toml`

## Example Configuration

```toml
[style]
hue = 0.6        # Base hue (0.0-1.0)
margin = 2       # Left margin

[features]
clipboard = true   # OSC 52 clipboard
savebrace = true   # Save code blocks to temp files
```

## Usage

```toml
[dependencies]
streamdown-config = "0.1"
```

```rust
use streamdown_config::Config;

let config = Config::load()?;
println!("Margin: {}", config.style.margin);
```

## Part of Streamdown

This is a component of [streamdown-rs](https://github.com/fed-stew/streamdown-rs), a streaming markdown renderer for modern terminals.

## License

MIT
