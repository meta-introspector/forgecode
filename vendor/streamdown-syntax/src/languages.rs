//! Language alias mapping.
//!
//! Maps common language aliases to syntect syntax names.
//! This handles cases like "py" → "Python", "js" → "JavaScript", etc.

use std::collections::HashMap;
use std::sync::LazyLock;

/// Static mapping of language aliases.
///
/// Each entry maps a lowercase alias to the canonical syntect syntax name.
pub static LANGUAGE_ALIASES: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();

    // Python
    m.insert("python", "Python");
    m.insert("py", "Python");
    m.insert("python3", "Python");
    m.insert("py3", "Python");

    // JavaScript
    m.insert("javascript", "JavaScript");
    m.insert("js", "JavaScript");
    m.insert("node", "JavaScript");
    m.insert("nodejs", "JavaScript");

    // TypeScript
    m.insert("typescript", "TypeScript");
    m.insert("ts", "TypeScript");

    // Rust
    m.insert("rust", "Rust");
    m.insert("rs", "Rust");

    // Shell/Bash
    m.insert("bash", "Bourne Again Shell (bash)");
    m.insert("sh", "Bourne Again Shell (bash)");
    m.insert("shell", "Bourne Again Shell (bash)");
    m.insert("zsh", "Bourne Again Shell (bash)");
    m.insert("fish", "Bourne Again Shell (bash)");

    // C
    m.insert("c", "C");
    m.insert("h", "C");

    // C++
    m.insert("cpp", "C++");
    m.insert("c++", "C++");
    m.insert("cxx", "C++");
    m.insert("hpp", "C++");
    m.insert("hxx", "C++");

    // C#
    m.insert("csharp", "C#");
    m.insert("cs", "C#");

    // Go
    m.insert("go", "Go");
    m.insert("golang", "Go");

    // Java
    m.insert("java", "Java");

    // Kotlin
    m.insert("kotlin", "Kotlin");
    m.insert("kt", "Kotlin");

    // Swift
    m.insert("swift", "Swift");

    // Ruby
    m.insert("ruby", "Ruby");
    m.insert("rb", "Ruby");

    // PHP
    m.insert("php", "PHP");

    // Perl
    m.insert("perl", "Perl");
    m.insert("pl", "Perl");

    // Lua
    m.insert("lua", "Lua");

    // R
    m.insert("r", "R");

    // Scala
    m.insert("scala", "Scala");

    // Haskell
    m.insert("haskell", "Haskell");
    m.insert("hs", "Haskell");

    // OCaml
    m.insert("ocaml", "OCaml");
    m.insert("ml", "OCaml");

    // F#
    m.insert("fsharp", "F#");
    m.insert("fs", "F#");

    // Elixir
    m.insert("elixir", "Elixir");
    m.insert("ex", "Elixir");
    m.insert("exs", "Elixir");

    // Erlang
    m.insert("erlang", "Erlang");
    m.insert("erl", "Erlang");

    // Clojure
    m.insert("clojure", "Clojure");
    m.insert("clj", "Clojure");

    // SQL
    m.insert("sql", "SQL");
    m.insert("mysql", "SQL");
    m.insert("postgresql", "SQL");
    m.insert("postgres", "SQL");
    m.insert("sqlite", "SQL");

    // HTML
    m.insert("html", "HTML");
    m.insert("htm", "HTML");
    m.insert("xhtml", "HTML");

    // CSS
    m.insert("css", "CSS");
    m.insert("scss", "SCSS");
    m.insert("sass", "Sass");
    m.insert("less", "Less");

    // JSON
    m.insert("json", "JSON");
    m.insert("jsonc", "JSON");

    // YAML
    m.insert("yaml", "YAML");
    m.insert("yml", "YAML");

    // TOML
    m.insert("toml", "TOML");

    // XML
    m.insert("xml", "XML");
    m.insert("xsl", "XML");
    m.insert("xslt", "XML");
    m.insert("svg", "XML");

    // Markdown
    m.insert("markdown", "Markdown");
    m.insert("md", "Markdown");
    m.insert("mdown", "Markdown");

    // LaTeX
    m.insert("latex", "LaTeX");
    m.insert("tex", "TeX");

    // Makefile
    m.insert("makefile", "Makefile");
    m.insert("make", "Makefile");
    m.insert("mk", "Makefile");

    // Docker
    m.insert("dockerfile", "Dockerfile");
    m.insert("docker", "Dockerfile");

    // Nginx
    m.insert("nginx", "nginx");

    // INI
    m.insert("ini", "INI");
    m.insert("conf", "INI");
    m.insert("cfg", "INI");

    // Diff
    m.insert("diff", "Diff");
    m.insert("patch", "Diff");

    // Git
    m.insert("git", "Git Commit");
    m.insert("gitcommit", "Git Commit");
    m.insert("gitignore", "Git Ignore");

    // Lisp
    m.insert("lisp", "Lisp");
    m.insert("elisp", "Lisp");
    m.insert("emacs-lisp", "Lisp");
    m.insert("commonlisp", "Lisp");
    m.insert("cl", "Lisp");

    // Scheme
    m.insert("scheme", "Scheme");
    m.insert("racket", "Scheme");

    // Dart
    m.insert("dart", "Dart");

    // Vue
    m.insert("vue", "Vue Component");

    // GraphQL
    m.insert("graphql", "GraphQL");
    m.insert("gql", "GraphQL");

    // Protocol Buffers
    m.insert("protobuf", "Protocol Buffers");
    m.insert("proto", "Protocol Buffers");

    // Terraform
    m.insert("terraform", "Terraform");
    m.insert("tf", "Terraform");
    m.insert("hcl", "Terraform");

    // Assembly
    m.insert("asm", "Assembly (x86_64)");
    m.insert("assembly", "Assembly (x86_64)");
    m.insert("nasm", "Assembly (x86_64)");

    // Plain text
    m.insert("text", "Plain Text");
    m.insert("txt", "Plain Text");
    m.insert("plain", "Plain Text");

    // Objective-C
    m.insert("objc", "Objective-C");
    m.insert("objective-c", "Objective-C");
    m.insert("objectivec", "Objective-C");

    // Objective-C++
    m.insert("objcpp", "Objective-C++");
    m.insert("objective-c++", "Objective-C++");

    // Pascal/Delphi
    m.insert("pascal", "Pascal");
    m.insert("delphi", "Pascal");

    // Groovy
    m.insert("groovy", "Groovy");

    // PowerShell
    m.insert("powershell", "PowerShell");
    m.insert("ps1", "PowerShell");
    m.insert("pwsh", "PowerShell");

    // Batch/CMD
    m.insert("batch", "Batch File");
    m.insert("bat", "Batch File");
    m.insert("cmd", "Batch File");

    // Regular Expressions
    m.insert("regex", "Regular Expression");
    m.insert("regexp", "Regular Expression");

    // AppleScript
    m.insert("applescript", "AppleScript");

    // JSX/TSX
    m.insert("jsx", "JavaScript (Babel)");
    m.insert("tsx", "TypeScript");

    // CoffeeScript
    m.insert("coffeescript", "CoffeeScript");
    m.insert("coffee", "CoffeeScript");

    // D
    m.insert("d", "D");
    m.insert("dlang", "D");

    // Nim
    m.insert("nim", "Nim");
    m.insert("nimrod", "Nim");

    // Zig
    m.insert("zig", "Zig");

    // Crystal
    m.insert("crystal", "Crystal");
    m.insert("cr", "Crystal");

    // Julia
    m.insert("julia", "Julia");
    m.insert("jl", "Julia");

    // Solidity
    m.insert("solidity", "Solidity");
    m.insert("sol", "Solidity");

    // Vyper
    m.insert("vyper", "Vyper");
    m.insert("vy", "Vyper");

    // Fortran
    m.insert("fortran", "Fortran (Modern)");
    m.insert("f90", "Fortran (Modern)");
    m.insert("f95", "Fortran (Modern)");
    m.insert("f03", "Fortran (Modern)");

    // COBOL
    m.insert("cobol", "COBOL");
    m.insert("cob", "COBOL");

    // ActionScript
    m.insert("actionscript", "ActionScript");
    m.insert("as", "ActionScript");

    // Handlebars
    m.insert("handlebars", "Handlebars");
    m.insert("hbs", "Handlebars");
    m.insert("mustache", "Handlebars");

    // Jinja
    m.insert("jinja", "Jinja");
    m.insert("jinja2", "Jinja");

    // Puppet
    m.insert("puppet", "Puppet");
    m.insert("pp", "Puppet");

    // ReStructuredText
    m.insert("rst", "reStructuredText");
    m.insert("restructuredtext", "reStructuredText");
    m.insert("rest", "reStructuredText");

    // AsciiDoc
    m.insert("asciidoc", "AsciiDoc");
    m.insert("adoc", "AsciiDoc");

    // Org Mode
    m.insert("org", "orgmode");
    m.insert("orgmode", "orgmode");

    m
});

