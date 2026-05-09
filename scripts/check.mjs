import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { join } from "node:path";

const svelteCheckBin = join(
  process.cwd(),
  "node_modules",
  ".bin",
  process.platform === "win32" ? "svelte-check.cmd" : "svelte-check"
);

if (!existsSync(svelteCheckBin)) {
  console.error(
    "svelte-check is not installed. Run npm install before running npm run check."
  );
  process.exit(1);
}

const result = spawnSync(svelteCheckBin, ["--tsconfig", "./tsconfig.json"], {
  stdio: "inherit",
  shell: process.platform === "win32",
});

if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}

process.exit(result.status ?? 1);
