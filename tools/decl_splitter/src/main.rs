use anyhow::{Context, Result};
use clap::Parser;
use proc_macro2::{Ident, Span, TokenStream};
use quote::ToTokens;
use std::fs;
use std::path::{Path, PathBuf};
use syn::visit::Visit;

// ── Sub-modules (inlined for standalone binary) ──────────────────────────

mod use_collector {
    use syn::visit::{self, Visit};
    use syn::ItemUse;

    /// Visitor to collect all top-level `use` statements.
    #[derive(Default)]
    pub struct UseStatementCollector {
        pub uses: Vec<ItemUse>,
    }

    impl<'ast> Visit<'ast> for UseStatementCollector {
        fn visit_item_use(&mut self, i: &'ast ItemUse) {
            self.uses.push(i.clone());
            visit::visit_item_use(self, i);
        }
    }
}

mod declaration_extractor {
    use quote::ToTokens;
    use syn::{self, Item};

    use crate::ExtractedDecl;

    /// Extracts a single declaration from a `syn::Item`.
    pub fn extract_single_declaration(item: &Item, item_count: usize) -> Option<ExtractedDecl> {
        match item {
            Item::Fn(item_fn) => Some(ExtractedDecl {
                name: item_fn.sig.ident.to_string(),
                kind: "fn".to_string(),
                content: item_fn.to_token_stream(),
            }),
            Item::Struct(item_struct) => Some(ExtractedDecl {
                name: item_struct.ident.to_string(),
                kind: "struct".to_string(),
                content: item_struct.to_token_stream(),
            }),
            Item::Enum(item_enum) => Some(ExtractedDecl {
                name: item_enum.ident.to_string(),
                kind: "enum".to_string(),
                content: item_enum.to_token_stream(),
            }),
            Item::Const(item_const) => Some(ExtractedDecl {
                name: item_const.ident.to_string(),
                kind: "const".to_string(),
                content: item_const.to_token_stream(),
            }),
            Item::Static(item_static) => Some(ExtractedDecl {
                name: item_static.ident.to_string(),
                kind: "static".to_string(),
                content: item_static.to_token_stream(),
            }),
            Item::Trait(item_trait) => Some(ExtractedDecl {
                name: item_trait.ident.to_string(),
                kind: "trait".to_string(),
                content: item_trait.to_token_stream(),
            }),
            Item::Impl(item_impl) => {
                let name = if let Some((_, path, _)) = &item_impl.trait_ {
                    let raw = path.to_token_stream().to_string();
                    // Sanitize: replace :: with _, remove non-ident chars
                    let sanitized: String = raw
                        .chars()
                        .map(|c| {
                            if c.is_alphanumeric() || c == '_' {
                                c
                            } else {
                                '_'
                            }
                        })
                        .collect::<String>()
                        .split('_')
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<_>>()
                        .join("_");
                    format!("impl_for_{}", sanitized)
                } else if let syn::Type::Path(type_path) = &*item_impl.self_ty {
                    if let Some(segment) = type_path.path.segments.last() {
                        format!("impl_for_{}", segment.ident.to_string())
                    } else {
                        format!("impl_{}", item_count)
                    }
                } else {
                    format!("impl_{}", item_count)
                };
                Some(ExtractedDecl {
                    name,
                    kind: "impl".to_string(),
                    content: item_impl.to_token_stream(),
                })
            }
            Item::Type(item_type) => Some(ExtractedDecl {
                name: item_type.ident.to_string(),
                kind: "type".to_string(),
                content: item_type.to_token_stream(),
            }),
            Item::Union(item_union) => Some(ExtractedDecl {
                name: item_union.ident.to_string(),
                kind: "union".to_string(),
                content: item_union.to_token_stream(),
            }),
            Item::Use(_) => {
                println!("  skip: use statement");
                None
            }
            Item::Mod(item_mod) => {
                println!("  skip: mod {}", item_mod.ident);
                None
            }
            Item::Macro(item_macro) => {
                println!(
                    "  skip: macro {}",
                    item_macro.mac.path.to_token_stream()
                );
                None
            }
            _ => {
                println!("  skip: unsupported item");
                None
            }
        }
    }
}

mod declaration_writer {
    use anyhow::{Context, Result};
    use proc_macro2::{Ident, TokenStream};
    use quote::quote;
    use std::fs;
    use std::path::Path;

    use crate::ExtractedDecl;

    /// Writes a single declaration to its own .rs file.
    pub fn write_declaration_file(
        decl: ExtractedDecl,
        decls_output_dir: &Path,
        common_uses: &TokenStream,
        module_name_ident: Ident,
        dry_run: bool,
    ) -> Result<()> {
        let decl_file_path =
            decls_output_dir.join(format!("{}.rs", module_name_ident.to_string()));

        let decl_token_stream = decl.content;

        let file_content = quote! {
            #common_uses
            #decl_token_stream
        };

        if !dry_run {
            fs::write(&decl_file_path, file_content.to_string())
                .context(format!("Failed to write to {}", decl_file_path.display()))?;
            println!(
                "  split: {} {} -> {}",
                decl.kind,
                decl.name,
                decl_file_path.display()
            );
        } else {
            println!(
                "  dry-run: would split {} {} -> {}",
                decl.kind,
                decl.name,
                decl_file_path.display()
            );
        }
        Ok(())
    }
}

mod invocation_generator {
    use anyhow::{Context, Result};
    use proc_macro2::Ident;
    use proc_macro2::TokenStream;
    use quote::quote;
    use std::fs;
    use std::path::Path;

