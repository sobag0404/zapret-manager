import Ajv2020 from "ajv/dist/2020";
import { describe, expect, it } from "vitest";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));

describe("strategy schema", () => {
  it("validates bundled safe stub strategies", () => {
    const schema = JSON.parse(readFileSync(join(root, "strategies/strategy.schema.json"), "utf8"));
    const ajv = new Ajv2020({ strict: false, validateFormats: false });
    const validate = ajv.compile(schema);
    for (const channel of ["stable", "experimental"]) {
      for (const file of readdirSync(join(root, "strategies", channel)).filter((name) => name.endsWith(".json"))) {
        const data = JSON.parse(readFileSync(join(root, "strategies", channel, file), "utf8"));
        expect(validate(data), JSON.stringify(validate.errors)).toBe(true);
        expect(data.notes).toBe("No real low-level parameters in scaffold");
      }
    }
  });
});
