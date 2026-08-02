//! What a document will actually do, stated before anything runs it.
//!
//! # The question this answers
//!
//! `core` versus `app` answers "may an untrusted agent send me this at all" — one bit, decided by the
//! vocabulary. That is the right first question and it is far too coarse to act on. A host embedding a
//! compiled document needs to know *which origins it will contact*, *whether it contains script*, and
//! *whether it reads storage*, because those are the things a Content-Security-Policy is written in terms
//! of. Answering them from a network log after the fact is not a security posture.
//!
//! So this walks a document and produces a manifest. Every field is derived from the AST — there is no
//! declaration for an author to get wrong, and nothing to keep in sync with the code.
//!
//! # Why a CSP is generated rather than documented
//!
//! Prose telling a host "add the origins your document uses to `connect-src`" puts the compiler's own
//! knowledge in a paragraph and asks a human to reproduce it. The compiler knows the exact list. A
//! generated `connect-src` is the difference between advice and a guarantee, and a wrong-by-omission CSP
//! is the failure mode that gets discovered in production.
//!
//! The generated policy is deliberately **tight and honest about its own gaps**: it names what the
//! document needs and nothing else, and where the compiler's own output requires a loosening (inline
//! styles, in the static-HTML backend) it says so with the reason rather than quietly adding
//! `unsafe-inline` to everything.
//!
//! # The escape-hatch budget
//!
//! `raw` and `js` are the two constructs where every guarantee stops. The report names a rising
//! escape-hatch rate as the early warning that the vocabulary is hitting an expressiveness cliff
//! (§12.1 risk 5), which is only useful if somebody is counting. `Manifest::escapes` is that count, and
//! `guml capabilities --max-escapes` turns it into something CI fails on.

use guml_ast::{Element, Program};
use serde::Serialize;

/// What a document needs from its host.
#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub page: String,
    /// `core` if the document needs no runtime at all, else `app`.
    pub level: &'static str,
    /// Origins the document will issue requests to. `"self"` for a same-origin path.
    pub network: Vec<String>,
    /// Every request, so a reviewer can see *what* is called and not only *where*.
    pub requests: Vec<Request>,
    /// True when the document contains a `js` block — arbitrary code the compiler does not check.
    pub script: bool,
    /// True when the document contains a `raw` block — host markup the compiler does not escape.
    pub raw_markup: bool,
    /// True when the document reads or writes host storage. Always false today: no construct does, and
    /// the field exists so a registry component that declares `capabilities.storage` has somewhere to
    /// surface. Reported as false rather than omitted, so a consumer can tell "no" from "unknown".
    pub storage: bool,
    /// Escape-hatch blocks, by kind. The number the report wants tracked continuously.
    pub escapes: Escapes,
    /// Registry components used that declare a capability of their own.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub components: Vec<ComponentNeed>,
}

#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Escapes {
    pub js: usize,
    pub raw: usize,
    /// As a share of the document's lines, which is the form the trend is readable in. A count alone
    /// rises with document size and says nothing.
    pub share_of_lines: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub method: String,
    pub url: String,
    pub origin: String,
    /// The resource or mutation that issues it.
    pub from: String,
    /// True for a request that changes server state, so a reviewer can find them first.
    pub mutating: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentNeed {
    pub tag: String,
    pub needs_runtime: bool,
    pub network: bool,
    pub storage: bool,
}

impl Manifest {
    /// Whether this document could be rendered by a host that permits no script and no network.
    pub fn is_inert(&self) -> bool {
        !self.script && self.network.is_empty() && self.level == "core"
    }

