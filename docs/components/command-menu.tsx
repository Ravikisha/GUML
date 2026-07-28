"use client";

import { Command } from "cmdk";
import { ArrowRight, FileCode2, Hash, Terminal } from "lucide-react";
import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";
import { NAV } from "@/lib/nav";
import { Kbd } from "./ui";

const EXTRAS = [
  { title: "Examples gallery", href: "/examples", icon: FileCode2 },
  { title: "CLI reference", href: "/docs/compiler/cli", icon: Terminal },
];

export function CommandMenu() {
  const [open, setOpen] = useState(false);
  const router = useRouter();

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "k" && (e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        setOpen((v) => !v);
      }
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, []);

  function go(href: string) {
    setOpen(false);
    router.push(href);
  }

  return (
    <>
      <button
        type="button"
        onClick={() => setOpen(true)}
        className="hidden items-center gap-2 rounded-full border border-line-strong px-3 py-1.5 text-sm text-fog transition-colors hover:border-chalk/30 hover:text-chalk sm:inline-flex"
      >
        <span>Search docs</span>
        <Kbd>⌘K</Kbd>
      </button>

      <Command.Dialog
        open={open}
        onOpenChange={setOpen}
        label="Search documentation"
        className="fixed inset-0 z-100 grid place-items-start justify-center bg-ink/80 pt-[12vh] backdrop-blur-sm"
      >
        <div className="w-[min(92vw,34rem)] overflow-hidden rounded-panel border border-line-strong bg-ink-raised shadow-[0_30px_80px_-40px_rgb(0_0_0/0.9)]">
          <Command.Input
            placeholder="Jump to a page…"
            className="w-full border-b border-line bg-transparent px-4 py-3.5 font-mono text-sm text-chalk outline-none placeholder:text-fog-dim"
          />
          <Command.List className="max-h-[52vh] overflow-y-auto p-2">
            <Command.Empty className="px-3 py-6 text-center font-mono text-xs text-fog-dim">
              Nothing matches. Try “registry”, “diagnostics” or “phase 0”.
            </Command.Empty>

            {NAV.map((group) => (
              <Command.Group
                key={group.title}
                heading={group.title}
                className="[&_[cmdk-group-heading]]:label [&_[cmdk-group-heading]]:px-3 [&_[cmdk-group-heading]]:py-2"
              >
                {group.items.map((item) => (
                  <Command.Item
                    key={item.href}
                    value={`${group.title} ${item.title}`}
                    onSelect={() => go(item.href)}
                    className="flex cursor-pointer items-center gap-2.5 rounded-chip px-3 py-2 text-sm text-fog data-[selected=true]:bg-chalk/8 data-[selected=true]:text-chalk"
                  >
                    <Hash className="size-3.5 text-fog-dim" />
                    <span>{item.title}</span>
                    <ArrowRight className="ml-auto size-3.5 opacity-0 data-[selected=true]:opacity-100" />
                  </Command.Item>
                ))}
              </Command.Group>
            ))}

            <Command.Group
              heading="More"
              className="[&_[cmdk-group-heading]]:label [&_[cmdk-group-heading]]:px-3 [&_[cmdk-group-heading]]:py-2"
            >
              {EXTRAS.map(({ title, href, icon: Icon }) => (
                <Command.Item
                  key={href}
                  value={title}
                  onSelect={() => go(href)}
                  className="flex cursor-pointer items-center gap-2.5 rounded-chip px-3 py-2 text-sm text-fog data-[selected=true]:bg-chalk/8 data-[selected=true]:text-chalk"
                >
                  <Icon className="size-3.5 text-fog-dim" />
                  {title}
                </Command.Item>
              ))}
            </Command.Group>
          </Command.List>
        </div>
      </Command.Dialog>
    </>
  );
}
