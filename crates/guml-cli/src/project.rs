//! `guml.json` — the project's compiler configuration.
//!
//! # Why a config file, and why now
//!
//! Registry packages and themes were reachable only through `--registry` and `--theme`. That is enough
//! to *prove* the loadable-registry design works and not enough to use it: every `check`, every `build`,
//! the editor, the formatter and CI each had to be told the same paths, and the moment one of them is
//! not, that call compiles against a different vocabulary. A document that is valid in the editor and
//! invalid in CI is the worst failure a closed vocabulary can have, because the whole point of closing it
//! is that everyone agrees on what the words are.
//!
//! So the vocabulary belongs to the *project*, stated once. The flags still work and still win, because
//! a one-off override is a real need and because CI should be able to pin explicitly.
//!
//! # Shape
//!
//! ```json
//! {
//!   "registries": [
//!     "./design-system.registry.json",
//!     { "path": "./widgets", "version": "0.1.0" }
//!   ],
//!   "theme": "./brand.theme.json",
//!   "backend": "react",
//!   "level": "app"
//! }
//! ```
//!
//! # Pinning
//!
//! A registry entry may be a bare path or `{ "path": …, "version": … }`. With a version, loading **fails**
//! if the package's own `version` differs.
//!
//! The reason it is worth having is the same reason the vocabulary belongs to the project: a registry decides
//! which tags a document may use and which classes the compiler emits, so a package that changes underneath a
//! project changes what its documents *mean*. Adding a tag is not even purely additive — a `def` may not
//! shadow one, so a document that defined its own `stat` stopped compiling the release `stat` became builtin,
//! and that happened three times in this repository when the vocabulary grew from 28 entries to 49.
//!
//! **Exact equality, not a range.** A range needs a resolver, a lockfile and a policy for what "compatible"
//! means for a vocabulary — and semver's answer ("additive is minor") is the one this project has evidence
//! against. Exact is the version that needs no design decision, and a project that wants to move pins moves
//! the string. The docs said "pin a registry" for a while before any of this existed, which is the kind of
//! advice worth not giving.
//!
//! Paths are relative to the config file, not to the working directory — otherwise `guml check` would
//! mean different things from different directories, which is the bug this file exists to remove.
//!
//! # What this deliberately does not do
//!
//! **No network.** `guml add` takes a path. A registry decides what tags a document may use and what
//! classes the compiler emits, so fetching one from a URL at build time would make the compiler's output
//! depend on a remote server — a supply-chain surface for a project whose pitch is reliability. Packages
//! arrive the way any dependency does (a file, a vendored directory, `node_modules`), and are installed
//! by an explicit command rather than resolved implicitly.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const FILE_NAME: &str = "guml.json";

/// One configured registry: a bare path, or a path with a pinned version.
///
/// Untagged, so both spellings are accepted and a project that does not pin writes nothing extra. The bare
/// form serialises back as a bare string, which keeps `guml add` from rewriting a hand-written config into a
/// shape its author did not choose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RegistryRef {
    Path(PathBuf),
    Pinned {
        path: PathBuf,
        /// Exact version the package must declare. See the module docs on why exact rather than a range.
        version: String,
    },
}

impl RegistryRef {
    pub fn path(&self) -> &Path {
        match self {
            RegistryRef::Path(p) => p,
            RegistryRef::Pinned { path, .. } => path,
        }
    }

    pub fn version(&self) -> Option<&str> {
        match self {
            RegistryRef::Path(_) => None,
            RegistryRef::Pinned { version, .. } => Some(version),
        }
    }
}

