import type { Metadata } from "next";
import { CodeBlock } from "@/components/code-block";
import { DocPage } from "@/components/doc-page";
import { A, C, H2, H3, LI, Note, P, Pkg, Table, UL } from "@/components/prose";

export const metadata: Metadata = {
  title: "Python",
  description:
    "Compile GUML from Python: render HTML in Flask, FastAPI or Django, and drive an LLM repair loop.",
};

export default function Page() {
  return (
    <DocPage
      pathname="/docs/python"
      meter={{ label: "package", value: "guml · pip install", tone: "iris" }}
      title="Python"
      lede="The real Rust compiler as a native extension. No Node, no Rust toolchain, no build step — and it works for both things Python is used for here: getting a model to build UI, and serving the result."
      toc={[
        { id: "install", title: "Install" },
        { id: "llm", title: "Building UI with a model" },
        { id: "web", title: "Flask, FastAPI, Django" },
        { id: "jinja", title: "Jinja" },
        { id: "security", title: "Security" },
        { id: "api", title: "API" },
        { id: "threading", title: "Threading" },
        { id: "cli", title: "Command line" },
      ]}
    >
      <H2 id="install">Install</H2>
      <CodeBlock lang="bash" filename="terminal" code={`pip install guml`} />
      <P>
        One wheel per platform, built against the stable C API, so a single artifact covers Python 3.9
        through every later 3.x. It also puts a <C>guml</C> command on your PATH — see{" "}
        <A href="#cli">below</A>.
      </P>

      <H2 id="llm">Building UI with a model</H2>
      <P>
        This is what GUML is for, and Python is where most of that work happens. A model writing React
        emits JSX, hooks, effect dependencies, class strings and ARIA attributes — most of it
        mechanically derivable. GUML moves that to the compiler: the model writes what it actually
        decided, and everything conventional is generated.
      </P>
      <CodeBlock
        lang="python"
        filename="generate.py"
        code={`import guml, anthropic

client = anthropic.Anthropic()

# The spec carries the rules; the registry slice carries the vocabulary. Ask for the
# tags a task plausibly needs rather than all 49 — that is what keeps a prompt small.
system = guml.SPEC + "\\n\\n" + guml.registry(["card", "btn", "list", "metric"])

reply = client.messages.create(
    model="claude-sonnet-5",
    max_tokens=2000,
    system=system,
    messages=[{"role": "user", "content": "A dashboard with revenue and recent orders"}],
)

source = reply.content[0].text

# The free round. Strips a markdown fence the model wrapped it in, formats, and applies
# every unambiguous fix. Costs nothing and resolves a good share of a first generation.
source = guml.repair(source).text

problems = guml.check(source)
if problems:
    # Every problem in one pass, never just the first — each retry is a full generation,
    # so fixing one error per round multiplies the cost by the mistake count.
    feedback = "\\n".join(f"{d.code} line {d.line}: {d.message}" for d in problems)
    ...  # one more model call, with the whole list

html = guml.render(source)`}
      />
      <Note tone="tip" title="Run repair before spending a round on the model">
        <p>
          <C>repair()</C> is sanitise → format → apply-every-suggestion, and none of it needs a model.
          Diagnostics carry a <C>suggestion</C> only when the fix is unambiguous, which is exactly what
          makes applying them automatically safe.
        </p>
      </Note>

      <H2 id="web">Flask, FastAPI, Django</H2>
      <P>
        <C>render()</C> produces HTML with no JavaScript and no build step, so there is nothing to
        serve alongside it and no framework adapter to install.
      </P>
      <H3>Flask</H3>
      <CodeBlock
        lang="python"
        filename="app.py"
        code={`@app.get("/dashboard")
def dashboard():
    return guml.render(SOURCE)`}
      />
      <H3>FastAPI</H3>
      <CodeBlock
        lang="python"
        filename="main.py"
        code={`from fastapi.responses import HTMLResponse

@app.get("/dashboard", response_class=HTMLResponse)
def dashboard():
    return guml.render(SOURCE)`}
      />
      <H3>Django</H3>
      <CodeBlock
        lang="python"
        filename="views.py"
        code={`from django.http import HttpResponse

def dashboard(request):
    return HttpResponse(guml.render(SOURCE))`}
      />

      <H2 id="jinja">Jinja</H2>
      <P>
        The one integration shipped rather than documented, because it is the one that is easy to get
        wrong — and Flask, Django and FastAPI-with-templates all sit on Jinja, so one extension covers
        all three.
      </P>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`pip install 'guml[jinja]'`}
      />
      <CodeBlock
        lang="python"
        filename="app.py"
        code={`from guml.jinja import GumlExtension
app.jinja_env.add_extension(GumlExtension)`}
      />
      <CodeBlock
        lang="tsx"
        filename="template.html"
        code={`<div class="panel">{{ source | guml }}</div>`}
      />
      <P>Two things it does that a hand-written filter usually gets wrong:</P>
      <UL>
        <LI>
          <strong>It marks the output safe.</strong> Jinja escapes strings by default, so a filter
          returning a plain <C>str</C> renders <C>&amp;lt;div&amp;gt;</C> — the markup appears as
          visible text.
        </LI>
        <LI>
          <strong>It defaults to a fragment.</strong> No doctype, no <C>&lt;head&gt;</C>, and no{" "}
          <C>&lt;main&gt;</C>. That last one looks like a loss and is the opposite: a document may hold
          exactly one <C>main</C> landmark, so a fragment carrying its own would create a second the
          moment a template embedded it.
        </LI>
      </UL>

      <H2 id="security">Security</H2>
      <Note tone="warn" title="render() defaults to level=&quot;core&quot;, unlike the CLI">
        <p>
          <C>js</C> and <C>raw</C> blocks compile through <strong>unchanged</strong> — that is GUML&rsquo;s
          documented escape hatch and its security boundary at once. So <C>render()</C> defaults to{" "}
          <C>level=&quot;core&quot;</C>: markup only, no <C>state</C>, no <C>data</C>, no actions, no{" "}
          <C>js</C>.
        </p>
        <p className="mt-3">
          This deliberately differs from <C>guml build</C>, which defaults to <C>app</C>. Different
          threat model: the CLI compiles a file you wrote, and <C>render()</C> very often does not.
        </p>
      </Note>
      <CodeBlock
        lang="python"
        filename="safety.py"
        code={`guml.render(source)                 # core — safe for a document you did not write
guml.render(source, level="app")    # full — executes any js the author included

caps = guml.capabilities(source)
if caps.uses_escape_hatch:
    raise ValueError("this document contains js")

response.headers["Content-Security-Policy"] = caps.csp`}
      />
      <P>
        <C>capabilities()</C> reports what a document will actually do — network, storage, script
        evaluation — and emits a policy permitting exactly that. See{" "}
        <A href="/docs/compiler/capabilities">capabilities and CSP</A>.
      </P>

      <H2 id="api">API</H2>
      <Table
        head={["function", "returns", "for"]}
        rows={[
          ["render(src, *, level, style, fragment)", "str", "GUML → HTML"],
          ["compile(src, backend, *, level)", "CompileResult", "any of the ten backend names — seven code generators, four HTML variants"],
          ["check(src, *, level)", "list[Diagnostic]", "every problem in one pass"],
          ["raise_for_errors(src)", "None", "the raising form, for try/except"],
          ["repair(src) / fix(src)", "Repaired", "mechanical repair, no model call"],
          ["format(src) / canonical(src)", "str", "idempotent formatting / normalising"],
          ["capabilities(src, backend)", "Capabilities", "what it does, plus a CSP"],
          ["registry(tags)", "str", "prompt-sized vocabulary slice"],
          ["SPEC", "str", "the language spec, for a system prompt"],
        ]}
      />
      <P>
        <C>Diagnostic</C> is a frozen dataclass — <C>d.code</C>, <C>d.line</C>, <C>d.column</C>,{" "}
        <C>d.message</C>, <C>d.help</C>, <C>d.suggestion</C>, <C>d.is_error</C> — not a dict. The
        package is fully typed and ships <C>py.typed</C>.
      </P>

      <H2 id="threading">Threading</H2>
      <P>
        The API is synchronous on purpose. Compiling is CPU-bound and takes single-digit milliseconds,
        and an <C>async def</C> wrapper would be fake async that still blocks the event loop.
      </P>
      <P>
        The GIL <em>is</em> released for the duration of every compile, so Flask on threads and
        FastAPI&rsquo;s threadpool genuinely parallelise rather than serialising through the
        interpreter lock. For an unusually large document inside an async handler,{" "}
        <C>await asyncio.to_thread(guml.render, src)</C>.
      </P>

      <H2 id="cli">Command line</H2>
      <CodeBlock
        lang="bash"
        filename="terminal"
        code={`guml build app.guml --backend html
guml check app.guml
guml fmt app.guml --write
guml capabilities app.guml`}
      />
      <P>
        A subset — enough that compiling a <C>.guml</C> file needs no Rust toolchain. The full CLI
        (source maps, custom themes, registry validation, token estimates) is{" "}
        <C>cargo install guml-cli</C>; see <A href="/docs/compiler/cli">the CLI reference</A>.
      </P>
      <P>
        Also available: <Pkg name="@guml/core" /> for the browser, <Pkg name="@guml/fmt" /> and{" "}
        <Pkg name="@guml/highlight" /> for tooling.
      </P>
    </DocPage>
  );
}
