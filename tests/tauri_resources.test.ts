import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const root = fileURLToPath(new URL("..", import.meta.url));

describe("tauri resources", () => {
  it("packages engine, profiles and strategies into the installer resources", () => {
    const config = JSON.parse(readFileSync(join(root, "app/tauri/tauri.conf.json"), "utf8"));
    expect(config.bundle.resources).toMatchObject({
      "../../engine": "engine",
      "../../profiles": "profiles",
      "../../strategies": "strategies",
    });
  });
});
