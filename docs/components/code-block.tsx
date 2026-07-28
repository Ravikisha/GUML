import { CLASS_STYLE, highlight, type Lang } from "@/lib/highlight";
import { cn } from "@/lib/utils";
import { CopyButton } from "./copy-button";

export function CodeBlock({
  code,
  lang = "guml",
  filename,
  meter,
  lines = false,
  className,
  maxHeight,
}: {
  code: string;
  lang?: Lang;
  filename?: string;
  /** Right-aligned readout: token count, line count, whatever is true. */
  meter?: string;
  lines?: boolean;
  className?: string;
  maxHeight?: number;
}) {
  const rows = highlight(code, lang);
  const gutter = String(rows.length).length;

  return (
    <figure
      className={cn(
        "group relative overflow-hidden rounded-card border border-line bg-code code-surface",
        className,
      )}
    >
      {(filename || meter) && (
        <figcaption className="flex items-center justify-between gap-4 border-b border-line bg-chalk/[0.02] px-4 py-2.5">
          <span className="font-mono text-xs text-fog">{filename}</span>
          <span className="flex items-center gap-3">
            {meter ? <span className="label">{meter}</span> : null}
            <CopyButton text={code} />
          </span>
        </figcaption>
      )}
      {!filename && !meter && (
        <CopyButton
          text={code}
          className="absolute top-3 right-3 z-10 opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100"
        />
      )}
      <div className="overflow-auto" style={maxHeight ? { maxHeight } : undefined}>
        <pre className="w-max min-w-full px-4 py-4 font-mono text-[0.82rem] leading-[1.65]">
          <code>
            {rows.map((row, i) => (
              <span key={i} className="block">
                {lines && (
                  <span
                    className="mr-4 inline-block select-none text-right text-syn-comment"
                    style={{ width: `${gutter}ch` }}
                  >
                    {i + 1}
                  </span>
                )}
                {row.length === 0 ? (
                  <span> </span>
                ) : (
                  row.map((tok, j) => (
                    // `tok.cls` is the compiler's class name; colour is looked up here so
                    // the classifier and its parity check never deal in CSS.
                    <span key={j} className={CLASS_STYLE[tok.cls]}>
                      {tok.text}
                    </span>
                  ))
                )}
              </span>
            ))}
          </code>
        </pre>
      </div>
    </figure>
  );
}
