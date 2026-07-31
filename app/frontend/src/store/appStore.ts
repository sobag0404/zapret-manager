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

export type PageId = "dashboard" | "diagnostics" | "recovery" | "updates" | "logs" | "settings";

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
  strategySelectedManually: boolean;
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
  strategySelectedManually: false,
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

export const YOUTUBE_RECOMMENDED_STRATEGY = "fake_tls_auto";
export const DISCORD_RECOMMENDED_STRATEGY = "alt";
export const COMBINED_RECOMMENDED_STRATEGY = "combined";

export interface EngineStrategyRecommendationInput {
  selectedProfiles: string[];
  runtimeStatus: AppStatus["status"] | null;
  currentStrategy: string;
  strategySelectedManually: boolean;
}

export function recommendedEngineStrategy(input: EngineStrategyRecommendationInput): string | null {
  if (input.runtimeStatus !== "disabled" || input.strategySelectedManually) return null;
  const selected = [...new Set(input.selectedProfiles)].sort();
  const desired = selected.length === 1 && selected[0] === "discord"
    ? DISCORD_RECOMMENDED_STRATEGY
    : selected.length === 1 && selected[0] === "youtube"
      ? YOUTUBE_RECOMMENDED_STRATEGY
      : selected.length === 2 && selected[0] === "discord" && selected[1] === "youtube"
        ? COMBINED_RECOMMENDED_STRATEGY
        : null;
  if (desired !== COMBINED_RECOMMENDED_STRATEGY && input.currentStrategy !== "general" && input.currentStrategy !== COMBINED_RECOMMENDED_STRATEGY) {
    return null;
  }
  return desired && desired !== input.currentStrategy ? desired : null;
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

export function nextSelectedProfiles(current: string[], id: string, enabled: boolean): string[] {
  if (id !== "discord" && id !== "youtube") return current;
  const next = enabled ? [...current, id] : current.filter((profileId) => profileId !== id);
  return [...new Set(next)].filter((profileId) => profileId === "discord" || profileId === "youtube").sort();
}

function defaultSettings(): AppSettings {
  return {
    autostart: false,
    strategy_channel: "stable",
    engine_strategy: "general",
    selected_profiles: [],
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
      selectedProfiles: status.enabled_profiles.length ? status.enabled_profiles : settings.selected_profiles,
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
        strategySelectedManually: false,
        error: startup.optionalErrors.length ? `Часть данных не загрузилась: ${startup.optionalErrors.join("; ")}` : null,
      });
    });
  },
  setProfileSelected: async (id: string, enabled: boolean) => {
    const selectedProfiles = nextSelectedProfiles(state.selectedProfiles, id, enabled);
    setState({ selectedProfiles, error: null });

    const persisted = await runAction("profile-selection", () => tauriCommands.setProfileEnabled(id, enabled));
    if (persisted) {
      setState({
        selectedProfiles: persisted,
        settings: { ...(state.settings ?? defaultSettings()), selected_profiles: persisted },
      });
    }

    const recommended = recommendedEngineStrategy({
      selectedProfiles,
      runtimeStatus: state.status?.status ?? null,
      currentStrategy: state.settings?.engine_strategy ?? "general",
      strategySelectedManually: state.strategySelectedManually,
    });
    if (recommended) await appActions.setEngineStrategy(recommended, false);
  },
  toggleEnabled: async () => {
    if (state.status?.status !== "running" && state.status?.status !== "error" && state.selectedProfiles.length === 0) {
      setState({ error: "Выберите Discord, YouTube или оба режима." });
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
    const strategyChanged = settings.engine_strategy !== state.settings?.engine_strategy;
    const saved = await runAction("settings", () => tauriCommands.saveSettings(settings));
    if (saved) setState({ settings: saved, strategySelectedManually: strategyChanged || state.strategySelectedManually });
  },
  setEngineStrategy: async (engine_strategy: string, manual = true) => {
    const nextSettings = { ...(state.settings ?? defaultSettings()), engine_strategy };
    setState({ settings: nextSettings, strategySelectedManually: manual, error: null });
    const saved = await runAction("settings", () => tauriCommands.saveSettings(nextSettings));
    if (saved) setState({ settings: saved, strategySelectedManually: manual });
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
