import path from "node:path";
import { fileURLToPath } from "node:url";

const landingDir = path.dirname(fileURLToPath(import.meta.url));

/** @type {import('next').NextConfig} */
const nextConfig = {
  outputFileTracingRoot: landingDir,
};

export default nextConfig;
