import { createMDX } from 'fumadocs-mdx/next';

const withMDX = createMDX();

// NEXT_PUBLIC_BASE_PATH is injected by actions/configure-pages when deploying
// to GitHub Pages (e.g. "/atom" for a project page at github.io/atom).
const basePath = process.env.NEXT_PUBLIC_BASE_PATH ?? '';

/** @type {import('next').NextConfig} */
const config = {
  reactStrictMode: true,
  output: 'export',
  trailingSlash: true,
  ...(basePath && {
    basePath,
    assetPrefix: basePath,
  }),
};

export default withMDX(config);
