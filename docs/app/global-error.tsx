"use client";

import { useEffect } from "react";
import { reportError } from "@/lib/report-error";

/**
 * The last resort: an error thrown in the root layout itself.
 *
 * `error.tsx` renders *inside* the root layout, so it cannot catch a failure in the layout that would
 * render it. This one replaces the entire document, which is why it has to supply its own `<html>` and
 * `<body>` — and why it cannot use the site's components, fonts or CSS variables. Anything it imported
 * could be the thing that broke.
 *
 * Hence the inline styles. They are not a shortcut; a stylesheet is exactly the dependency this page
 * cannot assume survived.
 */
export default function GlobalError({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  useEffect(() => {
    reportError(error, { boundary: "root", digest: error.digest });
  }, [error]);

  return (
    <html lang="en">
      <body
        style={{
          margin: 0,
          minHeight: "100vh",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          background: "#0b0b0e",
          color: "#e8e8ea",
          fontFamily:
            "ui-sans-serif, system-ui, -apple-system, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif",
          padding: "2rem",
        }}
      >
        <main style={{ maxWidth: "36rem" }}>
          <p
            style={{
              margin: 0,
              fontSize: "0.75rem",
              letterSpacing: "0.08em",
              textTransform: "uppercase",
              color: "#8a8a94",
            }}
          >
            error
          </p>
          <h1 style={{ margin: "0.75rem 0 0", fontSize: "2rem", fontWeight: 500 }}>
            GUML failed to load
          </h1>
          <p style={{ margin: "1.25rem 0 0", lineHeight: 1.7, color: "#a8a8b2" }}>
            Something failed before the page could render. Reloading usually resolves it.
          </p>
          {error.digest ? (
            <p
              style={{
                margin: "1.5rem 0 0",
                fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace",
                fontSize: "0.75rem",
                color: "#8a8a94",
              }}
            >
              reference {error.digest}
            </p>
          ) : null}
          <button
            type="button"
            onClick={reset}
            style={{
              marginTop: "2rem",
              padding: "0.6rem 1.1rem",
              borderRadius: "0.375rem",
              border: 0,
              background: "#e8e8ea",
              color: "#0b0b0e",
              fontSize: "0.875rem",
              fontWeight: 500,
              cursor: "pointer",
            }}
          >
            Reload
          </button>
        </main>
      </body>
    </html>
  );
}
