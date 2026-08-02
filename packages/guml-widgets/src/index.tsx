/**
 * The five components GUML deliberately does not ship as builtins.
 *
 * # Why these are a package and not part of the compiler
 *
 * `chart`, `calendar`, `date`, `upload` and `command` were left out of the builtin vocabulary on a specific
 * argument: no honest lowering exists without a design decision the registry should not make for a host. A
 * `chart` that emitted a bare `<div>` would be a promise the compiler does not keep, and there is no neutral
 * answer to "which charting library" or "which date picker". Those belong to whoever builds the application.
 *
 * So the compiler emits `<Chart …>` plus an import and does not pretend to know what a chart is. That is what
 * a PascalCase `element` in a registry entry means, and this file is the other half of it.
 *
 * # What these are and are not
 *
 * Small, real, dependency-free reference implementations — enough to make the package *provable* rather than
 * illustrative, and to give a host something to replace rather than something to write. A production chart
 * wants a real library; a production command palette wants focus trapping and keyboard navigation beyond
 * what is here. Each component says what it leaves out.
 *
 * What they are not is decorative. The accessibility contract each registry entry declares is enforced by
 * the compiler — `chart` and `calendar` are `requires_label`, so a document that omits the name is
 * `GUML0050`, an error — and these implementations honour it: the name reaches the accessible name, not just
 * a visual caption.
 */
import { useId, useMemo, useState } from "react";

/* --------------------------------------------------------------------------- chart */

export interface ChartProps {
  /** Accessible name. `chart "Revenue by month"` — required by the registry entry, so it is not optional. */
  "aria-label": string;
  rows: ReadonlyArray<Record<string, unknown>>;
  /** Row field to plot. */
  of: string;
  /** Row field naming each point, for the table alternative below. */
  label?: string;
  kind?: "bar" | "line";
  className?: string;
}

/**
 * A chart, as an SVG plus a visually-hidden table.
 *
 * The table is the point rather than a nicety: an SVG is opaque to a screen reader, and `role="img"` with a
 * label says *that* there is a chart without conveying any of it. A real library will not do this for you,
 * which is one reason to keep this component around as the thing a host replaces deliberately.
 */
