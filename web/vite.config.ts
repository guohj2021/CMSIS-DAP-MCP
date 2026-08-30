import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  build: {
    outDir: "../crates/cmsis-dap-web/assets",
    emptyOutDir: true,
    rolldownOptions: {
      output: {
        // Split heavy vendor libs (dockview, react) so no single chunk is huge.
        codeSplitting: {
          groups: [
            { name: "react", test: /node_modules[\\/](react|react-dom|scheduler)/ },
            { name: "dockview", test: /node_modules[\\/](dockview|dockview-react|dockview-core)/ },
          ],
        },
      },
    },
  },
  server: {
    proxy: {
      "/api": "http://127.0.0.1:8080",
      "/ws": { target: "ws://127.0.0.1:8080", ws: true },
    },
  },
});
