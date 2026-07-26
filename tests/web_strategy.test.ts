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

  it("Telegram Phase 0 uses the official Telegram CIDRs before TLS is visible", () => {
    const source = readFileSync(join(root, "engine/local", "web (TELEGRAM PHASE0).bat"), "utf8");
    const ipset = readFileSync(join(root, "engine/local/lists", "ipset-telegram-phase0.txt"), "utf8");

    expect(source.match(/winws\.exe/gi)).toHaveLength(1);
    expect(source).toContain("--wf-tcp=443");
    expect(source).toContain("--filter-tcp=443");
    expect(source).toContain('--ipset="%LISTS%ipset-telegram-phase0.txt"');
    expect(source).toContain("--dpi-desync=syndata,fake,fakedsplit");
    expect(source).not.toContain("--hostlist=");
    expect(source).not.toContain("--filter-udp=");
    expect(source).not.toContain("--wsize=");

    expect(ipset.trim().split(/\r?\n/)).toEqual([
      "91.108.56.0/22",
      "91.108.4.0/22",
      "91.108.8.0/22",
      "91.108.16.0/22",
      "91.108.12.0/22",
      "149.154.160.0/20",
      "91.105.192.0/23",
      "91.108.20.0/22",
      "185.76.151.0/24",
      "2001:b28:f23d::/48",
      "2001:b28:f23f::/48",
      "2001:67c:4e8::/48",
      "2001:b28:f23c::/48",
      "2a0a:f280::/32",
    ]);
  });
});
