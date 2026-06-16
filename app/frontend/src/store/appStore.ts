import { useSyncExternalStore } from "react";
import {
  AppSettings,
  AppStatus,
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
  updateStatus: StrategyUpdateStatus | null;
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
  updateStatus: null,
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

export const appActions = {
  setPage: (selectedPage: PageId) => setState({ selectedPage }),
  initialize: async () => {
    await runAction("initialize", async () => {
      const [status, profiles, diagnostics, updateStatus, userLog, settings] = await Promise.all([
        tauriCommands.getAppStatus(),
        tauriCommands.listProfiles(),
        tauriCommands.runDiagnostics(),
        tauriCommands.checkStrategyUpdates(),
        tauriCommands.readUserLogs(),
        tauriCommands.getSettings(),
      ]);
      setState({
        status,
        profiles,
        selectedProfiles: status.enabled_profiles,
        diagnostics: diagnostics.items,
        updateStatus,
        userLog,
        settings,
      });
    });
  },
  setProfileSelected: async (id: string, enabled: boolean) => {
    const selectedProfiles = enabled
      ? Array.from(new Set([...state.selectedProfiles, id]))
      : state.selectedProfiles.filter((profileId) => profileId !== id);
    setState({ selectedProfiles });
    await runAction(`profile:${id}`, () => tauriCommands.setProfileEnabled(id, enabled));
  },
  toggleEnabled: async () => {
    const status = await runAction("toggle", () => tauriCommands.toggleEnabled(state.selectedProfiles));
    if (status) {
      setState({ status, selectedProfiles: status.enabled_profiles.length ? status.enabled_profiles : state.selectedProfiles });
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
  checkUpdates: async () => {
    const updateStatus = await runAction("updates", tauriCommands.checkStrategyUpdates);
    if (updateStatus) setState({ updateStatus });
  },
  applyUpdate: async () => {
    const updateStatus = await runAction("apply-update", tauriCommands.applyStrategyUpdate);
    if (updateStatus) setState({ updateStatus });
  },
  rollbackUpdate: async () => {
    const updateStatus = await runAction("rollback-update", tauriCommands.rollbackStrategyUpdate);
    if (updateStatus) setState({ updateStatus });
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
