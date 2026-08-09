//! The `guml` CLI.
//!
//! `--format json` exists for the LLM repair loop, not for humans: it emits the full
//! diagnostic set with spans and suggestions so a harness can patch without another model
//! call (report §6.7).

mod mcp;
mod project;

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
        /// Compile at the **core** conformance level: markup only, no `state`, no `data`, no
        /// actions, no `js`. This is the switch a host embedding untrusted documents needs.
        #[arg(long)]
        core: bool,
        /// A JSON registry document adding components to the builtin vocabulary.
        #[arg(long, value_name = "FILE")]
        registry: Option<PathBuf>,
    },
    /// Compile to a target framework.
    Build {
        file: PathBuf,
        /// Also write a Source Map v3 beside each emitted file, and reference it from the
        /// output. Without one, a stack trace points at code the author never wrote.
        #[arg(long)]
        source_map: bool,
        /// Target to emit. `--help` lists them, from the compiler's own registry of backends rather
        /// than a second list here that could drift.
        #[arg(
            short,
            long,
            default_value = "react",
            value_parser = clap::builder::PossibleValuesParser::new(guml_compiler::backend_names())
        )]
        backend: String,
        /// Write files here instead of stdout.
        #[arg(short, long)]
        out: Option<PathBuf>,
        #[arg(long, value_enum, default_value_t = Format::Human)]
        format: Format,
        /// Compile at the **core** conformance level: markup only, no `state`, no `data`, no
        /// actions, no `js`. This is the switch a host embedding untrusted documents needs.
        #[arg(long)]
        core: bool,
        /// A JSON registry document adding components to the builtin vocabulary.
        #[arg(long, value_name = "FILE")]
        registry: Option<PathBuf>,
        /// A JSON theme document replacing the shipped design-system table. The theme must declare a
        /// focus treatment and a contrast floor, or it is refused.
        #[arg(long, value_name = "FILE")]
        theme: Option<PathBuf>,
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
    /// Apply every unambiguous diagnostic suggestion, with no model in the loop.
    Fix {
        #[arg(required = true)]
        files: Vec<PathBuf>,
        /// Rewrite in place instead of printing.
        #[arg(short, long)]
        write: bool,
        /// Cap on re-check rounds; fixing one problem can reveal another.
        #[arg(long, default_value_t = 3)]
        rounds: usize,
    },
    /// Repair model output: strip packaging, format, apply every unambiguous fix. No model call.
    ///
    /// This is `fix` plus the two layers that only existed in the benchmark harness — unwrapping a
    /// ```` ``` ```` fence and dropping trailing commentary. Reads stdin when no file is given, which
    /// is the shape a generation pipeline wants.
    Repair {
        /// Files to repair. Omit to read stdin and write stdout.
        files: Vec<PathBuf>,
        /// Rewrite in place instead of printing.
        #[arg(short, long)]
        write: bool,
        /// Cap on re-check rounds; fixing one problem can reveal another.
        #[arg(long, default_value_t = guml_compiler::repair::DEFAULT_ROUNDS)]
        rounds: usize,
        /// Report what each layer did as JSON, for a harness collecting telemetry.
        #[arg(long, value_enum, default_value_t = Format::Human)]
        format: Format,
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
    /// Explain a diagnostic code: what the rule is, and why the language has it.
    Explain {
        /// `GUML0064`, `0064` or `64`. Omit to list every code.
        code: Option<String>,
    },
    /// Which GUML line produced a line of emitted code, via the source map.
    Where {
        /// The GUML source.
        file: PathBuf,
        /// A line number in the emitted output, 1-based.
        emitted_line: u32,
        #[arg(short, long, default_value = "react")]
        backend: String,
    },
    /// Classify every byte for syntax highlighting, using the real lexer and registry.
    Highlight {
        file: PathBuf,
        #[arg(long, value_enum, default_value_t = Format::Json)]
        format: Format,
    },
    /// Serve the compiler over the Model Context Protocol, on stdio.
    ///
    /// GUML has no training data, so using it has meant ~3,000 tokens of spec in every system prompt
    /// for a language the model has never seen. This removes that: the model asks for the dozen tags
    /// it needs (~180 tokens), checks what it wrote against the compiler that will build it, and gets
    /// the mechanically-fixable errors fixed without spending a turn on them.
    ///
    /// Add to a client's config:
    ///
    ///   { "mcpServers": { "guml": { "command": "guml", "args": ["mcp"] } } }
    Mcp,
    /// Print the component registry, optionally as an LLM prompt block.
    Registry {
        /// Emit only these tags (the retrieval-augmented prompt path).
        #[arg(long, value_delimiter = ',')]
        tags: Option<Vec<String>>,
        /// Pick the tags a task description implies, instead of listing them by hand. This is the
        /// retrieval path: prompt in, prompt-sized vocabulary block out.
        #[arg(long, value_name = "TEXT", conflicts_with = "tags")]
        for_prompt: Option<String>,
        /// Audit a registry package and report every problem at once, without installing it.
        ///
        /// A JSON document, or a directory containing `guml.registry.json` — the same shapes `guml add`
        /// takes, so auditing before installing needs no change of argument.
        #[arg(long, value_name = "PATH")]
        validate: Option<PathBuf>,
        /// Emit reference documentation for the active vocabulary as Markdown.
        ///
        /// Generated rather than written, for the same reason the docs site's vocabulary block is: a
        /// hand-written table drifts from the registry silently, and a component page that lists an
        /// attribute the compiler rejects is worse than no page.
        #[arg(long)]
        docs: bool,
        /// A registry package to include, on top of `guml.json`.
        #[arg(long, value_name = "FILE")]
        registry: Option<PathBuf>,
    },
    /// What a document will actually do: origins, script, escape hatches — and a CSP for it.
    ///
    /// `--core` answers "may an untrusted agent send me this at all", one bit. This answers the question
    /// a host has to act on: *which origins will it contact, does it contain script, does it read
    /// storage.* Those are the terms a Content-Security-Policy is written in, and the compiler already
    /// knows the exact answers.
    Capabilities {
        /// Files or directories. A directory is searched for `*.guml`.
        #[arg(required = true)]
        paths: Vec<PathBuf>,
        #[arg(long, value_enum, default_value_t = Format::Human)]
        format: Format,
        /// Print a Content-Security-Policy for the given backend's output instead of the manifest.
        #[arg(long, value_name = "BACKEND")]
        csp: Option<String>,
        /// Fail when any document exceeds this many `js`/`raw` blocks.
        ///
        /// The escape-hatch rate is the early warning that the vocabulary is hitting an expressiveness
        /// cliff (report §12.1 risk 5), which is only useful if something is counting. This is the number
        /// CI fails on.
        #[arg(long, value_name = "N")]
        max_escapes: Option<usize>,
        /// Exit non-zero unless every document is **inert**: markup only, no script, no network.
        ///
        /// The safe-render gate. A host embedding a document that arrived from an untrusted agent wants
        /// one command and one answer, and `--core` is not quite it: the core *level* says the vocabulary
        /// admits no app constructs, and this says *this particular document* will not run code or talk to
        /// anything. Both are worth having — the level is a compile-time restriction, this is a fact about
        /// the artifact.
        #[arg(long)]
        assert_inert: bool,
    },
    /// Install a registry package into `guml.json`, after auditing it.
    ///
    /// Takes a path, never a URL. A registry decides which tags a document may use and which classes
    /// the compiler emits, so resolving one over the network at build time would make compiler output
    /// depend on a remote server — the wrong trade for a project whose claim is reliability.
    Add {
        /// Path to a registry package (a JSON document, or a directory containing `guml.registry.json`).
        package: PathBuf,
        /// Audit and report without writing `guml.json`.
        #[arg(long)]
        dry_run: bool,
    },
    /// Print the active theme, or every class it can emit.
    ///
    /// `--classes` exists because of a real integration problem: a utility-class framework generates
    /// only the classes it can see in your source, and GUML's classes are produced by the compiler at
    /// runtime. Without this list a host's build strips exactly the styles the compiler emits.
    Theme {
        /// A theme document; omit for the shipped theme.
        #[arg(long, value_name = "FILE")]
        theme: Option<PathBuf>,
        /// Print one class per line instead of the theme JSON. Feed this to a Tailwind `@source` or
        /// safelist.
        #[arg(long)]
        classes: bool,
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
        Cmd::Check { file, format, core, registry } => {
            cmd_check(&file, format, core, registry.as_deref())
        }
        Cmd::Build { file, backend, out, format, source_map, core, registry, theme } => cmd_build(
            &file,
            &backend,
            out.as_deref(),
            format,
            source_map,
            core,
            registry.as_deref(),
            theme.as_deref(),
        ),
        Cmd::Ast { file } => cmd_ast(&file),
        Cmd::Lex { file } => cmd_lex(&file),
        Cmd::Tokens { files } => cmd_tokens(&files),
        Cmd::Fmt { files, write, check, canonical } => cmd_fmt(&files, write, check, canonical),
        Cmd::Fix { files, write, rounds } => cmd_fix(&files, write, rounds),
        Cmd::Repair { files, write, rounds, format } => cmd_repair(&files, write, rounds, format),
        Cmd::Validate { paths, strict, format } => cmd_validate(&paths, strict, format),
        Cmd::Explain { code } => cmd_explain(code.as_deref()),
        Cmd::Where { file, emitted_line, backend } => cmd_where(&file, emitted_line, &backend),
        Cmd::Highlight { file, format } => cmd_highlight(&file, format),
        Cmd::Mcp => mcp::serve(),
        Cmd::Registry { tags, for_prompt, validate, docs, registry } => cmd_registry(
            tags,
            for_prompt.as_deref(),
            validate.as_deref(),
            docs,
            registry.as_deref(),
        ),
        Cmd::Add { package, dry_run } => cmd_add(&package, dry_run),
        Cmd::Capabilities { paths, format, csp, max_escapes, assert_inert } => {
            cmd_capabilities(&paths, format, csp.as_deref(), max_escapes, assert_inert)
        }
        Cmd::Theme { theme, classes } => cmd_theme(theme.as_deref(), classes),
    }
}