    /// Generates the `_decl_module_invocation.rs` file that re-exports all split modules.
    pub fn generate_decl_module_invocation(
        collected_module_names: Vec<Ident>,
        decls_output_dir: &Path,
        dry_run: bool,
    ) -> Result<()> {
        let decl_invocation_file_path = decls_output_dir.join("_decl_module_invocation.rs");

        // Generate mod declarations for each split file
        let mod_decls: TokenStream = collected_module_names
            .iter()
            .map(|name| {
                quote! { pub mod #name; }
            })
            .collect();

        if !dry_run {
            fs::write(&decl_invocation_file_path, mod_decls.to_string())
                .context("Failed to write _decl_module_invocation.rs")?;
            println!(
                "  generated: {}",
                decl_invocation_file_path.display()
            );
        } else {
            println!(
                "  dry-run: would generate {}",
                decl_invocation_file_path.display()
            );
        }
        Ok(())
    }
}

// ── Core types ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ExtractedDecl {
    pub name: String,
    pub kind: String,
    pub content: TokenStream,
}

// ── Core splitting logic ─────────────────────────────────────────────────

/// Splits a parsed AST into individual declaration files.
fn split_and_generate_decls(
    syntax_tree: &syn::File,
    crate_name: &str,
    decls_output_dir: &Path,
    dry_run: bool,
) -> Result<()> {
    if !dry_run {
        fs::create_dir_all(decls_output_dir).context(format!(
            "Failed to create directory {}",
            decls_output_dir.display()
        ))?;
        println!("Created directory: {}", decls_output_dir.display());
    } else {
        println!(
            "Dry-run: Would create directory: {}",
            decls_output_dir.display()
        );
    }

    // Collect use statements
    let mut use_collector_instance = use_collector::UseStatementCollector::default();
    use_collector_instance.visit_file(syntax_tree);
    let common_uses: TokenStream = use_collector_instance
        .uses
        .iter()
        .map(|u| u.to_token_stream())
        .collect();

    let mut collected_module_names: Vec<Ident> = Vec::new();
    let mut item_count = 0;

    // Extract and split declarations
    for item in &syntax_tree.items {
        if let Some(decl) =
            declaration_extractor::extract_single_declaration(item, item_count)
        {
            let module_name_str =
                format!("{}_{}", crate_name.replace("-", "_"), decl.name);
            let module_name_ident = Ident::new(&module_name_str, Span::call_site());
            collected_module_names.push(module_name_ident.clone());

            declaration_writer::write_declaration_file(
                decl,
                decls_output_dir,
                &common_uses,
                module_name_ident,
                dry_run,
            )?;
        }
        item_count += 1;
    }

    // Generate module invocation file
    invocation_generator::generate_decl_module_invocation(
        collected_module_names,
        decls_output_dir,
        dry_run,
    )?;

    Ok(())
}

// ── CLI ──────────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(name = "decl-splitter", about = "Split Rust declarations into individual files")]
struct Args {
    /// Path to the Rust source file to split
    #[arg(short, long)]
    input: PathBuf,

    /// Output directory for split declaration files (default: <input_dir>/decls/<stem>)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Crate name prefix for generated module names (default: derived from input path)
    #[arg(short, long)]
    crate_name: Option<String>,

    /// Dry run: show what would be done without writing files
    #[arg(long)]
    dry_run: bool,
}

fn derive_crate_name(input: &Path) -> Result<String> {
    // Walk up from the .rs file to find the Cargo.toml
    let mut dir = input.parent();
    while let Some(d) = dir {
        if d.join("Cargo.toml").exists() {
            let cargo_toml_content = fs::read_to_string(d.join("Cargo.toml"))?;
            if let Ok(value) = cargo_toml_content.parse::<toml::Value>() {
                if let Some(name) = value.get("package").and_then(|p| p.get("name")) {
                    if let Some(name_str) = name.as_str() {
                        return Ok(name_str.to_string());
                    }
                }
            }
        }
        dir = d.parent();
    }
    // Fallback: use the parent directory name
    input
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .context(format!(
            "Cannot derive crate name from path {}",
            input.display()
        ))
}

fn main() -> Result<()> {
    let args = Args::parse();

    let input_path = &args.input;
    anyhow::ensure!(
        input_path.exists(),
        "Input file does not exist: {}",
        input_path.display()
    );
    anyhow::ensure!(
        input_path.extension().map_or(false, |e| e == "rs"),
        "Input file must have .rs extension: {}",
        input_path.display()
    );

    let crate_name = match &args.crate_name {
        Some(name) => name.clone(),
        None => derive_crate_name(input_path)?,
    };

    let output_dir = match &args.output {
        Some(dir) => dir.clone(),
        None => {
            // Default: <input_parent>/decls/<input_stem>
            let stem = input_path
                .file_stem()
                .and_then(|s| s.to_str())
                .context("Input file has no stem")?;
            input_path
                .parent()
                .unwrap_or(Path::new("."))
                .join("decls")
                .join(stem)
        }
    };

    println!("decl-splitter: splitting {}", input_path.display());
    println!("  crate_name:  {}", crate_name);
    println!("  output_dir:  {}", output_dir.display());
    println!("  dry_run:     {}", args.dry_run);
    println!();

    // Parse the input file
    let source = fs::read_to_string(input_path)
        .context(format!("Failed to read {}", input_path.display()))?;

    let syntax_tree: syn::File = syn::parse_file(&source)
        .context(format!("Failed to parse {}", input_path.display()))?;

    // Perform the split
    split_and_generate_decls(&syntax_tree, &crate_name, &output_dir, args.dry_run)?;

    println!("\ndone.");
    Ok(())
}