export function Chart({ rows, of, label, kind = "bar", className, ...rest }: ChartProps) {
  const values = rows.map((r) => Number(r[of]) || 0);
  const max = Math.max(1, ...values);
  const width = 240;
  const height = 64;
  const step = rows.length > 0 ? width / rows.length : width;

  const points = values
    .map((v, i) => `${i * step + step / 2},${height - (v / max) * height}`)
    .join(" ");

  return (
    <figure className={className} style={{ margin: 0 }}>
      <svg
        role="img"
        aria-label={rest["aria-label"]}
        viewBox={`0 0 ${width} ${height}`}
        width="100%"
        height={height}
      >
        {kind === "line" ? (
          <polyline fill="none" stroke="currentColor" strokeWidth="2" points={points} />
        ) : (
          values.map((v, i) => (
            <rect
              key={i}
              x={i * step + step * 0.15}
              y={height - (v / max) * height}
              width={step * 0.7}
              height={(v / max) * height}
              fill="currentColor"
            />
          ))
        )}
      </svg>
      {/* The data, for anything that cannot read an SVG. Visually hidden, not `display: none` — the latter
          removes it from the accessibility tree as well, which would defeat the purpose. */}
      <figcaption
        style={{
          position: "absolute",
          width: 1,
          height: 1,
          overflow: "hidden",
          clip: "rect(0 0 0 0)",
          whiteSpace: "nowrap",
        }}
      >
        <table>
          <caption>{rest["aria-label"]}</caption>
          <tbody>
            {rows.map((r, i) => (
              <tr key={i}>
                <th scope="row">{label ? String(r[label] ?? i + 1) : i + 1}</th>
                <td>{values[i]}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </figcaption>
    </figure>
  );
}

/* ------------------------------------------------------------------------ calendar */

export interface CalendarProps {
  "aria-label": string;
  /** Selected ISO date, `YYYY-MM-DD`. */
  value?: string;
  onChange?: (iso: string) => void;
  min?: string;
  max?: string;
  className?: string;
}

/**
 * A month grid.
 *
 * `<table role="grid">` rather than a pile of buttons, because a date grid *is* tabular: the column header
 * tells a screen reader which weekday a cell is, and that is information a flat list cannot carry.
 *
 * Left out: keyboard arrow navigation between cells, and month-to-month paging by keyboard alone. A
 * production picker needs both.
 */
export function Calendar({ value, onChange, min, max, className, ...rest }: CalendarProps) {
  const selected = value ?? "";
  const [year, month] = useMemo(() => {
    const d = selected ? new Date(`${selected}T00:00:00`) : new Date(0);
    return [d.getUTCFullYear(), d.getUTCMonth()];
  }, [selected]);

  const first = new Date(Date.UTC(year, month, 1));
  const days = new Date(Date.UTC(year, month + 1, 0)).getUTCDate();
  const offset = first.getUTCDay();
  const cells: Array<string | null> = [
    ...Array.from({ length: offset }, () => null),
    ...Array.from({ length: days }, (_, i) => {
      const day = String(i + 1).padStart(2, "0");
      return `${year}-${String(month + 1).padStart(2, "0")}-${day}`;
    }),
  ];
  const weeks = Array.from({ length: Math.ceil(cells.length / 7) }, (_, w) =>
    cells.slice(w * 7, w * 7 + 7),
  );
  const outside = (iso: string) => (min !== undefined && iso < min) || (max !== undefined && iso > max);

  return (
    <table role="grid" aria-label={rest["aria-label"]} className={className}>
      <thead>
        <tr>
          {["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"].map((d) => (
            <th key={d} scope="col" abbr={d}>
              {d}
            </th>
          ))}
        </tr>
      </thead>
      <tbody>
        {weeks.map((week, w) => (
          <tr key={w}>
            {week.map((iso, d) => (
              <td key={d}>
                {iso === null ? null : (
                  <button
                    type="button"
                    aria-pressed={iso === selected}
                    disabled={outside(iso)}
                    onClick={() => onChange?.(iso)}
                  >
                    {Number(iso.slice(8))}
                  </button>
                )}
              </td>
            ))}
          </tr>
        ))}
      </tbody>
    </table>
  );
}

/* ---------------------------------------------------------------------- date field */

export interface DateFieldProps {
  "aria-label": string;
  value?: string;
  onChange?: (iso: string) => void;
  min?: string;
  max?: string;
  className?: string;
}

/**
 * `<input type="date">`, and deliberately nothing more.
 *
 * The native control is keyboard-accessible, localised and understood by every assistive technology, which
 * no hand-built picker manages. It is in the package rather than the compiler only because the *decision* to
 * use the native one is the host's — a design system that wants its own picker replaces this component and
 * changes nothing about the document.
 */
export function DateField({ value, onChange, min, max, className, ...rest }: DateFieldProps) {
  return (
    <input
      type="date"
      className={className}
      aria-label={rest["aria-label"]}
      value={value ?? ""}
      min={min}
      max={max}
      onChange={(e) => onChange?.(e.target.value)}
    />
  );
}

/* -------------------------------------------------------------------------- upload */

export interface UploadProps {
  "aria-label": string;
  accept?: string;
  multiple?: boolean;
  onChange?: (files: FileList | null) => void;
  className?: string;
}

/**
 * A file picker, as a real `<input type="file">` with a `<label>`.
 *
 * The usual version of this is a styled `<div>` with a hidden input and a click handler, which loses keyboard
 * operation and the accessible name at once. A label wrapping the input keeps both and still styles freely.
 *
 * Left out: drag-and-drop, progress, and retry. All three are host concerns — an upload's destination is not
 * something a markup language should decide.
 */
export function Upload({ accept, multiple, onChange, className, ...rest }: UploadProps) {
  const id = useId();
  return (
    <span className={className}>
      <label htmlFor={id}>{rest["aria-label"]}</label>{" "}
      <input
        id={id}
        type="file"
        accept={accept}
        multiple={multiple}
        onChange={(e) => onChange?.(e.target.files)}
      />
    </span>
  );
}

/* --------------------------------------------------------------------- command menu */

export interface CommandMenuProps {
  "aria-label": string;
  rows: ReadonlyArray<Record<string, unknown>>;
  /** Row field holding the visible text of each command. */
  label: string;
  onSelect?: (row: Record<string, unknown>) => void;
  className?: string;
}

/**
 * A filterable palette, as a labelled dialog over a listbox.
 *
 * `role="dialog"` with `aria-modal` and `aria-label` is the part the compiler's registry entry promises; the
 * filter is a plain controlled input so the whole thing is one render path.
 *
 * Left out, and it matters: **focus trapping** and restore-on-close. A modal that does not trap focus lets a
 * keyboard user tab into the page behind it, and doing that correctly needs more than this file should carry.
 * A host shipping this to production replaces it or wraps it in a focus-trap.
 */
export function CommandMenu({ rows, label, onSelect, className, ...rest }: CommandMenuProps) {
  const [query, setQuery] = useState("");
  const id = useId();
  const needle = query.trim().toLowerCase();
  const matches =
    needle === ""
      ? rows
      : rows.filter((r) => String(r[label] ?? "").toLowerCase().includes(needle));

  return (
    <div role="dialog" aria-modal="true" aria-label={rest["aria-label"]} className={className}>
      <label htmlFor={id}>Filter</label>{" "}
      <input
        id={id}
        type="text"
        role="combobox"
        aria-expanded="true"
        aria-controls={`${id}-list`}
        value={query}
        onChange={(e) => setQuery(e.target.value)}
      />
      <ul id={`${id}-list`} role="listbox" aria-label={rest["aria-label"]}>
        {matches.map((row, i) => (
          <li key={i} role="option" aria-selected="false">
            <button type="button" onClick={() => onSelect?.(row)}>
              {String(row[label] ?? "")}
            </button>
          </li>
        ))}
        {matches.length === 0 ? <li role="option" aria-selected="false">Nothing matches.</li> : null}
      </ul>
    </div>
  );
}
