import type { Config } from 'tailwindcss';
import { createPreset } from 'fumadocs-ui/tailwind-plugin';

const config: Config = {
  content: [
    './node_modules/fumadocs-ui/dist/**/*.js',
    './app/**/*.{ts,tsx}',
    './content/**/*.{md,mdx}',
    './mdx-components.{ts,tsx}',
  ],
  presets: [createPreset()],
};

export default config;
