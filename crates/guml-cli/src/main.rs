//! The `guml` CLI.
//!
//! `--format json` exists for the LLM repair loop, not for humans: it emits the full
//! diagnostic set with spans and suggestions so a harness can patch without another model
//! call (report §6.7).

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use guml_compiler::{Options, approx_tokens, check, compile};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "guml",
    version,
    about = "Generative UI Markup Language compiler",
    long_about = "GUML compiles a compact declarative UI representation into framework source code.\n\
                  Designed so an LLM emits ~5-8x fewer tokens than hand-written React."
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Parse and validate without emitting code.
    Check {
        file: PathBuf,
        #[arg(long, value_enum, default_value_t = Format::Human)]
        format: Format,
    },
    /// Compile to a target framework.
    Build {
        file: PathBuf,
        #[arg(short, long, default_value = "react")]
        backend: String,
        /// Write files here instead of stdout.
        #[arg(short, long)]
        out: Option<PathBuf>,
        #[arg(long, value_enum, default_value_t = Format::Human)]
        format: Format,
    },
    /// Dump the AST (JSON), for tooling and for inter-run consistency measurement.
    Ast { file: PathBuf },
    /// Dump the token stream, for debugging the lexer.
    Lex { file: PathBuf },
    /// Approximate token counts. Estimates only — see `guml tokens --help`.
    Tokens {
        #[arg(required = true)]
        files: Vec<PathBuf>,
    },
    /// Print the component registry, optionally as an LLM prompt block.
    Registry {
        /// Emit only these tags (the retrieval-augmented prompt path).
        #[arg(long, value_delimiter = ',')]
        tags: Option<Vec<String>>,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum Format {
    Human,
    Json,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Check { file, format } => cmd_check(&file, format),
        Cmd::Build { file, backend, out, format } => {
            cmd_build(&file, &backend, out.as_deref(), format)
        }
        Cmd::Ast { file } => cmd_ast(&file),
        Cmd::Lex { file } => cmd_lex(&file),
        Cmd::Tokens { files } => cmd_tokens(&files),
        Cmd::Registry { tags } => cmd_registry(tags),
    }
}

fn read(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))
}

fn cmd_check(path: &Path, format: Format) -> Result<()> {
    let src = read(path)?;
    let (_, diags) = check(&src);
    report(&diags, &src, path, format);
    if diags.has_errors() {
        std::process::exit(1);
    }
    if format == Format::Human {
        println!("ok: {} ({} warnings)", path.display(), diags.len());
    }
    Ok(())
}

fn cmd_build(path: &Path, backend: &str, out: Option<&Path>, format: Format) -> Result<()> {
    if guml_compiler::resolve_backend(backend).is_none() {
        anyhow::bail!(
            "unknown backend `{backend}` (available: {})",
            guml_compiler::backend_names().join(", ")
        );
    }
    let src = read(path)?;
    let res = compile(&src, &Options { backend: backend.to_string() });
    report(&res.diagnostics, &src, path, format);

    if res.diagnostics.has_errors() {
        std::process::exit(1);
    }

    for f in &res.files {
        match out {
            Some(dir) => {
                std::fs::create_dir_all(dir)?;
                let target = dir.join(&f.path);
                std::fs::write(&target, &f.contents)
                    .with_context(|| format!("writing {}", target.display()))?;
                if format == Format::Human {
                    println!("wrote {}", target.display());
                }
            }
            None => print!("{}", f.contents),
        }
    }

    if format == Format::Human && out.is_some() {
        let s = res.stats;
        println!(
            "\nsource ~{} tokens -> emitted ~{} tokens ({:.1}x expansion, estimates only)",
            s.approx_source_tokens,
            s.approx_emitted_tokens,
            s.ratio()
        );
    }
    Ok(())
}

fn cmd_ast(path: &Path) -> Result<()> {
    let src = read(path)?;
    let (program, diags) = check(&src);
    if diags.has_errors() {
        eprint!("{}", diags.render(&src, &path.display().to_string()));
        std::process::exit(1);
    }
    println!("{}", serde_json::to_string_pretty(&program)?);
    Ok(())
}

fn cmd_lex(path: &Path) -> Result<()> {
    let src = read(path)?;
    let lexed = guml_syntax::lex(&src);
    for line in &lexed.lines {
        println!(
            "{:>3} indent={:<2} {:?}",
            line.line_no,
            line.indent,
            line.tokens.iter().map(|t| &t.tok).collect::<Vec<_>>()
        );
    }
    if !lexed.diagnostics.is_empty() {
        eprint!("{}", lexed.diagnostics.render(&src, &path.display().to_string()));
    }
    Ok(())
}

fn cmd_tokens(files: &[PathBuf]) -> Result<()> {
    println!("{:<28} {:>8} {:>7} {:>12}", "file", "bytes", "lines", "~tokens");
    let mut total = 0usize;
    for f in files {
        let src = read(f)?;
        let t = approx_tokens(&src);
        total += t;
        let name = f.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        println!("{:<28} {:>8} {:>7} {:>12}", name, src.len(), src.lines().count(), t);
    }
    println!("{:<28} {:>8} {:>7} {:>12}", "TOTAL", "", "", total);
    println!(
        "\nnote: ~3.6 chars/token heuristic. For anything that goes in a paper or a README, \
         count with the target model's own tokenizer (see spec/PHASE0.md)."
    );
    Ok(())
}

fn cmd_registry(tags: Option<Vec<String>>) -> Result<()> {
    let reg = guml_registry::Registry::builtin();
    match tags {
        Some(t) => {
            let refs: Vec<&str> = t.iter().map(String::as_str).collect();
            print!("{}", reg.prompt_context(&refs));
        }
        None => {
            for name in reg.names() {
                let c = reg.get(name).unwrap();
                println!("{:<10} {:<10?} {}", c.name, c.kind, c.doc);
            }
            println!("\nmodifiers: {}", guml_registry::MODIFIERS.join(" "));
        }
    }
    Ok(())
}

fn report(diags: &guml_diagnostics::Diagnostics, src: &str, path: &Path, format: Format) {
    if diags.is_empty() {
        return;
    }
    match format {
        Format::Human => eprint!("{}", diags.render(src, &path.display().to_string())),
        Format::Json => println!("{}", diags.to_json()),
    }
}
