import type { NextConfig } from "next";
import { fileURLToPath } from "node:url";

const nextConfig: NextConfig = {
  // This app lives inside the GUML Rust repo, so Turbopack would otherwise walk
  // up and pick a lockfile from a parent directory as the workspace root.
  turbopack: {
    // The docs app lives inside the GUML repo next to the `guml` workspace
    // package, so the root has to include both.
    root: fileURLToPath(new URL("..", import.meta.url)),
  },

  // `guml` ships TypeScript source plus a wasm module, so Next compiles it
  // rather than treating it as a prebuilt dependency.
  transpilePackages: ["guml"],
};

export default nextConfig;
