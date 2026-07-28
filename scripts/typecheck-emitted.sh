#!/usr/bin/env bash
# Typecheck the compiler's own output.
#
# The strongest possible test of a code generator: emit every fixture, then run
# `tsc --strict` over the result. Two real bugs were found this way — a missing
# JSX fragment around multi-root pages, and a layout attribute emitted as a DOM
# prop — neither of which any Rust-side assertion would have caught.
set -euo pipefail

out="${TMPDIR:-/tmp}/guml-emitted"
rm -rf "$out" && mkdir -p "$out"

for f in fixtures/*.guml; do
  cargo run -q -p guml-cli -- build "$f" -o "$out" >/dev/null
done

cat > "$out/tsconfig.json" <<'JSON'
{
  "compilerOptions": {
    "target": "ES2022", "lib": ["ES2022", "DOM"], "jsx": "react-jsx",
    "module": "ESNext", "moduleResolution": "bundler",
    "strict": true, "noEmit": true, "skipLibCheck": true
  },
  "include": ["*.tsx"]
}
JSON

echo "typechecking $(ls "$out"/*.tsx | wc -l) emitted components…"
(cd docs && node node_modules/typescript/bin/tsc -p "$out/tsconfig.json" --typeRoots node_modules/@types)
echo "emitted output typechecks under --strict"
