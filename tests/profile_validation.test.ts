import Ajv2020 from "ajv/dist/2020";
import { describe, expect, it } from "vitest";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));

describe("profile schema", () => {
  it("validates bundled profiles", () => {
    const schema = JSON.parse(readFileSync(join(root, "profiles/profile.schema.json"), "utf8"));
    const ajv = new Ajv2020({ strict: false });
    const validate = ajv.compile(schema);
    for (const file of readdirSync(join(root, "profiles")).filter((name) => name.endsWith(".json") && !name.includes("schema"))) {
      const data = JSON.parse(readFileSync(join(root, "profiles", file), "utf8"));
      expect(validate(data), JSON.stringify(validate.errors)).toBe(true);
      expect(data.notes).toContain("No low-level");
    }
  });
});
