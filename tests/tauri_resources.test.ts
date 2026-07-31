import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const root = fileURLToPath(new URL("..", import.meta.url));

describe("tauri resources", () => {
  it("packages only the approved Discord/YouTube engine subset", () => {
    const config = JSON.parse(readFileSync(join(root, "app/tauri/tauri.conf.json"), "utf8"));
    const resources = config.bundle.resources as Record<string, string>;
    expect(resources).toMatchObject({
      "../../profiles": "profiles",
      "../../strategies": "strategies",
      "../../engine/manifest.json": "engine/manifest.json",
      "../../engine/local/general (ALT).bat": "engine/local/general (ALT).bat",
      "../../engine/local/general (FAKE TLS AUTO).bat": "engine/local/general (FAKE TLS AUTO).bat",
    });
    expect(resources["../../engine"]).toBeUndefined();
    expect(Object.keys(resources).join("\n")).not.toMatch(/telegram|whatsapp|common/i);

    const manifest = JSON.parse(readFileSync(join(root, "engine/manifest.json"), "utf8"));
    const bundledEngineFiles = Object.keys(resources)
      .filter((source) => source.startsWith("../../engine/local/"))
      .map((source) => source.slice("../../engine/local/".length))
      .sort();
    const manifestFiles = manifest.files.map((file: { relative_path: string }) => file.relative_path).sort();

    expect(bundledEngineFiles).toEqual(manifestFiles);
  });

  it("removes retired product files on upgrade without touching user data", () => {
    const hooks = readFileSync(join(root, "app/tauri/nsis-hooks.nsh"), "utf8");
    expect(hooks).toContain("!macro NSIS_HOOK_PREINSTALL");
    expect(hooks).toContain('Delete "$INSTDIR\\engine\\local\\web (TELEGRAM).bat"');
    expect(hooks).toContain('Delete "$INSTDIR\\profiles\\whatsapp.json"');
    expect(hooks).not.toMatch(/RMDir \/r|Delete \"\$(?:APPDATA|LOCALAPPDATA)/i);
  });
});
