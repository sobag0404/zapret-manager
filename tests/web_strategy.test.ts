import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const root = fileURLToPath(new URL("..", import.meta.url));

describe("web-only engine strategies", () => {
  for (const [name, hostlist] of [
    ["web (TELEGRAM).bat", "list-telegram-web.txt"],
    ["web (WHATSAPP).bat", "list-whatsapp-web.txt"],
  ]) {
    it(`${name} limits the existing ALT TLS profile to its web hostlist`, () => {
      const source = readFileSync(join(root, "engine/local", name), "utf8");

      expect(source.match(/winws\.exe/gi)).toHaveLength(1);
      expect(source).toContain(`--hostlist="%LISTS%${hostlist}"`);
      expect(source).toContain("--filter-tcp=443");
      expect(source).toContain("--dpi-desync=fake,fakedsplit");
      expect(source).toContain("--dpi-desync-fakedsplit-pattern=0x00");
      expect(source).not.toContain("--filter-udp=");
      expect(source).not.toContain("--ipset=");
    });
  }
});
