import type { NextConfig } from 'next';

const isGitHubPages = process.env.GITHUB_PAGES === 'true';

const nextConfig: NextConfig = {
  output: isGitHubPages ? 'export' : undefined,
  basePath: isGitHubPages ? '/silkprint' : '',
  turbopack: {
    root: __dirname,
  },
};

export default nextConfig;
