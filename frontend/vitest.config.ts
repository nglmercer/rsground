import { defineConfig } from "vitest/config";
import { fileURLToPath } from "node:url";

const frontendRoot = fileURLToPath(new URL(".", import.meta.url));

export default defineConfig({
  resolve: {
    conditions: ["browser", "development"],
    alias: {
      "@constants": `${frontendRoot}/src/constants.ts`,
    },
  },
  test: {
    environment: "node",
    environmentOptions: {
      jsdom: {
        url: "http://localhost",
      },
    },
    include: ["src/**/*.test.ts"],
    coverage: {
      provider: "v8",
      reporter: ["text", "html"],
      include: ["src/**/*.ts"],
      exclude: ["src/**/*.d.ts", "src/vite-env.d.ts", "src/**/*.test.ts"],
    },
  },
});
