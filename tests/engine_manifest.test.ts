import { createHash } from "node:crypto";
import { existsSync, readFileSync, statSync } from "node:fs";
import { isAbsolute, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const root = fileURLToPath(new URL("..", import.meta.url));
const engineRoot = resolve(root, "engine/local");

describe("engine manifest", () => {
  it("matches bundled engine files and hashes", () => {
    const manifest = JSON.parse(readFileSync(join(root, "engine/manifest.json"), "utf8"));
    expect(manifest.schema_version).toBe("1");
    expect(manifest.files.length).toBeGreaterThan(0);

    const seen = new Set<string>();
    for (const file of manifest.files) {
      expect(file.relative_path).not.toContain("..");
      expect(isAbsolute(file.relative_path)).toBe(false);
      expect(file.sha256).toMatch(/^[a-fA-F0-9]{64}$/);
      expect(seen.has(file.relative_path)).toBe(false);
      seen.add(file.relative_path);

      const path = resolve(engineRoot, file.relative_path);
      expect(path.startsWith(engineRoot)).toBe(true);
      expect(existsSync(path), `${file.relative_path} missing`).toBe(true);
      expect(statSync(path).isFile(), `${file.relative_path} is not a file`).toBe(true);

      const hash = createHash("sha256").update(readFileSync(path)).digest("hex");
      expect(hash, file.relative_path).toBe(file.sha256);
    }
  });
});
