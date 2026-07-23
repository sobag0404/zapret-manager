import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const root = fileURLToPath(new URL("..", import.meta.url));
const harnessFiles = [
  "tools/remote-test/Start-ZapretManagerCdp.ps1",
  "tools/remote-test/Stop-ZapretManagerCdp.ps1",
  "tools/remote-test/Export-ZapretManagerRemoteDiagnostics.ps1",
  "docs/REMOTE_TESTING.md",
];

describe("remote test harness", () => {
  it("keeps CDP loopback-only and avoids machine-specific secrets", () => {
    for (const relativePath of harnessFiles) {
      const content = readFileSync(join(root, relativePath), "utf8");

      expect(content, relativePath).not.toMatch(/ghp_[A-Za-z0-9_]+/);
      expect(content, relativePath).not.toMatch(/(?:password|passwd|token|secret)\s*=/i);
      expect(content, relativePath).not.toMatch(/\b(?!127\.0\.0\.1\b)(?:\d{1,3}\.){3}\d{1,3}\b/);
    }

    const startScript = readFileSync(join(root, "tools/remote-test/Start-ZapretManagerCdp.ps1"), "utf8");
    expect(startScript).toContain("--remote-debugging-address=127.0.0.1");
    expect(startScript).toContain("--remote-debugging-port=$CdpPort");
    expect(startScript).toContain("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS");
    expect(startScript).toContain("[Security.Principal.WindowsIdentity]::GetCurrent()");
    expect(startScript).not.toContain("$env:USERDOMAIN\\$env:USERNAME");
    expect(startScript).toContain("ValidateOnly");
  });

  it("scopes cleanup to Zapret Manager runtime instead of killing arbitrary winws", () => {
    const stopScript = readFileSync(join(root, "tools/remote-test/Stop-ZapretManagerCdp.ps1"), "utf8");
    const exportScript = readFileSync(join(root, "tools/remote-test/Export-ZapretManagerRemoteDiagnostics.ps1"), "utf8");

    expect(stopScript).toContain("engine-runtime");
    expect(stopScript).toContain("CleanupScopedWinws");
    expect(stopScript).toContain("Refusing to stop PID");
    expect(stopScript).not.toMatch(/taskkill/i);
    expect(stopScript).not.toMatch(/Stop-Process\s+-Name\s+winws/i);
    expect(exportScript).toContain("engine-runtime");
    expect(exportScript).not.toMatch(/Stop-Process/i);
  });
});
