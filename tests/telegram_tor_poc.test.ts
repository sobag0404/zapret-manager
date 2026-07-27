import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const root = fileURLToPath(new URL("..", import.meta.url));
const read = (path: string) => readFileSync(join(root, path), "utf8");

describe("Telegram Tor PoC", () => {
  it("is loopback-only, control-port-free, and does not launch or download Tor", () => {
    const template = read("tools/telegram-tor-poc/torrc.template");
    const config = read("tools/telegram-tor-poc/New-TelegramTorPocConfig.ps1");

    expect(template).toContain("SocksPort 127.0.0.1:{{SOCKS_PORT}} IsolateSOCKSAuth");
    expect(template).toContain("SocksPolicy reject *");
    expect(template).toContain("ControlPort 0");
    expect(template).toContain("DNSPort 0");
    expect(template).toContain("TransPort 0");
    expect(template).toContain("SafeLogging 1");
    expect(config).toContain("telegram-tor-poc");
    expect(config).toContain("ValidateOnly");
    expect(config).toContain("SpecialFolder]::LocalApplicationData");
    expect(config).toContain("ReparsePoint");
    expect(config).not.toMatch(/Invoke-WebRequest|Start-BitsTransfer|Start-Process/i);
    expect(config).not.toMatch(/tor\.exe/i);
  });

  it("pins official provenance and rejects unsigned/hash-mismatched artifacts", () => {
    const verifier = read("tools/telegram-tor-poc/Stage-TorExpertBundle.ps1");
    const schema = JSON.parse(read("tools/telegram-tor-poc/manifest.schema.json"));

    expect(verifier).toContain("EF6E286DDA85EA2A4BA7DE684E2C6E8793298290");
    expect(verifier).toContain("Get-FileHash");
    expect(verifier).toContain("VALIDSIG");
    expect(verifier).toContain("IsPathFullyQualified");
    expect(verifier).toContain("GpgvSha256");
    expect(verifier).toContain("Archive changed during verification");
    expect(verifier).toContain("ExtractorSha256");
    expect(verifier).toContain("[IO.FileShare]::Read");
    expect(verifier).toContain("SetAccessRuleProtection($true, $false)");
    expect(verifier).toContain("pending-");
    expect(verifier).toContain("receipt.json");
    expect(verifier).toContain("Move-Item -LiteralPath $pendingRoot");
    expect(verifier).toMatch(/archive\|dist/);
    expect(verifier).not.toMatch(/Invoke-WebRequest|Start-BitsTransfer|Start-Process/i);
    expect(schema.properties.source_url.pattern).toContain("torproject");
    expect(schema.properties.signer_fingerprint.const).toBe(
      "EF6E286DDA85EA2A4BA7DE684E2C6E8793298290",
    );
  });

  it("documents routed trust and bounded Web/Desktop behavior", () => {
    const adr = read("docs/ADR-0001-TELEGRAM-TOR-POC.md");
    const threatModel = read("docs/TELEGRAM_TOR_THREAT_MODEL.md");

    expect(adr).toContain("third-party routed network");
    expect(adr).toContain("`ControlPort`, `DNSPort`, `TransPort`");
    expect(adr).toContain("not claim anonymity");
    expect(adr).toContain("does not edit Telegram Desktop private configuration files");
    expect(threatModel).toContain("DNS/IP leak");
    expect(threatModel).toContain("Version: 6c19f94d624ca2fe3f970efc28b90296f7e8d5dd");
  });
});
