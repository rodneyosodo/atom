import react from "@astrojs/react";
import { defineConfig } from "astro/config";

const backendUrl = process.env.ATOM_BACKEND_URL ?? "http://localhost:8080";

export default defineConfig({
  base: "/graphql/console",
  output: "static",
  integrations: [react()],
  server: {
    host: "localhost",
    port: 4321,
  },
  vite: {
    server: {
      proxy: {
        "^/graphql$": {
          target: backendUrl,
          changeOrigin: true,
        },
        "^/api/custom/.*": {
          target: backendUrl,
          changeOrigin: true,
        },
        "^/auth/.*": {
          target: backendUrl,
          changeOrigin: true,
        },
      },
    },
  },
});