fn read(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))
}

/// Build the vocabulary a command compiles against.
///
/// Three sources, and the precedence between them is the point:
///
/// 1. The **builtin** vocabulary, or its `core` subset.
/// 2. Every registry in **`guml.json`**, in order. This is what makes a package first-class: the
///    project states its vocabulary once, so the editor, the formatter, `check` and CI cannot disagree
///    about what the words are.
/// 3. **`--registry`**, which wins by being applied last. A one-off override is a real need, and CI
///    should be able to pin explicitly rather than inherit.
///
/// `--core` composes with all of it: a core host may load extra *markup* components, and any app-level
/// entry in those documents is skipped rather than merged, so no package can smuggle behaviour past a
/// host that asked for markup only.
fn vocabulary_for(
    core: bool,
    registry: Option<&Path>,
    project: &project::Project,
) -> Result<guml_registry::Registry> {
    let core = core || project.is_core();
    let mut reg =
        if core { guml_registry::Registry::core() } else { guml_registry::Registry::builtin() };
    for (path, pinned) in project.registry_refs()? {
        let json = read(&path)?;
        // The pin is checked *before* the vocabulary is extended, so a mismatched package never contributes
        // a tag. Refusing rather than warning: a document compiled against the wrong vocabulary is not a
        // degraded build, it is a different document, and the failure would surface somewhere else entirely.
        if let Some(want) = &pinned {
            let found = guml_registry::Registry::audit_package(&json).version;
            match found.as_deref() {
                Some(have) if have == want => {}
                Some(have) => anyhow::bail!(
                    "{} declares version {have}, but {} pins it to {want}\n\
                     a registry decides which tags a document may use, so a version change is a change in \
                     what its documents mean — update the pin deliberately",
                    path.display(),
                    project::FILE_NAME
                ),
                None => anyhow::bail!(
                    "{} declares no version, but {} pins it to {want}\n\
                     add a top-level \"version\" to the package, or drop the pin",
                    path.display(),
                    project::FILE_NAME
                ),
            }
        }
        reg = reg.extend_from_json(&json).map_err(|e| {
            anyhow::anyhow!("{} (from {}): {e}", path.display(), project::FILE_NAME)
        })?;
    }
    if let Some(path) = registry {
        let json = read(path)?;
        reg =
            reg.extend_from_json(&json).map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
    }
    Ok(reg)
}

