import { describe, expect, it } from "vitest";
import { loadStartupState } from "../app/frontend/src/store/appStore";
import type { AppSettings, AppStatus, DiagnosticReport, Profile, StrategyUpdateStatus } from "../app/frontend/src/api/tauriCommands";

const status: AppStatus = {
  status: "disabled",
  enabled_profiles: ["discord"],
  profiles: [],
  message: "Отключено",
};

const profiles: Profile[] = [
  {
    id: "discord",
    name: "Discord",
    description: "Discord",
    status: "experimental",
    version: "1.0.0",
    targets: ["web"],
    health_checks: ["dns"],
    engine_profile_ref: "discord-default",
    fallback_profiles: ["discord-safe"],
    risk_level: "medium",
    notes: "test",
  },
];

const settings: AppSettings = {
  autostart: false,
  strategy_channel: "stable",
  engine_strategy: "alt",
  logs_path: "logs",
  engine_path: "engine/local",
  safety_mode: true,
  allow_vpn_conflict: true,
};

const diagnostics: DiagnosticReport = {
  overall: "ok",
  items: [{ id: "engine", title: "Engine", status: "ok", problem: null, action: "ok" }],
};

const strategyUpdateStatus: StrategyUpdateStatus = {
  app_version: "1.2.0",
  strategy_version: "1.0.0",
  last_checked: "2026-07-13T00:00:00Z",
  channel: "stable",
  message: "ok",
};

describe("startup state loading", () => {
  it("keeps critical startup state when optional calls fail", async () => {
    const result = await loadStartupState({
      getAppStatus: async () => status,
      listProfiles: async () => profiles,
      getSettings: async () => settings,
      runDiagnostics: async () => diagnostics,
      checkStrategyUpdates: async () => {
        throw new Error("update unavailable");
      },
      readUserLogs: async () => {
        throw new Error("log unavailable");
      },
    });

    expect(result.critical.status).toEqual(status);
    expect(result.critical.profiles).toEqual(profiles);
    expect(result.critical.selectedProfiles).toEqual(["discord"]);
    expect(result.critical.settings).toEqual(settings);
    expect(result.optional.diagnostics).toEqual(diagnostics.items);
    expect("strategyUpdateStatus" in result.optional).toBe(false);
    expect("userLog" in result.optional).toBe(false);
    expect(result.optionalErrors).toEqual([
      "strategy updates: update unavailable",
      "user log: log unavailable",
    ]);
  });

  it("fails when critical startup state fails", async () => {
    await expect(
      loadStartupState({
        getAppStatus: async () => {
          throw new Error("status unavailable");
        },
        listProfiles: async () => profiles,
        getSettings: async () => settings,
        runDiagnostics: async () => diagnostics,
        checkStrategyUpdates: async () => strategyUpdateStatus,
        readUserLogs: async () => "log",
      }),
    ).rejects.toThrow("status unavailable");
  });
});
