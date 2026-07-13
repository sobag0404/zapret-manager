import { useSyncExternalStore } from "react";
import {
  AppSettings,
  AppStatus,
  AppUpdateStatus,
  DiagnosticItem,
  Profile,
  StrategyUpdateStatus,
  tauriCommands,
} from "../api/tauriCommands";

export type PageId = "dashboard" | "profiles" | "diagnostics" | "recovery" | "updates" | "logs" | "settings";

interface AppState {
  status: AppStatus | null;
  profiles: Profile[];
  selectedProfiles: string[];
  diagnostics: DiagnosticItem[];
  strategyUpdateStatus: StrategyUpdateStatus | null;
  appUpdateStatus: AppUpdateStatus | null;
  userLog: string;
  exportPath: string | null;
  settings: AppSettings | null;
  selectedPage: PageId;
  loading: Record<string, boolean>;
  error: string | null;
}

const initialState: AppState = {
  status: null,
  profiles: [],
  selectedProfiles: [],
  diagnostics: [],
  strategyUpdateStatus: null,
  appUpdateStatus: null,
  userLog: "",
  exportPath: null,
  settings: null,
  selectedPage: "dashboard",
  loading: {},
  error: null,
};

let state = initialState;
const listeners = new Set<() => void>();
const emit = () => listeners.forEach((listener) => listener());
const setState = (patch: Partial<AppState>) => {
  state = { ...state, ...patch };
  emit();
};
const setLoading = (key: string, value: boolean) => {
  state = { ...state, loading: { ...state.loading, [key]: value } };
  emit();
};

type StartupCommands = Pick<
  typeof tauriCommands,
  "getAppStatus" | "listProfiles" | "getSettings" | "runDiagnostics" | "checkStrategyUpdates" | "readUserLogs"
>;

export interface StartupStateResult {
  critical: Pick<AppState, "status" | "profiles" | "selectedProfiles" | "settings">;
  optional: Partial<Pick<AppState, "diagnostics" | "strategyUpdateStatus" | "userLog">>;
  optionalErrors: string[];
}

async function runAction<T>(key: string, action: () => Promise<T>): Promise<T | null> {
  setLoading(key, true);
  setState({ error: null });
  try {
    return await action();
  } catch (error) {
    setState({ error: error instanceof Error ? error.message : String(error) });
    return null;
  } finally {
    setLoading(key, false);
  }
}

function nextSelectedProfiles(id: string, enabled: boolean): string[] {
  const allProfileIds = state.profiles.map((profile) => profile.id);
  const regularProfileIds = allProfileIds.filter((profileId) => profileId !== "common");

  if (id === "common") {
    return enabled ? allProfileIds : [];
  }

  if (!enabled) {
    return state.selectedProfiles.filter((profileId) => profileId !== id && profileId !== "common");
  }

  const selected = Array.from(new Set([...state.selectedProfiles, id]));
  if (regularProfileIds.every((profileId) => selected.includes(profileId))) {
    return Array.from(new Set([...selected, "common"]));
  }
  return selected;
}

function defaultSettings(): AppSettings {
  return {
    autostart: false,
    strategy_channel: "stable",
    engine_strategy: "general",
    logs_path: "logs",
    engine_path: "engine/local",
    safety_mode: true,
    allow_vpn_conflict: true,
  };
}

async function optionalStartup<T>(
  label: string,
  action: () => Promise<T>,
): Promise<{ value?: T; error?: string }> {
  try {
    return { value: await action() };
  } catch (error) {
    return { error: `${label}: ${error instanceof Error ? error.message : String(error)}` };
  }
}

export async function loadStartupState(commands: StartupCommands = tauriCommands): Promise<StartupStateResult> {
  const [status, profiles, settings] = await Promise.all([
    commands.getAppStatus(),
    commands.listProfiles(),
    commands.getSettings(),
  ]);

  const [diagnostics, strategyUpdateStatus, userLog] = await Promise.all([
    optionalStartup("diagnostics", commands.runDiagnostics),
    optionalStartup("strategy updates", commands.checkStrategyUpdates),
    optionalStartup("user log", commands.readUserLogs),
  ]);

  const optionalErrors = [diagnostics.error, strategyUpdateStatus.error, userLog.error].filter(Boolean) as string[];

  const optional: StartupStateResult["optional"] = {};
  if (diagnostics.value) optional.diagnostics = diagnostics.value.items;
  if (strategyUpdateStatus.value) optional.strategyUpdateStatus = strategyUpdateStatus.value;
  if (userLog.value) optional.userLog = userLog.value;

  return {
    critical: {
      status,
      profiles,
      selectedProfiles: status.enabled_profiles,
      settings,
    },
    optional,
    optionalErrors,
  };
}

