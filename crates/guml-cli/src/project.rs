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

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Project {
    /// Registry packages to load, in order. Later packages may not shadow earlier ones or the builtins.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub registries: Vec<RegistryRef>,
    /// Theme document replacing the shipped design-system table.
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

    /// Each configured registry as `(resolved path, pinned version)`.
    pub fn registry_refs(&self) -> Vec<(PathBuf, Option<String>)> {
        self.registries
            .iter()
            .map(|r| (self.resolve(r.path()), r.version().map(str::to_string)))
            .collect()
    }

    pub fn theme_path(&self) -> Option<PathBuf> {
        self.theme.as_ref().map(|p| self.resolve(p))
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
        let refs = p.registry_refs();
        assert_eq!(refs[0].0, PathBuf::from("/project").join("./ds.json"));
        assert_eq!(refs[0].1.as_deref(), Some("0.1.0"));
    }
}
