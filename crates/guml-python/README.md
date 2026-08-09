# guml

Compile [GUML](https://github.com/guml-lang/guml) — an intermediate representation for LLM-generated
user interfaces — to HTML, React, Svelte and more. The real Rust compiler, built as a native
extension. No Node, no Rust toolchain, no build step.

```sh
pip install guml
```

```python
import guml

src = '''page "Dashboard"
card
  h Revenue
  metric $48,120
  p Up 12% on last month
'''

html = guml.render(src)      # a complete page: no JavaScript, nothing to serve alongside it
guml.check(src)              # []
```

`pip install guml` also puts a `guml` command on your PATH.

---

## Two things this is for

### 1. Getting a model to build UI

This is what GUML exists for. A model writing React emits JSX, hooks, effect dependencies, Tailwind
class strings and ARIA attributes — most of it mechanically derivable. GUML moves that to the
compiler, so the model writes what it actually decided and everything conventional is generated.

```python
import guml, anthropic

client = anthropic.Anthropic()

prompt = guml.SPEC + "\n\n" + guml.registry(["card", "btn", "list", "metric"])

reply = client.messages.create(
    model="claude-sonnet-5",
    max_tokens=2000,
    system=prompt,
    messages=[{"role": "user", "content": "A dashboard showing revenue and recent orders"}],
)

source = reply.content[0].text

# Free round. Strips a markdown fence, formats, applies every unambiguous fix.
# Costs nothing and resolves a surprising share of what a first generation gets wrong.
source = guml.repair(source).text

problems = guml.check(source)
if problems:
    # Every problem in one pass, never just the first — each repair round is a full generation.
    feedback = "\n".join(f"{d.code} line {d.line}: {d.message}" for d in problems)
    ...

html = guml.render(source)
```

- **`guml.SPEC`** — the language spec, sized to sit in a system prompt. Rules, not vocabulary.
- **`guml.registry([...])`** — the vocabulary, as a prompt-sized slice. Ask for the dozen tags a task
  needs rather than all 49.
- **`guml.repair()`** — everything a repair loop can do *without* another model call.
- **`guml.check()`** — every diagnostic in one pass. Codes are append-only and never renumbered, so a
  loop can key on them.

### 2. Serving pages from Python

`render()` produces HTML with no JavaScript and no build step, so it drops into any framework.

**Flask**

```python
@app.get("/dashboard")
def dashboard():
    return guml.render(SOURCE)
```

**FastAPI**

```python
from fastapi.responses import HTMLResponse

@app.get("/dashboard", response_class=HTMLResponse)
def dashboard():
    return guml.render(SOURCE)
```

**Django**

```python
from django.http import HttpResponse

def dashboard(request):
    return HttpResponse(guml.render(SOURCE))
```

**Jinja** — the one integration shipped rather than documented, because it is the one that is
annoying to write correctly:

```sh
pip install 'guml[jinja]'
```

```python
from guml.jinja import GumlExtension
app.jinja_env.add_extension(GumlExtension)
```

```jinja
<div class="panel">{{ source | guml }}</div>
```

It defaults to `fragment=True` — no doctype, no `<head>`, no `<main>` — and marks the result safe so
Jinja does not escape it into visible angle brackets.

---

## Security

**`js` and `raw` blocks compile through unchanged.** That is GUML's documented escape hatch and its
security boundary at the same time: everything outside them is constrained, everything inside them is
the author's own code.

So `render()` defaults to **`level="core"`** — markup only, no `state`, no `data`, no actions, no
`js`:

```python
guml.render(source)                 # core: safe for a document you did not write
guml.render(source, level="app")    # full: executes any `js` the author included
```

This deliberately differs from the `guml build` CLI, which defaults to `app`. Different threat model:
the CLI compiles a file you wrote, and `render()` very often does not.

Before rendering something at `app` level, `capabilities()` tells you what it will actually do:

```python
caps = guml.capabilities(source)
if caps.uses_escape_hatch:
    raise ValueError("this document contains js")

response.headers["Content-Security-Policy"] = caps.csp
```

---

## API

| | |
|---|---|
| `render(src, *, level, style, fragment)` | GUML → HTML |
| `compile(src, backend, *, level)` | any of `react`, `svelte`, `html`, `html-bare`, `html-fragment`, `html-cdn`, `wc`, `json`, `a2ui`, `mcp-ui` |
| `check(src, *, level)` | `list[Diagnostic]`, every problem in one pass |
| `raise_for_errors(src)` | the raising version, for `try`/`except` |
| `repair(src)` / `fix(src)` | mechanical repair, no model call |
| `format(src)` / `canonical(src)` | idempotent formatting / normalisation for comparison |
| `capabilities(src, backend)` | what it does, plus a matching CSP |
| `registry(tags)` | prompt-sized vocabulary slice |
| `SPEC` | the language spec, for a system prompt |
| `BACKENDS` | every backend name, from the compiler itself |

`Diagnostic` is a frozen dataclass — `d.code`, `d.line`, `d.column`, `d.message`, `d.help`,
`d.suggestion`, `d.is_error` — not a dict. Fully typed, `py.typed` included.

### Threading

The API is synchronous, deliberately: compiling is CPU-bound and takes single-digit milliseconds, and
an `async def` wrapper would be fake async that still blocks the loop. The GIL **is** released for the
duration of every compile, so Flask on threads and FastAPI's threadpool genuinely parallelise. For a
very large document in an async handler, `await asyncio.to_thread(guml.render, src)`.

---

## Command line

```sh
guml build app.guml --backend html
guml check app.guml
guml fmt app.guml --write
guml capabilities app.guml
```

A subset — enough that compiling a `.guml` file does not require a Rust toolchain. The full CLI
(source maps, custom themes, registry validation, token estimates) is `cargo install guml-cli`.

---

## Also available

- [`@guml/core`](https://www.npmjs.com/package/@guml/core) — the compiler as WebAssembly, plus a React runtime
- [`@guml/fmt`](https://www.npmjs.com/package/@guml/fmt), [`@guml/highlight`](https://www.npmjs.com/package/@guml/highlight) — formatter and highlighter, separately
- [`guml-cli`](https://crates.io/crates/guml-cli) — the full command line
- [Documentation](https://guml.vercel.app/docs/python)

---

MIT.