    /// A Content-Security-Policy for this document.
    ///
    /// Derived, not templated. `connect-src` is exactly the origins the document declares, so a request
    /// the document did not declare is blocked by the browser rather than merely unexpected.
    ///
    /// `for_backend` matters because the policy is a property of the *output*, not of the source: the
    /// static-HTML backend inlines the theme stylesheet, so it needs `style-src 'unsafe-inline'` and the
    /// policy says why rather than leaving a reader to guess.
    pub fn csp(&self, for_backend: &str) -> String {
        let mut parts: Vec<String> = vec!["default-src 'none'".into()];

        let connect = if self.network.is_empty() {
            "connect-src 'none'".to_string()
        } else {
            format!(
                "connect-src {}",
                self.network
                    .iter()
                    .map(|o| if o == "self" { "'self'".to_string() } else { o.clone() })
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        };
        parts.push(connect);

        // A document with no behaviour needs no script at all, and saying `'none'` is worth far more than
        // saying `'self'` — it is the difference between "we did not need it" and "we did not restrict it".
        parts.push(if self.level == "core" && !self.script {
            "script-src 'none'".into()
        } else {
            "script-src 'self'".into()
        });

        match for_backend {
            // The no-JavaScript backend inlines the stylesheet, because it has no build step by design and
            // therefore cannot run a utility-class compiler. That is a real requirement, so it is stated
            // with its reason rather than added silently.
            "html" | "mcp-ui" => parts.push(
                "style-src 'unsafe-inline' /* the html backend inlines the theme; it has no build step */"
                    .into(),
            ),
            _ => parts.push("style-src 'self'".into()),
        }

        parts.push("img-src 'self' data:".into());
        parts.push("form-action 'none'".into());
        // Two that cost nothing and close the two most common embedding attacks.
        parts.push("frame-ancestors 'none'".into());
        parts.push("base-uri 'none'".into());
        parts.join("; ")
    }
}

/// Build the manifest for a document.
pub fn analyse(program: &Program, source_lines: usize) -> Manifest {
    let mut m = Manifest {
        page: program.page.as_ref().map(|p| p.name.clone()).unwrap_or_else(|| "Page".into()),
        level: "core",
        storage: false,
        ..Default::default()
    };

    for r in &program.resources {
        m.requests.push(Request {
            method: if r.method.is_empty() { "GET".into() } else { r.method.clone() },
            url: r.url.clone(),
            origin: origin_of(&r.url),
            from: r.name.clone(),
            mutating: false,
        });
        for mu in &r.mutations {
            m.requests.push(Request {
                method: mu.method.clone(),
                url: mu.url.clone(),
                origin: origin_of(&mu.url),
                from: format!("{}.{}", r.name, mu.name),
                // Anything that is not a read changes something. `GET` and `HEAD` are the safe methods;
                // deciding by method rather than by name means a mutation called `fetch` is still counted.
                mutating: !matches!(mu.method.as_str(), "GET" | "HEAD" | ""),
            });
        }
    }
    for r in &m.requests {
        if !r.origin.is_empty() && !m.network.contains(&r.origin) {
            m.network.push(r.origin.clone());
        }
    }
    m.network.sort();

    walk(&program.tree, &mut m);

    // App-level if anything needs a runtime. The same question `mcp-ui` asks to choose its media type,
    // asked here so a host gets one answer from both.
    let interactive = !program.states.is_empty()
        || !program.resources.is_empty()
        || !program.effects.is_empty()
        || m.script;
    if interactive || has_actions(&program.tree) {
        m.level = "app";
    }

    m.escapes.share_of_lines = if source_lines == 0 {
        0.0
    } else {
        (m.escapes.js + m.escapes.raw) as f32 / source_lines as f32
    };
    m
}

fn walk(els: &[Element], m: &mut Manifest) {
    let reg = guml_registry::Registry::builtin();
    for el in els {
        match el.tag.as_str() {
            "js" => {
                m.script = true;
                m.escapes.js += 1;
            }
            "raw" => {
                m.raw_markup = true;
                m.escapes.raw += 1;
            }
            tag => {
                // A registry component may declare needs of its own — including a loaded third-party one,
                // which is the case a hardcoded list here could never cover.
                if let Some(def) = reg.get(tag) {
                    let c = &def.capabilities;
                    if c.needs_runtime || c.network || c.storage {
                        if c.storage {
                            m.storage = true;
                        }
                        if !m.components.iter().any(|n| n.tag == tag) {
                            m.components.push(ComponentNeed {
                                tag: tag.to_string(),
                                needs_runtime: c.needs_runtime,
                                network: c.network,
                                storage: c.storage,
                            });
                        }
                    }
                }
            }
        }
        walk(&el.children, m);
    }
}

fn has_actions(els: &[Element]) -> bool {
    els.iter().any(|el| !el.actions.is_empty() || has_actions(&el.children))
}

/// The origin of a URL, or `"self"` for a same-origin path.
///
/// Shared with the MCP-UI emitter's resource metadata, so a host reading the manifest and a host reading
/// the resource cannot be told two different things about the same document.
fn origin_of(url: &str) -> String {
    guml_codegen::agent::request_origins_of(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(src: &str) -> Manifest {
        let (program, diags) = crate::check(src);
        assert!(!diags.has_errors(), "{:?}", diags.items);
        analyse(&program, src.lines().count())
    }

    #[test]
    fn a_markup_document_needs_nothing() {
        let m = manifest("page P\ncard \"Title\"\n  p Body.\n");
        assert_eq!(m.level, "core");
        assert!(m.is_inert());
        assert!(m.network.is_empty());
        assert!(!m.script);
        assert_eq!(m.escapes.js, 0);
        // The strongest statement a CSP can make, and it is truthful here.
        let csp = m.csp("html");
        assert!(csp.contains("default-src 'none'"), "{csp}");
        assert!(csp.contains("script-src 'none'"), "{csp}");
        assert!(csp.contains("connect-src 'none'"), "{csp}");
    }

    #[test]
    fn every_origin_a_document_contacts_is_listed_once() {
        let src = "page P\ntype T {id}\ndata rows:T[] GET https://api.example.com/rows\n  save PATCH https://api.example.com/rows/{id} {id}\n  log POST /audit {id}\n\nlist rows\n  text {id}\n  empty None.\n";
        let m = manifest(src);
        assert_eq!(m.network, vec!["https://api.example.com", "self"]);
        assert_eq!(m.requests.len(), 3, "{:?}", m.requests);
        // A reviewer looking for what changes server state should not have to read every URL.
        let mutating: Vec<&str> =
            m.requests.iter().filter(|r| r.mutating).map(|r| r.from.as_str()).collect();
        assert_eq!(mutating, vec!["rows.save", "rows.log"]);
        // And the CSP names exactly those origins — a request the document did not declare is blocked by
        // the browser rather than merely unexpected.
        let csp = m.csp("react");
        assert!(csp.contains("connect-src https://api.example.com 'self'"), "{csp}");
    }

    #[test]
    fn an_escape_hatch_is_counted_as_a_rate_not_only_a_number() {
        // A count alone rises with document size and says nothing about whether the vocabulary is
        // failing. The share is the form the trend is readable in.
        let src =
            "page P\nstate n=0\n\njs\n  const helper = 1;\nraw react\n  <Chart />\nmetric {n}\n";
        let m = manifest(src);
        assert_eq!(m.escapes.js, 1);
        assert_eq!(m.escapes.raw, 1);
        assert!(m.escapes.share_of_lines > 0.0);
        assert!(m.script && m.raw_markup);
        // Script present means the CSP can no longer say `'none'`.
        assert!(m.csp("react").contains("script-src 'self'"), "{}", m.csp("react"));
        assert!(!m.is_inert());
    }

    #[test]
    fn a_component_declaring_a_need_surfaces_in_the_manifest() {
        // Read from the registry rather than from a list here, which is what makes it cover a loaded
        // third-party component too.
        let m = manifest("page P\nstate open=true\nmodal \"Edit\" if={open}\n  p Body.\n");
        assert!(
            m.components.iter().any(|c| c.tag == "modal" && c.needs_runtime),
            "{:?}",
            m.components
        );
        assert_eq!(m.level, "app");
    }

    #[test]
    fn actions_alone_make_a_document_app_level() {
        // No state, no data — but a button that does something still needs a runtime, and a host allowing
        // only markup has to be told.
        let m = manifest("page P\nstate n=0\nbtn Go >n = 1\nmetric {n}\n");
        assert_eq!(m.level, "app");
        assert!(!m.is_inert());
    }
}