export const appActions = {
  setPage: (selectedPage: PageId) => setState({ selectedPage }),
  initialize: async () => {
    await runAction("initialize", async () => {
      const startup = await loadStartupState();
      setState({
        ...startup.critical,
        ...startup.optional,
        error: startup.optionalErrors.length ? `Часть данных не загрузилась: ${startup.optionalErrors.join("; ")}` : null,
      });
    });
  },
  setProfileSelected: async (id: string, enabled: boolean) => {
    setState({ selectedProfiles: nextSelectedProfiles(id, enabled), error: null });
  },
  toggleEnabled: async () => {
    if (state.status?.status !== "running" && state.selectedProfiles.length === 0) {
      setState({ error: "Выберите хотя бы один режим: Discord, YouTube, Telegram, WhatsApp или Общий." });
      return;
    }
    const status = await runAction("toggle", () => tauriCommands.toggleEnabled(state.selectedProfiles));
    if (status) {
      setState({
        status,
        selectedProfiles: status.enabled_profiles.length ? status.enabled_profiles : state.selectedProfiles,
      });
      await appActions.refreshLogs();
    }
  },
  runDiagnostics: async () => {
    const report = await runAction("diagnostics", tauriCommands.runDiagnostics);
    if (report) setState({ diagnostics: report.items });
  },
  runDnsCheck: async () => {
    const report = await runAction("dns", tauriCommands.runDnsCheck);
    if (report) setState({ diagnostics: report.items });
  },
  runConnectivity: async () => {
    const report = await runAction("connectivity", tauriCommands.runServiceConnectivityTests);
    if (report) setState({ diagnostics: report.items });
  },
  runMessagingDiagnostics: async () => {
    const report = await runAction("messaging-diagnostics", tauriCommands.runMessagingDiagnostics);
    if (report) setState({ diagnostics: report.items });
  },
  recoveryAction: async (id: string) => {
    const actions: Record<string, () => Promise<unknown>> = {
      repair_driver: tauriCommands.repairDriver,
      repair_service: tauriCommands.repairService,
      restart_engine: tauriCommands.restartEngine,
      emergency_disable: tauriCommands.emergencyDisable,
      create_snapshot: tauriCommands.createSnapshot,
      restore_snapshot: tauriCommands.restoreSnapshot,
      disable_all: tauriCommands.disableAll,
    };
    await runAction(`recovery:${id}`, actions[id] ?? tauriCommands.repairService);
    const status = await tauriCommands.getAppStatus();
    setState({ status });
    await appActions.refreshLogs();
  },
  checkStrategyUpdates: async () => {
    const strategyUpdateStatus = await runAction("strategy-updates", tauriCommands.checkStrategyUpdates);
    if (strategyUpdateStatus) setState({ strategyUpdateStatus });
  },
  applyStrategyUpdate: async () => {
    const strategyUpdateStatus = await runAction("apply-strategy-update", tauriCommands.applyStrategyUpdate);
    if (strategyUpdateStatus) setState({ strategyUpdateStatus });
  },
  rollbackStrategyUpdate: async () => {
    const strategyUpdateStatus = await runAction("rollback-strategy-update", tauriCommands.rollbackStrategyUpdate);
    if (strategyUpdateStatus) setState({ strategyUpdateStatus });
  },
  checkAppUpdate: async () => {
    const appUpdateStatus = await runAction("app-update-check", tauriCommands.checkAppUpdate);
    if (appUpdateStatus) setState({ appUpdateStatus });
  },
  installAppUpdate: async () => {
    const appUpdateStatus = await runAction("app-update-install", tauriCommands.installAppUpdate);
    if (appUpdateStatus) setState({ appUpdateStatus });
  },
  refreshLogs: async () => {
    const userLog = await runAction("logs", tauriCommands.readUserLogs);
    if (userLog !== null) setState({ userLog });
  },
  exportLogs: async () => {
    const exportPath = await runAction("export-logs", tauriCommands.exportDebugLogs);
    if (exportPath) setState({ exportPath });
  },
  saveSettings: async (settings: AppSettings) => {
    const saved = await runAction("settings", () => tauriCommands.saveSettings(settings));
    if (saved) setState({ settings: saved });
  },
  setEngineStrategy: async (engine_strategy: string) => {
    const nextSettings = { ...(state.settings ?? defaultSettings()), engine_strategy };
    setState({ settings: nextSettings, error: null });
    const saved = await runAction("settings", () => tauriCommands.saveSettings(nextSettings));
    if (saved) setState({ settings: saved });
  },
  nextProfileStrategy: async () => {
    const profile = state.selectedProfiles.length === 1 ? state.selectedProfiles[0] : null;
    const candidates =
      profile === "telegram" || profile === "whatsapp"
        ? ["alt", "alt3", "simple_fake", "general", "alt5", "fake_tls_auto"]
        : ["alt", "alt3", "simple_fake", "alt5"];
    const current = state.settings?.engine_strategy ?? "general";
    const currentIndex = candidates.indexOf(current);
    const engine_strategy = candidates[(currentIndex + 1) % candidates.length];
    await appActions.setEngineStrategy(engine_strategy);
  },
};

export function useAppStore(): AppState {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => state,
    () => state,
  );
}