/// The vocabulary for a command operating on `path`, with that file's project config discovered.
fn vocabulary_near(
    path: &Path,
    core: bool,
    registry: Option<&Path>,
) -> Result<guml_registry::Registry> {
    let project = project::Project::discover(path)?;
    vocabulary_for(core, registry, &project)
}

fn cmd_check(path: &Path, format: Format, core: bool, registry: Option<&Path>) -> Result<()> {
    let src = read(path)?;
    let reg = vocabulary_near(path, core, registry)?;
    let (_, diags) = guml_compiler::check_with(&src, &reg);
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
        eprintln!(
            "
run: guml fmt --write {}",
            unformatted.join(" ")
        );
        std::process::exit(1);
    }
    Ok(())
}

/// The free layer of the repair loop.
///
/// Every diagnostic carrying a `suggestion` is a repair the compiler already worked out.
/// Spending a model round on it is the most expensive possible way to rename a typo, so this
/// runs first and reports what it did — which is what makes the saving measurable.
fn cmd_fix(files: &[PathBuf], write: bool, rounds: usize) -> Result<()> {
    let mut total = 0usize;
    for path in files {
        let src = read(path)?;
        let out = guml_compiler::fix::fix(&src, rounds);
        let before = check(&src).1.error_count();
        let after = check(&out.text).1.error_count();

        if write {
            if out.text != src {
                std::fs::write(path, &out.text)
                    .with_context(|| format!("writing {}", path.display()))?;
            }
            println!(
                "{}: {} applied ({}), {before} errors -> {after}",
                path.display(),
                out.codes.len(),
                if out.codes.is_empty() { "none".to_string() } else { out.codes.join(", ") },
            );
        } else {
            print!("{}", out.text);
        }
        total += out.codes.len();
    }
    if write {
        println!(
            "
{total} edit(s) applied with no model call"
        );
    }
    Ok(())
}