/// How a `theme` entry was written, resolved in [`Project::theme_source`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThemeSource {
    /// A theme compiled into the binary: `"tailwind"`, `"shadcn"`.
    Builtin(String),
    /// A `.json` theme document, or a plugin directory containing `guml.theme.json`.
    File(PathBuf),
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Project {
    /// Plugins: packages that contribute vocabulary, styling, or both.
    ///
    /// The friendly form of `registries` + `theme`, and the one to reach for. Each entry is a package
    /// name resolved through `node_modules` (`"@guml/shadcn"`) or a directory (`"./design-system"`),
    /// and the compiler loads whichever of these it finds inside:
    ///
    /// * `guml.registry.json` — tags, and the components they lower to
    /// * `guml.theme.json` — the class table
    ///
    /// A design system is normally both, and stating it once is the point: naming the vocabulary and
    /// the styling separately is two chances to install one and forget the other, which produces a
    /// document full of tags that render unstyled.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub plugins: Vec<String>,
    /// Registry packages to load, in order. Later packages may not shadow earlier ones or the builtins.
    ///
    /// Explicit paths to registry *files*. `plugins` covers the common case; this stays for a registry
    /// that is not laid out as a package, and for pinning.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub registries: Vec<RegistryRef>,
    /// The design-system table: a builtin name (`"tailwind"`, `"shadcn"`), a path to a theme document,
    /// or a plugin that ships one.
    ///
    /// Omitted means `tailwind` — stock utilities that any Tailwind install resolves with no setup.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<PathBuf>,
    /// Default backend for `guml build`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend: Option<String>,
    /// `core` to compile markup-only by default. A host embedding untrusted documents states this once
    /// here rather than remembering `--core` at every call site.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    /// Where this config was loaded from. Not serialised — it is how relative paths get resolved.
    #[serde(skip)]
    pub root: PathBuf,
}

impl Project {
    /// Find and load `guml.json`, walking up from `start` to the filesystem root.
    ///
    /// Walking up rather than requiring the working directory to be the project root, because that is
    /// what every other tool in this space does and because `guml check src/pages/home.guml` should work
    /// from anywhere in the tree.
    ///
    /// Returns the default config when there is none. A project without a config file is the normal case
    /// and must not need one.
    pub fn discover(start: &Path) -> Result<Self> {
        let mut dir = if start.is_dir() {
            start.to_path_buf()
        } else {
            start.parent().unwrap_or(Path::new(".")).to_path_buf()
        };
        // `Path::parent` of a bare filename is `""`, not `"."` — and an empty path canonicalizes to an
        // error, joins to a relative name, and `pop`s to `false`. So `guml check page.guml` found no
        // config at all while `guml check ./page.guml` found one: the walk never started.
        if dir.as_os_str().is_empty() {
            dir = PathBuf::from(".");
        }
        // Canonicalise so the walk is over absolute directories: `pop` on a relative path stops at the
        // first component, which would search the current directory and give up.
        dir = dir.canonicalize().unwrap_or(dir);
        loop {
            let candidate = dir.join(FILE_NAME);
            if candidate.is_file() {
                return Self::load(&candidate);
            }
            if !dir.pop() {
                return Ok(Self::default());
            }
        }
    }

    pub fn load(path: &Path) -> Result<Self> {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let mut project: Self = serde_json::from_str(&text)
            .with_context(|| format!("{} is not valid JSON", path.display()))?;
        project.root = path.parent().unwrap_or(Path::new(".")).to_path_buf();
        Ok(project)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let json =
            serde_json::to_string_pretty(self).context("serialising the project configuration")?;
        std::fs::write(path, format!("{json}\n"))
            .with_context(|| format!("writing {}", path.display()))
    }

    /// A configured path, resolved against the config's own directory.
    pub fn resolve(&self, path: &Path) -> PathBuf {
        if path.is_absolute() { path.to_path_buf() } else { self.root.join(path) }
    }

    /// Locate a plugin: a `node_modules` package name, or a directory.
    ///
    /// `node_modules` is searched upward from the config, the way every JavaScript tool resolves — a
    /// pnpm workspace hoists to the repository root, so looking only beside `guml.json` would fail in
    /// exactly the layout this project itself uses.
    pub fn plugin_dir(&self, name: &str) -> Result<PathBuf> {
        let direct = self.resolve(Path::new(name));
        if direct.is_dir() {
            return Ok(direct);
        }

        let mut dir = Some(self.root.as_path());
        while let Some(d) = dir {
            let candidate = d.join("node_modules").join(name);
            if candidate.is_dir() {
                return Ok(candidate);
            }
            dir = d.parent();
        }

        anyhow::bail!(
            "plugin `{name}` not found\n\
             looked for a directory at {} and for `node_modules/{name}` from there upward\n\
             install it (`pnpm add {name}`), or point at a directory if it is local",
            direct.display()
        )
    }

