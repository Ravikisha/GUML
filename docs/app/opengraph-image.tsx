import { ImageResponse } from "next/og";
import { FIXTURES } from "@/lib/fixtures.generated";

export const alt = "GUML — an intermediate representation and compiler for LLM-generated interfaces";
export const size = { width: 1200, height: 630 };
export const contentType = "image/png";

/**
 * The social card.
 *
 * It shows the one number the project is an argument about, taken from the generated fixture
 * data rather than typed in — a share card quoting a token count that no longer matches the
 * measurement would be the most public possible version of the drift this repo keeps fixing.
 *
 * Rendered by Satori, which supports a flexbox subset: every container that holds more than one
 * child declares `display: flex` explicitly, and there is no grid.
 */
export default function Image() {
  const tasks = FIXTURES.find((f) => f.id === "tasks") ?? FIXTURES[0];

  return new ImageResponse(
    (
      <div
        style={{
          width: "100%",
          height: "100%",
          display: "flex",
          flexDirection: "column",
          justifyContent: "space-between",
          background: "#1a0400",
          color: "#fbeeea",
          padding: "72px 80px",
          fontFamily: "monospace",
        }}
      >
        <div style={{ display: "flex", flexDirection: "column" }}>
          <div style={{ fontSize: 26, letterSpacing: 6, color: "#c3a49c" }}>GUML</div>
          <div style={{ fontSize: 62, lineHeight: 1.1, marginTop: 28, maxWidth: 900 }}>
            An IR and compiler for LLM-generated interfaces
          </div>
        </div>

        <div style={{ display: "flex", alignItems: "flex-end", justifyContent: "space-between" }}>
          <div style={{ display: "flex", flexDirection: "column" }}>
            <div style={{ fontSize: 22, color: "#c3a49c", letterSpacing: 3 }}>
              SAME APP, TWO REPRESENTATIONS
            </div>
            <div style={{ display: "flex", alignItems: "baseline", marginTop: 18 }}>
              <span style={{ fontSize: 76, color: "#c3a49c" }}>
                {tasks.tokens.react.toLocaleString()}
              </span>
              <span style={{ fontSize: 34, color: "#9a7d75", margin: "0 22px" }}>→</span>
              <span style={{ fontSize: 76, color: "#ff6a4d" }}>{tasks.tokens.guml}</span>
              <span style={{ fontSize: 26, color: "#c3a49c", marginLeft: 18 }}>tokens</span>
            </div>
          </div>
          <div style={{ fontSize: 22, color: "#9a7d75" }}>guml.vercel.app</div>
        </div>
      </div>
    ),
    size,
  );
}
