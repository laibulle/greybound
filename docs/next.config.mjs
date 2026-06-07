import { createMDX } from 'fumadocs-mdx/next';

/** @type {import('next').NextConfig} */
const config = {
  reactStrictMode: true,
  webpack(nextConfig) {
    nextConfig.module.rules.push({
      test: /\.json5$/,
      type: 'asset/source',
    });
    return nextConfig;
  },
};

const withMDX = createMDX();

export default withMDX(config);
