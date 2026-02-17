import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "path"
import tailwindcss from "@tailwindcss/vite"

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async () => ({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },

  envPrefix: ["VITE_", "TAURI_"],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },

  build: {
    rollupOptions: {
      output: {
        manualChunks: {
          ui: ['radix-ui', 'lucide-react'],
          router: ['@tanstack/react-router'],
        },
      },
    },
  },
}));