    /// Every registry file to load: each plugin's `guml.registry.json`, then explicit `registries`.
    ///
    /// Plugins first so an explicit entry can be listed after one and win the ordering, matching how
    /// `--registry` wins over the config.
    pub fn registry_refs(&self) -> Result<Vec<(PathBuf, Option<String>)>> {
        let mut out = Vec::new();

        for name in &self.plugins {
            let dir = self.plugin_dir(name)?;
            let registry = dir.join("guml.registry.json");
            if registry.is_file() {
                out.push((registry, None));
            } else if !dir.join("guml.theme.json").is_file() {
                // Neither half present. Silence here would leave the author believing a vocabulary was
                // loaded, and the failure would surface as "unknown tag" pointing at their document
                // rather than at their config.
                anyhow::bail!(
                    "plugin `{name}` at {} contains neither `guml.registry.json` nor `guml.theme.json`\n\
                     a plugin must contribute vocabulary, styling, or both",
                    dir.display()
                );
            }
        }

        out.extend(
            self.registries
                .iter()
                .map(|r| (self.resolve(r.path()), r.version().map(str::to_string))),
        );
        Ok(out)
    }

    /// Where the theme comes from: a builtin name, a file, or a plugin that ships one.
    ///
    /// Resolution order, and the ambiguity is worth stating because `"shadcn"` is both a builtin theme
    /// and a package name:
    ///
    /// 1. A **builtin name** wins. `"shadcn"` means the shipped table, which is what someone typing it
    ///    means, and it needs nothing installed.
    /// 2. Otherwise a **path** — a `.json` file, or a directory holding `guml.theme.json`.
    /// 3. Otherwise a **plugin name**, resolved through `node_modules`.
    ///
    /// With no `theme` at all, a plugin's own `guml.theme.json` applies if exactly one ships a theme.
    /// Two plugins with themes is ambiguous and is reported rather than resolved by list order: the
    /// answer would depend on something the author never intended to express.
    pub fn theme_source(&self) -> Result<Option<ThemeSource>> {
        if let Some(theme) = &self.theme {
            let name = theme.to_string_lossy();
            if guml_codegen::theme::Theme::by_name(&name).is_some() {
                return Ok(Some(ThemeSource::Builtin(name.into_owned())));
            }

            let path = self.resolve(theme);
            if path.is_file() {
                return Ok(Some(ThemeSource::File(path)));
            }
            let in_dir = path.join("guml.theme.json");
            if in_dir.is_file() {
                return Ok(Some(ThemeSource::File(in_dir)));
            }
            if let Ok(dir) = self.plugin_dir(&name) {
                let from_plugin = dir.join("guml.theme.json");
                if from_plugin.is_file() {
                    return Ok(Some(ThemeSource::File(from_plugin)));
                }
            }

            anyhow::bail!(
                "theme `{name}` not found\n\
                 it is not a builtin ({}), not a file at {}, and no plugin by that name ships \
                 `guml.theme.json`",
                guml_codegen::theme::Theme::builtin_names().join(", "),
                path.display()
            );
        }

        let from_plugins: Vec<PathBuf> = self
            .plugins
            .iter()
            .filter_map(|name| self.plugin_dir(name).ok())
            .map(|d| d.join("guml.theme.json"))
            .filter(|p| p.is_file())
            .collect();

        match from_plugins.len() {
            0 => Ok(None),
            1 => Ok(Some(ThemeSource::File(from_plugins.into_iter().next().unwrap()))),
            _ => anyhow::bail!(
                "{} plugins ship a theme and none is selected: {}\n\
                 add a `\"theme\"` naming the one you want — resolving this by list order would make \
                 the design of every page depend on something you did not intend to say",
                from_plugins.len(),
                from_plugins.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(", ")
            ),
        }
    }

