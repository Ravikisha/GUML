import type { Metadata } from "next";
import { Chat } from "@/components/chat";
import { Meter } from "@/components/ui";
import { SYSTEM_PROMPT_EST_TOKENS } from "@/lib/prompt.generated";

export const metadata: Metadata = {
  title: "Chat",
  description:
    "Describe an interface and watch a model generate it as GUML, compiled and rendered in your browser. Runs on models hosted at build.nvidia.com.",
};

export default function Page() {
  return (
    <div className="pt-24">
      <header className="mx-auto max-w-(--container-page) px-4 pb-6 md:px-6">
        <div className="flex flex-wrap items-baseline justify-between gap-4">
          <div>
            <h1 className="display-narrow text-3xl font-medium text-chalk md:text-4xl">
              Generative UI
            </h1>
            <p className="mt-3 max-w-2xl leading-relaxed text-fog">
              A model writes the interface as GUML; the compiler builds it in your browser. This is
              the loop the whole project is an argument about — a small representation generated
              fast, then expanded by a compiler that owns the parts a model gets wrong.
            </p>
          </div>
          <div className="flex flex-wrap gap-x-8 gap-y-2">
            <Meter label="generation" value="build.nvidia.com" />
            <Meter label="prompt tax" value={`~${SYSTEM_PROMPT_EST_TOKENS} tokens`} tone="ember" />
            <Meter label="compiler" value="wasm, in this tab" tone="iris" />
          </div>
        </div>
      </header>
      <Chat />
    </div>
  );
}