/// Look up the canonical syntax name for a language alias.
///
/// Returns the canonical name if found, or the original input if not.
///
/// # Example
/// ```
/// use streamdown_syntax::language_alias;
///
/// assert_eq!(language_alias("py"), "Python");
/// assert_eq!(language_alias("js"), "JavaScript");
/// assert_eq!(language_alias("rust"), "Rust");
/// assert_eq!(language_alias("unknown"), "unknown"); // Returns original
/// ```
pub fn language_alias(name: &str) -> &str {
    let lower = name.to_lowercase();
    LANGUAGE_ALIASES
        .get(lower.as_str())
        .copied()
        .unwrap_or(name)
}

/// Get all known language aliases.
///
/// Returns an iterator over (alias, canonical_name) pairs.
pub fn all_aliases() -> impl Iterator<Item = (&'static str, &'static str)> {
    LANGUAGE_ALIASES.iter().map(|(k, v)| (*k, *v))
}

/// Get all aliases that map to a specific syntax name.
///
/// # Example
/// ```
/// use streamdown_syntax::aliases_for;
///
/// let python_aliases = aliases_for("Python");
/// assert!(python_aliases.contains(&"py"));
/// assert!(python_aliases.contains(&"python"));
/// ```
pub fn aliases_for(syntax_name: &str) -> Vec<&'static str> {
    LANGUAGE_ALIASES
        .iter()
        .filter_map(|(alias, name)| {
            if *name == syntax_name {
                Some(*alias)
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_aliases() {
        assert_eq!(language_alias("python"), "Python");
        assert_eq!(language_alias("py"), "Python");
        assert_eq!(language_alias("Python"), "Python"); // case insensitive
        assert_eq!(language_alias("PY"), "Python");
    }

    #[test]
    fn test_javascript_aliases() {
        assert_eq!(language_alias("javascript"), "JavaScript");
        assert_eq!(language_alias("js"), "JavaScript");
        assert_eq!(language_alias("node"), "JavaScript");
    }

    #[test]
    fn test_rust_aliases() {
        assert_eq!(language_alias("rust"), "Rust");
        assert_eq!(language_alias("rs"), "Rust");
    }

    #[test]
    fn test_shell_aliases() {
        let expected = "Bourne Again Shell (bash)";
        assert_eq!(language_alias("bash"), expected);
        assert_eq!(language_alias("sh"), expected);
        assert_eq!(language_alias("shell"), expected);
        assert_eq!(language_alias("zsh"), expected);
    }

    #[test]
    fn test_unknown_returns_original() {
        assert_eq!(language_alias("unknown-lang"), "unknown-lang");
        assert_eq!(language_alias("foo"), "foo");
    }

    #[test]
    fn test_aliases_for() {
        let python_aliases = aliases_for("Python");
        assert!(python_aliases.contains(&"py"));
        assert!(python_aliases.contains(&"python"));
        assert!(python_aliases.contains(&"python3"));
    }

    #[test]
    fn test_case_insensitivity() {
        assert_eq!(language_alias("PYTHON"), "Python");
        assert_eq!(language_alias("JavaScript"), "JavaScript");
        assert_eq!(language_alias("RUST"), "Rust");
    }

    #[test]
    fn test_all_aliases_not_empty() {
        let count = all_aliases().count();
        assert!(count > 100); // We have lots of aliases
    }
}
