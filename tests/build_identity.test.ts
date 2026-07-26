import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const root = fileURLToPath(new URL("..", import.meta.url));

describe("release build identity", () => {
  it("uses an explicit clean Git id for local packaging and CI", () => {
    const buildRs = readFileSync(join(root, "app/tauri/build.rs"), "utf8");
    const packageScript = readFileSync(join(root, "scripts/package.ps1"), "utf8");
    const workflow = readFileSync(join(root, ".github/workflows/build-windows.yml"), "utf8");

    expect(buildRs).toContain("cargo:rerun-if-env-changed=ZAPRET_MANAGER_BUILD_ID");
    expect(buildRs).toContain("fn explicit_build_id() -> Option<String>");
    expect(packageScript).toContain("status --porcelain");
    expect(packageScript).toContain("Refusing to package a dirty worktree");
    expect(packageScript).toContain("$env:ZAPRET_MANAGER_BUILD_ID");
    expect(workflow).toContain("ZAPRET_MANAGER_BUILD_ID: ${{ github.sha }}");
  });
});
