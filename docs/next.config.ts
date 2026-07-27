import type { NextConfig } from "next";
import { fileURLToPath } from "node:url";

const nextConfig: NextConfig = {
  // This app lives inside the GUML Rust repo, so Turbopack would otherwise walk
  // up and pick a lockfile from a parent directory as the workspace root.
  turbopack: {
    root: fileURLToPath(new URL(".", import.meta.url)),
  },
};

export default nextConfig;