/// The whole free repair pipeline: sanitise, format, fix.
///
/// Separate from `fix` rather than a flag on it, for the same reason `html-cdn` is a separate backend
/// name: `fix` only ever applies edits the compiler described, and this also *deletes* things — a code
/// fence, trailing commentary. That is a different promise, and it should not become the default
/// behaviour of an existing command by accident.
fn cmd_repair(files: &[PathBuf], write: bool, rounds: usize, format: Format) -> Result<()> {
    // Stdin is the shape a generation pipeline wants: pipe the model's raw output straight in.
    if files.is_empty() {
        let mut src = String::new();
        std::io::stdin().read_to_string(&mut src).context("reading stdin")?;
        let out = guml_compiler::repair::repair(&src, rounds);
        if format == Format::Json {
            println!("{}", repair_json(&out, None));
        } else {
            print!("{}", out.text);
        }
        return Ok(());
    }

    let mut reports = Vec::new();
    let mut still_broken = 0usize;
    for path in files {
        let src = read(path)?;
        let out = guml_compiler::repair::repair(&src, rounds);
        if !out.ok() {
            still_broken += 1;
        }

        match format {
            Format::Json => reports.push(repair_json(&out, Some(path))),
            Format::Human => {
                if write {
                    if out.text != src {
                        std::fs::write(path, &out.text)
                            .with_context(|| format!("writing {}", path.display()))?;
                    }
                    println!(
                        "{}: {} errors -> {}{}",
                        path.display(),
                        out.errors_before,
                        out.errors_after,
                        if out.changed() { "" } else { " (unchanged)" }
                    );
                    for line in out.report() {
                        println!("  {line}");
                    }
                } else {
                    print!("{}", out.text);
                }
            }
        }
    }

    if format == Format::Json {
        println!("[{}]", reports.join(","));
    }
    // Non-zero when something is still wrong, so a pipeline can branch on "does this need a model
    // round" without parsing the output.
    if still_broken > 0 {
        std::process::exit(1);
    }
    Ok(())
}

/// One repair record. Hand-built rather than derived: `Repaired` carries a `Diagnostics`, and the
/// harness wants the *layer* accounting here, with diagnostics fetched separately by `check`.
fn repair_json(out: &guml_compiler::repair::Repaired, path: Option<&Path>) -> String {
    let file = path.map(|p| p.display().to_string()).unwrap_or_else(|| "<stdin>".to_string());
    serde_json::json!({
        "file": file,
        "ok": out.ok(),
        "changed": out.changed(),
        "errorsBefore": out.errors_before,
        "errorsAfter": out.errors_after,
        "sanitize": {
            "fence": out.stripped.fence,
            "rules": out.stripped.rules,
            "trailing": out.stripped.trailing,
        },
        "reformatted": out.reformatted,
        "applied": out.applied,
        "rounds": out.rounds,
        "text": out.text,
    })
    .to_string()
}

/// Batch validation.
///
/// `check` takes one file and is what an editor and the repair loop call. This takes many,
/// which is what you need to answer "did the model produce valid documents" over a run of
/// generations — and it exits non-zero on the first problem so CI notices.
/// Expand paths into `.guml` files, searching a directory.
fn guml_files(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
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
    Ok(files)
}

/// The capability manifest for one or more documents.
fn cmd_capabilities(
    paths: &[PathBuf],
    format: Format,
    csp: Option<&str>,
    max_escapes: Option<usize>,
    assert_inert: bool,
) -> Result<()> {
    let files = guml_files(paths)?;
    let mut reports = Vec::new();
    let mut over_budget: Vec<String> = Vec::new();
    let mut not_inert: Vec<String> = Vec::new();

    for path in &files {
        let src = read(path)?;
        let reg = vocabulary_near(path, false, None)?;
        let (program, _) = guml_compiler::check_with(&src, &reg);
        let m = guml_compiler::capabilities::analyse(&program, src.lines().count());

        let escapes = m.escapes.js + m.escapes.raw;
        if max_escapes.is_some_and(|max| escapes > max) {
            over_budget.push(format!("{}: {escapes} escape hatch(es)", path.display()));
        }
        if assert_inert && !m.is_inert() {
            // Say *which* property failed. "Not inert" sends a reader back to the manifest; naming the
            // reason is the difference between a gate and an obstacle.
            let mut why: Vec<String> = Vec::new();
            if m.script {
                why.push("contains a `js` block".into());
            }
            if !m.network.is_empty() {
                why.push(format!("contacts {}", m.network.join(", ")));
            }
            if m.level != "core" {
                why.push("needs a runtime".into());
            }
            not_inert.push(format!("{}: {}", path.display(), why.join("; ")));
        }

        if let Some(backend) = csp {
            match format {
                Format::Json => reports.push(
                    serde_json::json!({ "file": path.display().to_string(), "csp": m.csp(backend) })
                        .to_string(),
                ),
                Format::Human => println!("{}", m.csp(backend)),
            }
            continue;
        }

        match format {
            Format::Json => reports.push(
                serde_json::to_string(&serde_json::json!({
                    "file": path.display().to_string(),
                    "manifest": m,
                }))
                .unwrap_or_default(),
            ),
            Format::Human => {
                println!("{} — {} ({})", path.display(), m.page, m.level);
                if m.is_inert() {
                    println!("  inert: no script, no network, markup only");
                } else {
                    if m.script {
                        println!("  script: a `js` block runs code the compiler does not check");
                    }
                    if m.raw_markup {
                        println!("  raw: host markup the compiler does not escape");
                    }
                    if !m.network.is_empty() {
                        println!("  network: {}", m.network.join(" "));
                    }
                    for r in &m.requests {
                        println!(
                            "    {:6} {} ({}){}",
                            r.method,
                            r.url,
                            r.from,
                            if r.mutating { "  mutating" } else { "" }
                        );
                    }
                    for c in &m.components {
                        println!("  component `{}` needs a runtime", c.tag);
                    }
                }
                if escapes > 0 {
                    println!(
                        "  escape hatches: {escapes} ({:.1}% of lines)",
                        m.escapes.share_of_lines * 100.0
                    );
                }
            }
        }
    }

    if format == Format::Json {
        println!("[{}]", reports.join(","));
    }
    for line in &not_inert {
        eprintln!("not safe to render untrusted — {line}");
    }
    for line in &over_budget {
        eprintln!("over the escape-hatch budget — {line}");
    }
    if !over_budget.is_empty() || !not_inert.is_empty() {
        // A rising escape-hatch rate is the early warning that the vocabulary is hitting an
        // expressiveness cliff. Exiting non-zero is what makes it a signal rather than a statistic.
        std::process::exit(1);
    }
    if assert_inert && format == Format::Human {
        println!(
            "
{} document(s) are inert: markup only, no script, no network",
            files.len()
        );
    }
    Ok(())
}