    pub fn is_core(&self) -> bool {
        self.level.as_deref() == Some("core")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(json: &str) -> Project {
        serde_json::from_str(json).expect("a project config")
    }

    #[test]
    fn a_registry_is_a_bare_path_or_a_pinned_one() {
        // Both spellings, because a project that does not pin should write nothing extra — and because a
        // config written by hand must not be rejected for choosing the short form.
        let p = parse(r#"{"registries":["./a.json",{"path":"./b","version":"1.2.3"}]}"#);
        assert_eq!(p.registries.len(), 2);
        assert_eq!(p.registries[0].version(), None);
        assert_eq!(p.registries[1].version(), Some("1.2.3"));
        assert_eq!(p.registries[1].path(), Path::new("./b"));
    }

    #[test]
    fn a_bare_path_serialises_back_as_a_bare_path() {
        // `guml add` rewrites this file, so an unpinned entry must not come back as `{"path": …}`. Rewriting
        // a hand-written config into a shape its author did not choose is the kind of thing that makes a
        // tool untrustworthy with your files.
        let p = parse(r#"{"registries":["./a.json"]}"#);
        let out = serde_json::to_string(&p).expect("serialises");
        assert!(out.contains(r#"["./a.json"]"#), "{out}");
    }

    #[test]
    fn paths_resolve_against_the_config_not_the_working_directory() {
        // The bug this whole file exists to remove: `guml check` meaning different things from different
        // directories, so a document is valid in the editor and invalid in CI.
        let mut p = parse(r#"{"registries":[{"path":"./ds.json","version":"0.1.0"}]}"#);
        p.root = PathBuf::from("/project");
        let refs = p.registry_refs().expect("resolvable");
        assert_eq!(refs[0].0, PathBuf::from("/project").join("./ds.json"));
        assert_eq!(refs[0].1.as_deref(), Some("0.1.0"));
    }
}

#[cfg(test)]
mod plugin_tests {
    use super::*;

    fn project_at(dir: &Path, json: &str) -> Project {
        let mut p: Project = serde_json::from_str(json).expect("a project config");
        p.root = dir.to_path_buf();
        p
    }

    /// A plugin directory holding whichever halves the test needs.
    fn plugin(dir: &Path, name: &str, registry: bool, theme: bool) -> PathBuf {
        let d = dir.join(name);
        std::fs::create_dir_all(&d).expect("mkdir");
        if registry {
            std::fs::write(d.join("guml.registry.json"), r#"{"name":"x","components":[]}"#)
                .unwrap();
        }
        if theme {
            std::fs::write(d.join("guml.theme.json"), r#"{"name":"x","contract":{},"rules":[]}"#)
                .unwrap();
        }
        d
    }

    fn tmp(label: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("guml-plugin-{label}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("tmp");
        d
    }

    #[test]
    fn a_plugin_contributes_both_halves_from_one_entry() {
        // The whole point of `plugins` over `registries` + `theme`: naming a design system's vocabulary
        // and its styling separately is two chances to install one and forget the other, and the
        // failure mode of forgetting the theme is a page full of correct tags rendering unstyled.
        let dir = tmp("both");
        plugin(&dir, "design", true, true);
        let p = project_at(&dir, r#"{"plugins":["./design"]}"#);

        assert_eq!(p.registry_refs().unwrap().len(), 1);
        assert!(matches!(p.theme_source().unwrap(), Some(ThemeSource::File(_))));
    }

    #[test]
    fn a_plugin_may_ship_only_a_theme() {
        let dir = tmp("themeonly");
        plugin(&dir, "brand", false, true);
        let p = project_at(&dir, r#"{"plugins":["./brand"]}"#);

        assert!(p.registry_refs().unwrap().is_empty(), "no vocabulary to contribute");
        assert!(matches!(p.theme_source().unwrap(), Some(ThemeSource::File(_))));
    }

    #[test]
    fn a_plugin_contributing_nothing_is_reported() {
        // Silence would leave the author believing a vocabulary was loaded, and the failure would then
        // surface as `unknown tag` pointing at their document rather than at their config.
        let dir = tmp("empty");
        plugin(&dir, "hollow", false, false);
        let p = project_at(&dir, r#"{"plugins":["./hollow"]}"#);

        let err = p.registry_refs().unwrap_err().to_string();
        assert!(err.contains("neither"), "{err}");
    }

    #[test]
    fn a_missing_plugin_says_where_it_looked() {
        let dir = tmp("missing");
        let p = project_at(&dir, r#"{"plugins":["@nope/nothing"]}"#);
        let err = p.registry_refs().unwrap_err().to_string();
        assert!(err.contains("node_modules"), "{err}");
        assert!(err.contains("pnpm add"), "the message should say how to fix it: {err}");
    }

    #[test]
    fn a_builtin_theme_name_beats_a_package_of_the_same_name() {
        // `shadcn` is both a shipped theme and a package. Someone typing `"theme": "shadcn"` means the
        // theme, and must not need the package installed to get it.
        let dir = tmp("builtin");
        let p = project_at(&dir, r#"{"theme":"shadcn"}"#);
        assert_eq!(p.theme_source().unwrap(), Some(ThemeSource::Builtin("shadcn".into())));
    }

    #[test]
    fn an_unknown_theme_names_the_builtins() {
        let dir = tmp("badtheme");
        let p = project_at(&dir, r#"{"theme":"nope"}"#);
        let err = p.theme_source().unwrap_err().to_string();
        assert!(err.contains("tailwind"), "{err}");
        assert!(err.contains("shadcn"), "{err}");
    }

    #[test]
    fn two_plugins_shipping_themes_is_reported_rather_than_resolved_by_order() {
        // Picking by list position would make the design of every page depend on something the author
        // never intended to express, and would change silently when the list was reordered.
        let dir = tmp("twothemes");
        plugin(&dir, "a", true, true);
        plugin(&dir, "b", true, true);
        let p = project_at(&dir, r#"{"plugins":["./a","./b"]}"#);

        let err = p.theme_source().unwrap_err().to_string();
        assert!(err.contains("2 plugins"), "{err}");

        // Naming one resolves it.
        let chosen = project_at(&dir, r#"{"plugins":["./a","./b"],"theme":"./b"}"#);
        assert!(matches!(chosen.theme_source().unwrap(), Some(ThemeSource::File(_))));
    }

    #[test]
    fn no_config_at_all_means_the_default_theme() {
        // The out-of-the-box case, and the reason the default is stock Tailwind: nothing configured,
        // nothing installed, and the output still styles under a bare `pnpm add tailwindcss`.
        let dir = tmp("none");
        let p = project_at(&dir, "{}");
        assert_eq!(p.theme_source().unwrap(), None);
        assert!(p.registry_refs().unwrap().is_empty());
    }
}

#[cfg(test)]
mod schema_tests {
    /// The published JSON Schema must describe the compiler that exists.
    ///
    /// `guml.json` carries `$schema`, so an editor autocompletes from this file. A schema listing a
    /// backend the compiler cannot resolve — or missing one it can — is worse than no schema: it
    /// offers a completion that fails at build time, or underlines a valid config in red. Both send
    /// the author looking in the wrong place.
    #[test]
    fn the_published_schema_lists_exactly_the_backends_that_resolve() {
        let raw = std::fs::read_to_string("../../docs/public/schema/guml.json")
            .expect("the published schema");
        let schema: serde_json::Value = serde_json::from_str(&raw).expect("valid JSON");

        let listed: Vec<&str> = schema["properties"]["backend"]["enum"]
            .as_array()
            .expect("backend enum")
            .iter()
            .map(|v| v.as_str().expect("string"))
            .collect();

        let mut from_schema = listed.clone();
        from_schema.sort_unstable();
        let mut from_compiler = guml_codegen::backend_names().to_vec();
        from_compiler.sort_unstable();

        assert_eq!(
            from_schema, from_compiler,
            "the schema and the compiler disagree about backends"
        );

        for name in &listed {
            assert!(
                guml_codegen::backend(name).is_some(),
                "schema offers `{name}`, which is unknown"
            );
        }
    }

    /// And the themes it offers as examples must be selectable by those names.
    #[test]
    fn the_schema_examples_for_theme_are_real() {
        let raw = std::fs::read_to_string("../../docs/public/schema/guml.json")
            .expect("the published schema");
        let schema: serde_json::Value = serde_json::from_str(&raw).expect("valid JSON");

        for example in schema["properties"]["theme"]["examples"].as_array().expect("examples") {
            let name = example.as_str().expect("string");
            // Paths and package names are resolved at load time; only bare names are claims about
            // what is compiled in.
            if !name.contains('/') && !name.contains('.') {
                assert!(
                    guml_codegen::theme::Theme::by_name(name).is_some(),
                    "the schema offers theme `{name}`, which is not builtin"
                );
            }
        }
    }
}
