"use client";

import { Monitor, Moon, Sun } from "lucide-react";
import { useSyncExternalStore } from "react";
import { THEME_KEY } from "@/lib/inline-scripts";
import { cn } from "@/lib/utils";

export type Theme = "light" | "dark" | "system";

// Imported and re-exported, not `export { THEME_KEY } from "…"` — that form creates no local binding,
// and this module reads the value itself three times below.
//
// The definition moved to `lib/inline-scripts.ts`, which `next.config.ts` also imports to hash the
// inline script that reads this key. A client component cannot be imported from the Node-side config,
// so the shared constant cannot live in this file.
export { THEME_KEY };

/** Same-tab notification; the `storage` event only fires in *other* tabs. */
const THEME_EVENT = "guml-theme-change";

/**
 * Three states rather than two, because "follow my system" is a real preference
 * and a two-way switch silently overrides it forever after one click.
 *
 * The applied theme is written to `data-theme` on `<html>`; the CSS in
 * `globals.css` reads that attribute, and falls back to `prefers-color-scheme`
 * when it is absent. `system` therefore means "remove the attribute", not
 * "compute the current system value" — so a reader who changes their OS theme
 * mid-session is followed without a reload.
 */
/** The stored preference is external state, so it is read rather than mirrored. */
const store = {
  subscribe(onChange: () => void) {
    window.addEventListener("storage", onChange);
    window.addEventListener(THEME_EVENT, onChange);
    return () => {
      window.removeEventListener("storage", onChange);
      window.removeEventListener(THEME_EVENT, onChange);
    };
  },
  snapshot(): Theme {
    try {
      const stored = localStorage.getItem(THEME_KEY);
      return stored === "light" || stored === "dark" ? stored : "system";
    } catch {
      return "system";
    }
  },
  // On the server there is no preference to read, so nothing renders as active.
  server(): Theme {
    return "system";
  },
};

export function ThemeToggle() {
  // `useSyncExternalStore` rather than an effect that mirrors localStorage into
  // state: the preference already lives outside React, and copying it in would be
  // a synchronous setState inside an effect — which React 19 rightly flags.
  const theme = useSyncExternalStore(store.subscribe, store.snapshot, store.server);

  function apply(next: Theme) {
    const root = document.documentElement;
    if (next === "system") {
      root.removeAttribute("data-theme");
      localStorage.removeItem(THEME_KEY);
    } else {
      root.setAttribute("data-theme", next);
      localStorage.setItem(THEME_KEY, next);
    }
    // `storage` only fires in other tabs, so tell this one explicitly.
    window.dispatchEvent(new Event(THEME_EVENT));
  }

  const options: Array<{ value: Theme; icon: typeof Sun; label: string }> = [
    { value: "light", icon: Sun, label: "Light" },
    { value: "dark", icon: Moon, label: "Dark" },
    { value: "system", icon: Monitor, label: "System" },
  ];

  return (
    <div
      role="radiogroup"
      aria-label="Colour theme"
      className="inline-flex items-center gap-0.5 rounded-full border border-line-strong p-0.5"
    >
      {options.map(({ value, icon: Icon, label }) => {
        const active = theme === value;
        return (
          <button
            key={value}
            type="button"
            role="radio"
            aria-checked={active}
            aria-label={label}
            title={label}
            onClick={() => apply(value)}
            className={cn(
              "inline-flex size-7 items-center justify-center rounded-full transition-colors",
              active ? "bg-chalk text-ink" : "text-fog hover:text-chalk",
            )}
          >
            <Icon className="size-3.5" />
          </button>
        );
      })}
    </div>
  );
}

/**
 * Runs before first paint to apply the stored theme, so a reader who chose light
 * never sees a dark frame first. Inlined in the document head deliberately: any
 * deferred script is too late to prevent the flash.
 */