fn cmd_validate(paths: &[PathBuf], strict: bool, format: Format) -> Result<()> {
    let files = guml_files(paths)?;

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
        println!(
            "
{} of {} valid{}",
            files.len() - failed,
            files.len(),
            if strict { " (strict)" } else { "" }
        );
    }

    if failed > 0 {
        std::process::exit(1);
    }
    Ok(())
}

/// The prose half of a diagnostic.
///
/// A diagnostic message has to fit on a line and be useful to a repair loop, so it says what is
/// wrong rather than why the language is like that. This is where the second half lives.
fn cmd_explain(code: Option<&str>) -> Result<()> {
    use guml_diagnostics::Code;

    let Some(query) = code else {
        println!("{:<10}  summary", "code");
        println!("{}  {}", "-".repeat(10), "-".repeat(52));
        for c in Code::ALL {
            println!("{:<10}  {}", c.id(), c.title());
        }
        println!(
            "
`guml explain <code>` for the long form."
        );
        return Ok(());
    };

    let Some(found) = Code::from_id(query) else {
        anyhow::bail!("no such code: `{query}` (try `guml explain` for the list)");
    };

    println!(
        "{} — {}
",
        found.id(),
        found.title()
    );
    println!("{}", found.explain());
    Ok(())
}

/// Reverse source-map lookup.
///
/// The compiler expands 24 lines of GUML into 160 of TSX, so "the error is at Tasks.tsx:88" is
/// not an answer anybody can act on. This turns it back into a line the author wrote.
fn cmd_where(path: &Path, emitted_line: u32, backend: &str) -> Result<()> {
    let src = read(path)?;
    let res = compile(&src, &Options { backend: backend.to_string(), ..Default::default() });

    let Some(file) = res.files.first() else {
        anyhow::bail!("the `{backend}` backend emitted nothing");
    };
    let Some(map) = &file.source_map else {
        anyhow::bail!("the `{backend}` backend does not record line provenance");
    };
    if emitted_line == 0 {
        anyhow::bail!("emitted lines are numbered from 1");
    }

    match map.source_line_of(emitted_line - 1) {
        Some(source_line) => {
            let text = src.lines().nth(source_line as usize - 1).unwrap_or("").trim_end();
            let emitted = file.contents.lines().nth(emitted_line as usize - 1).unwrap_or("").trim();
            println!("{}:{source_line}", path.display());
            println!("  guml     {text}");
            println!("  emitted  {emitted}");
        }
        None => println!(
            "{}:{emitted_line} has no mapping — compiler boilerplate rather than anything the author wrote",
            file.path
        ),
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

#[allow(clippy::too_many_arguments)]
fn cmd_build(
    path: &Path,
    backend: &str,
    out: Option<&Path>,
    format: Format,
    source_map: bool,
    core: bool,
    registry: Option<&Path>,
    theme: Option<&Path>,
) -> Result<()> {
    let project = project::Project::discover(path)?;

    // `--theme` wins over `guml.json`, for the same reason `--registry` does: a one-off override is a
    // real need, and CI should be able to pin explicitly instead of inheriting.
    //
    // Either may be a builtin *name* rather than a path — `--theme shadcn` — because a design system a
    // user selects by name should not require them to type a path into someone else's `node_modules`.
    let source = match theme {
        Some(t) => {
            let name = t.to_string_lossy();
            match guml_codegen::theme::Theme::by_name(&name) {
                Some(_) => Some(project::ThemeSource::Builtin(name.into_owned())),
                // Not a builtin and not a file either: say so *here*, naming the builtins, rather than
                // letting it fall through to a bare `reading nope` from the filesystem. A typo'd theme
                // name is the likeliest way to reach this, and the fix is one of the names below.
                None if !t.exists() => anyhow::bail!(
                    "theme `{name}` is not a builtin and no such file exists\n\
                     builtin themes: {}\n\
                     or pass a path to a theme document",
                    guml_codegen::theme::Theme::builtin_names().join(", ")
                ),
                None => Some(project::ThemeSource::File(t.to_path_buf())),
            }
        }
        None => project.theme_source()?,
    };

    if let Some(source) = source {
        let loaded = match &source {
            project::ThemeSource::Builtin(name) => guml_codegen::theme::Theme::by_name(name)
                .expect("checked when the source was resolved"),
            project::ThemeSource::File(path) => {
                let json = read(path)?;
                guml_codegen::theme::Theme::from_json(&json)
                    .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?
            }
        };
        // Write-once per process, and this is the only caller, so a failure here would be a bug
        // rather than a user error.
        guml_codegen::theme::set(loaded).map_err(|t| {
            anyhow::anyhow!("a theme (`{}`) is already active for this process", t.name)
        })?;
    }

    // A project may name its own default backend. The flag has a clap default of `react`, so the config
    // only applies when the caller left it alone — checked against the default rather than with an
    // `Option`, because making the flag optional would change `--help` for every existing user.
    let backend = if backend == "react" {
        project.backend.clone().unwrap_or_else(|| backend.to_string())
    } else {
        backend.to_string()
    };
    let backend = backend.as_str();

    if guml_compiler::resolve_backend(backend).is_none() {
        anyhow::bail!(
            "unknown backend `{backend}` (available: {})",
            guml_compiler::backend_names().join(", ")
        );
    }
    let src = read(path)?;
    let vocabulary = vocabulary_for(core, registry, &project)?;

    // The backends need the *loaded* vocabulary, not just the builtins, or a package's `element` and
    // `capabilities` are invisible to them — which is what made a registry package validate documents it
    // could not compile. Write-once per process, and this is the only caller.
    guml_codegen::set_registry(vocabulary.clone())
        .map_err(|_| anyhow::anyhow!("a vocabulary is already active for this process"))?;

    let res = compile(&src, &Options { backend: backend.to_string(), registry: vocabulary });
    report(&res.diagnostics, &src, path, format);

    if res.diagnostics.has_errors() {
        std::process::exit(1);
    }

    let source_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "input.guml".to_string());

    for f in &res.files {
        // The map is serialised here rather than in the backend, because this is the layer that
        // holds the source text the map has to inline.
        let map = match (source_map, &f.source_map) {
            (true, Some(m)) => Some(m.to_json(&source_name, &src, f.contents.lines().count())),
            _ => None,
        };
        let contents = match &map {
            Some(_) => format!(
                "{}//# sourceMappingURL={}.map
",
                f.contents, f.path
            ),
            None => f.contents.clone(),
        };

        match out {
            Some(dir) => {
                std::fs::create_dir_all(dir)?;
                let target = dir.join(&f.path);
                std::fs::write(&target, &contents)
                    .with_context(|| format!("writing {}", target.display()))?;
                if format == Format::Human {
                    println!("wrote {}", target.display());
                }
                if let Some(json) = &map {
                    let map_path = dir.join(format!("{}.map", f.path));
                    std::fs::write(&map_path, json)
                        .with_context(|| format!("writing {}", map_path.display()))?;
                    if format == Format::Human {
                        println!("wrote {}", map_path.display());
                    }
                }
            }
            None => print!("{contents}"),
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

fn cmd_theme(path: Option<&Path>, classes: bool) -> Result<()> {
    let theme = match path {
        Some(p) => guml_codegen::theme::Theme::from_json(&read(p)?)
            .map_err(|e| anyhow::anyhow!("{}: {e}", p.display()))?,
        None => guml_codegen::theme::active().clone(),
    };

    if !classes {
        println!("{}", serde_json::to_string_pretty(&theme)?);
        return Ok(());
    }

    // Sorted and deduplicated: this is consumed by build tooling, so a stable order means a stable
    // diff when the theme changes.
    let mut out: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for rule in &theme.rules {
        out.extend(rule.base.split_whitespace());
        out.extend(rule.add.split_whitespace());
    }
    out.extend(theme.contract.focus_visible.split_whitespace());
    out.extend(theme.contract.disabled.split_whitespace());
    for class in out {
        println!("{class}");
    }
    Ok(())
}

fn cmd_registry(
    tags: Option<Vec<String>>,
    for_prompt: Option<&str>,
    validate: Option<&Path>,
    docs: bool,
    registry: Option<&Path>,
) -> Result<()> {
    // Auditing a package is about the *document*, not about the active vocabulary, so it happens before
    // anything is loaded — and reports every problem rather than the first, because a package author
    // fixing five entries should not need five runs.
    if let Some(path) = validate {
        let file = package_document(path);
        let json = read(&file)?;
        let audit = guml_registry::Registry::audit_package(&json);
        print_audit(&audit, &file);
        if !audit.ok() {
            std::process::exit(1);
        }
        return Ok(());
    }

    let project = project::Project::discover(Path::new("."))?;
    let reg = vocabulary_for(false, registry, &project)?;

    if docs {
        print!("{}", registry_markdown(&reg));
        return Ok(());
    }

    // Retrieval: the slice a task description implies. Generous by design — a missing tag makes the
    // task impossible to express, an extra one costs about eight tokens.
    if let Some(prompt) = for_prompt {
        print!("{}", reg.prompt_context(&reg.tags_for_prompt(prompt)));
        return Ok(());
    }

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
            // Printed so the docs site's highlighter reads the vocabulary instead of keeping its own
            // copy. Three separate bugs in this project were a second hand-maintained list drifting
            // from the compiler's, and each surfaced only because someone happened to compare them.
            println!("directives: {}", guml_fmt::highlight::DIRECTIVES.join(" "));
            // Tags whose indented children are *content lines* rather than elements. Printed for the
            // same reason the modifier and directive lists are: the tree-sitter scanner needs it to know
            // where a verbatim line is legal, and a hand-maintained copy would be a second vocabulary
            // that can drift from the compiler.
            println!("content-children: {}", guml_registry::TEXT_CHILD_TAGS.join(" "));
        }
    }
    Ok(())
}

fn print_audit(audit: &guml_registry::PackageAudit, path: &Path) {
    let label = audit
        .name
        .as_deref()
        .map(|n| match &audit.version {
            Some(v) => format!("{n} {v}"),
            None => n.to_string(),
        })
        .unwrap_or_else(|| path.display().to_string());
    println!("{label}: {} component(s)", audit.components.len());
    if !audit.components.is_empty() {
        println!("  {}", audit.components.join(" "));
        println!("  ~{} est. prompt tokens for the whole package", audit.approx_prompt_tokens);
    }
    for w in &audit.warnings {
        eprintln!("  warning: {w}");
    }
    for e in &audit.errors {
        eprintln!("  error: {e}");
    }
    if audit.ok() {
        println!("  no errors");
    }
}

/// Reference documentation for a vocabulary, as Markdown.
///
/// Generated rather than written. The docs site already generates its vocabulary block from the
/// compiler because three separate bugs in this project were a hand-maintained second list drifting from
/// the registry — and each surfaced only because somebody happened to compare them. A *host* publishing
/// a design system has exactly the same problem and no such script, so the compiler emits it.
fn registry_markdown(reg: &guml_registry::Registry) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(out, "# Component vocabulary\n");
    let _ = writeln!(
        out,
        "Generated by `guml registry --docs` from the active registry. Do not edit by hand: a table \
         that disagrees with the compiler is worse than no table.\n"
    );
    let _ = writeln!(
        out,
        "Builtin vocabulary version: `{}`.\n",
        guml_registry::Registry::builtin_version()
    );

    for kind in ["Container", "Text", "Control", "Field", "Repeater"] {
        let names: Vec<&str> =
            reg.names().filter(|n| format!("{:?}", reg.get(n).unwrap().kind) == kind).collect();
        if names.is_empty() {
            continue;
        }
        let _ = writeln!(out, "## {kind}\n");
        let _ = writeln!(out, "| tag | level | attributes | notes | since |");
        let _ = writeln!(out, "|---|---|---|---|---|");
        for name in names {
            let c = reg.get(name).unwrap();
            let attrs = if c.attrs.is_empty() {
                "—".to_string()
            } else {
                c.attrs.iter().map(|a| format!("`{a}`")).collect::<Vec<_>>().join(" ")
            };
            // The things a page has to say that the doc line does not: what a component needs from its
            // host, and what its children may be.
            let mut notes = vec![c.doc.replace('|', "\\|")];
            if c.a11y.requires_label {
                notes.push("Needs an accessible name.".to_string());
            }
            if c.capabilities.needs_runtime {
                notes.push("Needs a JavaScript runtime.".to_string());
            }
            if c.capabilities.network {
                notes.push("Makes network requests.".to_string());
            }
            if c.children.is_leaf() {
                notes.push("Takes no children.".to_string());
            } else if !c.children.allow.is_empty() {
                notes.push(format!(
                    "Children: {}.",
                    c.children
                        .allow
                        .iter()
                        .map(|a| format!("`{a}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            if !c.children.require.is_empty() {
                notes.push(format!(
                    "Requires at least one {}.",
                    c.children
                        .require
                        .iter()
                        .map(|a| format!("`{a}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            if !c.positionals.is_empty() {
                notes.push(format!("Positionals: {}.", c.positionals.join(", ")));
            }
            let _ = writeln!(
                out,
                "| `{}` | {} | {attrs} | {} | {} |",
                c.name,
                c.level.as_str(),
                notes.join(" "),
                c.since.as_deref().unwrap_or("—")
            );
        }
        out.push('\n');
    }

    let _ = writeln!(out, "## Modifiers\n");
    let _ = writeln!(
        out,
        "Semantic, never utility classes — the compiler owns all presentation.\n\n`{}`\n",
        guml_registry::MODIFIERS.join("` `")
    );
    let _ = writeln!(out, "## Global attributes\n");
    let _ = writeln!(out, "`{}`\n", guml_registry::GLOBAL_ATTRS.join("` `"));
    out
}

/// The registry document at a package path.
///
/// A directory is accepted so a vendored package can be a folder with a conventional filename, which is how
/// a design system actually ships.
///
/// Shared by `guml add` and `guml registry --validate` because they disagreed: `add` resolved a directory and
/// `--validate` did not, so auditing before installing — the order anyone would use them in — failed with
/// "Access is denied", which reads like a permissions problem rather than a path convention.
fn package_document(package: &Path) -> PathBuf {
    if package.is_dir() { package.join("guml.registry.json") } else { package.to_path_buf() }
}

/// Install a registry package into `guml.json`.
fn cmd_add(package: &Path, dry_run: bool) -> Result<()> {
    let file = package_document(package);
    let json = read(&file)?;

    let audit = guml_registry::Registry::audit_package(&json);
    print_audit(&audit, &file);
    if !audit.ok() {
        anyhow::bail!(
            "{} has {} error(s) and was not installed",
            file.display(),
            audit.errors.len()
        );
    }

    // Loaded against the *project's* current vocabulary, not just the builtins: two packages can each be
    // valid alone and collide with each other, and finding that out at install time is the whole reason
    // to have an install step.
    let mut project = project::Project::discover(Path::new("."))?;
    let existing = vocabulary_for(false, None, &project)?;
    existing.extend_from_json(&json).map_err(|e| {
        anyhow::anyhow!("{} conflicts with the project's vocabulary: {e}", file.display())
    })?;

    let config_path = if project.root.as_os_str().is_empty() {
        PathBuf::from(project::FILE_NAME)
    } else {
        project.root.join(project::FILE_NAME)
    };

    // Stored relative to the config where possible, so the file is portable across checkouts.
    let stored = pathdiff(&file, &project.root);
    if project.registries.iter().any(|r| r.path() == stored) {
        println!("{} is already in {}", stored.display(), config_path.display());
        return Ok(());
    }

    // Pinned to whatever the package declares, when it declares one. `add` has just audited this exact file,
    // so recording the version it saw is free — and the alternative is a config that silently follows a
    // vocabulary as it changes, which is the thing pinning exists to prevent. A package with no `version`
    // gets a bare path, because inventing one would be a claim the package did not make.
    let entry = match &audit.version {
        Some(version) => {
            project::RegistryRef::Pinned { path: stored.clone(), version: version.clone() }
        }
        None => project::RegistryRef::Path(stored.clone()),
    };
    let pin = audit.version.as_deref().map(|v| format!(" pinned to {v}")).unwrap_or_default();
    if dry_run {
        println!("would add {}{pin} to {}", stored.display(), config_path.display());
        return Ok(());
    }
    project.registries.push(entry);
    project.save(&config_path)?;
    println!("added {}{pin} to {}", stored.display(), config_path.display());
    Ok(())
}

/// `file` expressed relative to `base`, when that is possible without `..` gymnastics.
///
/// Deliberately simple: a prefix strip, falling back to the path as given. A full relative-path
/// computation would need to canonicalise both sides, and a wrong answer here writes a broken path into
/// a config file — the failure is worse than the inconvenience of an absolute path.
fn pathdiff(file: &Path, base: &Path) -> PathBuf {
    let (Ok(f), Ok(b)) = (file.canonicalize(), base.canonicalize()) else {
        return file.to_path_buf();
    };
    match f.strip_prefix(&b) {
        // `./` prefix so the value reads as a path rather than a package name.
        Ok(rest) => PathBuf::from(".").join(rest),
        Err(_) => file.to_path_buf(),
    }
}

fn report(diags: &guml_diagnostics::Diagnostics, src: &str, path: &Path, format: Format) {
    match format {
        // Nothing to say, so say nothing. A terminal does not want an empty line.
        Format::Human if diags.is_empty() => {}
        Format::Human => eprint!("{}", diags.render(src, &path.display().to_string())),
        // **`[]`, not silence.** A clean document used to print nothing here, which is not valid JSON
        // — so every consumer of `--format json` had to special-case empty output before parsing, and
        // the one case they most need to handle correctly (the document is fine) was the one the
        // format did not describe. The repair loop reads this.
        Format::Json => println!("{}", diags.to_json()),
    }
}
