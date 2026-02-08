import type { NextConfig } from 'next';

const nextConfig: NextConfig = {
  reactStrictMode: false,
  turbopack: {
    root: __dirname,
  },
};

export default nextConfig;
