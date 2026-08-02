import { DocsSidebar } from "@/components/docs-sidebar";

export default function DocsLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="mx-auto flex max-w-(--container-page) gap-12 px-6 py-12 md:px-10">
      <aside className="sticky top-24 hidden h-[calc(100dvh-8rem)] w-56 shrink-0 overflow-y-auto pr-2 lg:block">
        <DocsSidebar />
      </aside>
      <div className="min-w-0 flex-1">{children}</div>
    </div>
  );
}
