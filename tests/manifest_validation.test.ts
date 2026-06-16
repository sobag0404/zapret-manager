import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const root = fileURLToPath(new URL("..", import.meta.url));

describe("strategy manifest", () => {
  it("matches strategy file hashes and consent rules", () => {
    const manifest = JSON.parse(readFileSync(join(root, "strategies/manifest.json"), "utf8"));
    expect(manifest.schema_version).toBe("1");
    for (const entry of manifest.entries) {
      const body = readFileSync(join(root, "strategies", entry.path));
      const hash = createHash("sha256").update(body).digest("hex");
      expect(entry.sha256).toBe(hash);
      if (entry.channel === "experimental") {
        expect(entry.experimental_requires_consent).toBe(true);
      }
      expect(entry.trusted_source).toMatch(/^file:\/\/\/local\/strategies\//);
    }
  });
});
