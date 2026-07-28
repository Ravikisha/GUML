//! The `guml` CLI.
//!
//! `--format json` exists for the LLM repair loop, not for humans: it emits the full
//! diagnostic set with spans and suggestions so a harness can patch without another model
//! call (report §6.7).

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use guml_compiler::{Options, approx_tokens, check, compile};
use std::io::Read as _;
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
    /// Format source. Reads stdin when no file is given, which is what editors want.
    Fmt {
        /// Files to format. Omit to read stdin and write stdout.
        files: Vec<PathBuf>,
        /// Rewrite the files in place instead of printing.
        #[arg(short, long)]
        write: bool,
        /// Exit 1 if any file is not already formatted. Prints nothing on success.
        #[arg(long)]
        check: bool,
        /// Strip every discretionary byte: comments, blank lines, declaration order.
        /// Semantically identical documents become byte-identical, which is what dedup
        /// and inter-run consistency measurement need.
        #[arg(long)]
        canonical: bool,
    },
    /// Validate one or more documents. Same analysis as `check`, built for batches.
    Validate {
        /// Files or directories. A directory is searched for `*.guml`.
        #[arg(required = true)]
        paths: Vec<PathBuf>,
        /// Treat warnings as failures. For CI, and for scoring generated output.
        #[arg(long)]
        strict: bool,
        #[arg(long, value_enum, default_value_t = Format::Human)]
        format: Format,
    },
    /// Classify every byte for syntax highlighting, using the real lexer and registry.
    Highlight {
        file: PathBuf,
        #[arg(long, value_enum, default_value_t = Format::Json)]
        format: Format,
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
        Cmd::Fmt { files, write, check, canonical } => cmd_fmt(&files, write, check, canonical),
        Cmd::Validate { paths, strict, format } => cmd_validate(&paths, strict, format),
        Cmd::Highlight { file, format } => cmd_highlight(&file, format),
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

fn cmd_fmt(files: &[PathBuf], write: bool, check_only: bool, canonical: bool) -> Result<()> {
    let opts = guml_fmt::Options { canonical };

    // No files means stdin -> stdout: the shape every editor's format-on-save expects.
    if files.is_empty() {
        let mut src = String::new();
        std::io::stdin().read_to_string(&mut src).context("reading stdin")?;
        let out = guml_fmt::format(&src, opts);
        if check_only {
            if out.changed {
                std::process::exit(1);
            }
            return Ok(());
        }
        print!("{}", out.text);
        return Ok(());
    }

    let mut unformatted = Vec::new();
    for path in files {
        let src = read(path)?;
        let out = guml_fmt::format(&src, opts);
        if check_only {
            if out.changed {
                unformatted.push(path.display().to_string());
            }
            continue;
        }
        if write {
            if out.changed {
                std::fs::write(path, &out.text)
                    .with_context(|| format!("writing {}", path.display()))?;
                println!("formatted {}", path.display());
            }
        } else {
            print!("{}", out.text);
        }
    }

    if !unformatted.is_empty() {
        eprintln!("not formatted:");
        for f in &unformatted {
            eprintln!("  {f}");
        }
        eprintln!("
run: guml fmt --write {}", unformatted.join(" "));
        std::process::exit(1);
    }
    Ok(())
}

/// Batch validation.
///
/// `check` takes one file and is what an editor and the repair loop call. This takes many,
/// which is what you need to answer "did the model produce valid documents" over a run of
/// generations — and it exits non-zero on the first problem so CI notices.
fn cmd_validate(paths: &[PathBuf], strict: bool, format: Format) -> Result<()> {
    let mut files: Vec<PathBuf> = Vec::new();
    for p in paths {
        if p.is_dir() {
            let mut found: Vec<PathBuf> = std::fs::read_dir(p)
                .with_context(|| format!("reading {}", p.display()))?
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|e| e.extension().is_some_and(|x| x == "guml"))
                .collect();
            found.sort();
            files.extend(found);
        } else {
            files.push(p.clone());
        }
    }
    if files.is_empty() {
        anyhow::bail!("no .guml files found");
    }

    #[derive(serde::Serialize)]
    struct FileReport {
        file: String,
        ok: bool,
        errors: usize,
        warnings: usize,
        codes: Vec<String>,
    }

    let mut reports = Vec::new();
    let mut failed = 0usize;

    for file in &files {
        let src = read(file)?;
        let (_, diags) = check(&src);
        let errors = diags.error_count();
        let warnings = diags.len() - errors;
        let ok = if strict { diags.is_empty() } else { errors == 0 };
        if !ok {
            failed += 1;
        }

        if format == Format::Human {
            print!("{}", diags.render(&src, &file.display().to_string()));
            println!(
                "{} {} ({errors} errors, {warnings} warnings)",
                if ok { "ok:" } else { "FAIL:" },
                file.display()
            );
        }
        reports.push(FileReport {
            file: file.display().to_string(),
            ok,
            errors,
            warnings,
            codes: diags.items.iter().map(|d| d.id.to_string()).collect(),
        });
    }

    if format == Format::Json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "files": reports,
                "total": files.len(),
                "failed": failed,
                "strict": strict,
            }))?
        );
    } else {
        println!("
{} of {} valid{}", files.len() - failed, files.len(), if strict { " (strict)" } else { "" });
    }

    if failed > 0 {
        std::process::exit(1);
    }
    Ok(())
}

fn cmd_highlight(path: &Path, format: Format) -> Result<()> {
    let src = read(path)?;
    match format {
        Format::Json => println!("{}", guml_fmt::highlight::to_json(&src)),
        // Human output is for eyeballing the classifier, not for piping.
        Format::Human => {
            for span in guml_fmt::highlight::classify(&src) {
                println!(
                    "{:>4}:{:<5} {:<9} {:?}",
                    span.line,
                    span.start,
                    span.class.name(),
                    &src[span.start..span.end]
                );
            }
        }
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

/// The AST is always printed, errors or not.
///
/// The parser recovers, so a document with three bad lines still has a tree — and refusing to
/// show it makes the dump useless exactly when it is most wanted: inspecting what a model
/// actually produced. Diagnostics go to stderr and the exit code still reports failure, so a
/// script can have both.
fn cmd_ast(path: &Path) -> Result<()> {
    let src = read(path)?;
    let (program, diags) = check(&src);
    println!("{}", serde_json::to_string_pretty(&program)?);
    if diags.has_errors() {
        eprint!("{}", diags.render(&src, &path.display().to_string()));
        std::process::exit(1);
    }
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
