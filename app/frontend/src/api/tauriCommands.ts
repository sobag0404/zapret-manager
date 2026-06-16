export type RuntimeStatus = "disabled" | "starting" | "running" | "stopping" | "error";
export type ProfileStatus = "stable" | "experimental";
export type DiagnosticStatus = "ok" | "warning" | "error" | "skipped";

export interface Profile {
  id: string;
  name: string;
  description: string;
  status: ProfileStatus;
  version: string;
  targets: string[];
  health_checks: string[];
  engine_profile_ref: string;
  fallback_profiles: string[];
  risk_level: "low" | "medium" | "high";
  notes: string;
}

export interface AppStatus {
  status: RuntimeStatus;
  enabled_profiles: string[];
  profiles: Profile[];
  message: string;
}

export interface DiagnosticItem {
  id: string;
  title: string;
  status: DiagnosticStatus;
  problem?: string | null;
  action?: string | null;
}

export interface DiagnosticReport {
  overall: DiagnosticStatus;
  items: DiagnosticItem[];
}

export interface StrategyUpdateStatus {
  app_version: string;
  strategy_version: string;
  last_checked: string;
  channel: string;
  message: string;
}

export interface AppSettings {
  autostart: boolean;
  strategy_channel: string;
  logs_path: string;
  engine_path: string;
  safety_mode: boolean;
}

type TauriInvoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

declare global {
  interface Window {
    __TAURI__?: { core?: { invoke?: TauriInvoke }; tauri?: { invoke?: TauriInvoke } };
    __TAURI_INTERNALS__?: { invoke?: TauriInvoke };
  }
}

const now = () => new Date().toISOString();

const mockProfiles: Profile[] = [
  profile("discord", "Discord", "Discord desktop, web and voice checks", "medium"),
  profile("youtube", "YouTube", "YouTube web and video checks", "medium"),
  profile("telegram", "Telegram", "Telegram desktop and web checks", "medium"),
  profile("common", "Общий режим", "General safe profile", "low"),
];

let mockStatus: AppStatus = {
  status: "disabled",
  enabled_profiles: [],
  profiles: mockProfiles,
  message: "Отключено",
};

let mockSettings: AppSettings = {
  autostart: false,
  strategy_channel: "stable",
  logs_path: "logs",
  engine_path: "engine/local",
  safety_mode: true,
};

let userLog = `${new Date().toLocaleTimeString()} - Приложение запущено.\n`;

function profile(id: string, name: string, description: string, risk_level: Profile["risk_level"]): Profile {
  return {
    id,
    name,
    description,
    status: "stable",
    version: "1.0.0",
    targets: ["desktop_app", "web"],
    health_checks: ["dns", "tcp", "https"],
    engine_profile_ref: `${id}-default`,
    fallback_profiles: [`${id}-safe`],
    risk_level,
    notes: "No low-level strategy values in initial scaffold",
  };
}

function invoke(): TauriInvoke | null {
  return window.__TAURI__?.core?.invoke ?? window.__TAURI__?.tauri?.invoke ?? window.__TAURI_INTERNALS__?.invoke ?? null;
}

async function call<T>(command: string, args: Record<string, unknown> | undefined, fallback: () => T | Promise<T>): Promise<T> {
  const tauriInvoke = invoke();
  if (tauriInvoke) return tauriInvoke<T>(command, args);
  await new Promise((resolve) => window.setTimeout(resolve, 120));
  return fallback();
}

const clone = <T>(value: T): T => JSON.parse(JSON.stringify(value)) as T;
const addLog = (message: string) => {
  userLog += `${new Date().toLocaleTimeString()} - ${message}\n`;
};

function mockDiagnostics(): DiagnosticReport {
  const items: DiagnosticItem[] = [
    diag("admin", "Права администратора", "warning", "Запустите приложение от имени администратора для реального управления службой."),
    diag("service_installed", "Служба установлена", "ok", "Действий не требуется."),
    diag("service_running", "Служба запущена", "ok", "Действий не требуется."),
    diag("engine_found", "Engine найден", "warning", "Подключите проверенный engine manifest."),
    diag("engine_hash", "Engine hash совпадает", "skipped", "Будет проверяться после подключения engine."),
    diag("driver", "Драйвер доступен", "skipped", "В mock-режиме драйвер не используется."),
    diag("profile_valid", "Профили валидны", "ok", "Действий не требуется."),
    diag("strategy_valid", "Стратегии валидны", "ok", "Действий не требуется."),
    diag("dns", "DNS работает", "ok", "Действий не требуется."),
    diag("internet", "Интернет доступен", "ok", "Действий не требуется."),
    diag("discord", "Discord доступен", "ok", "Действий не требуется."),
    diag("youtube", "YouTube доступен", "ok", "Действий не требуется."),
    diag("telegram", "Telegram доступен", "ok", "Действий не требуется."),
    diag("vpn", "Нет конфликта с VPN", "skipped", "Автоопределение будет добавлено через Windows API."),
    diag("proxy", "Нет конфликта с proxy", "ok", "Proxy не менялся."),
    diag("antivirus", "Нет конфликта с антивирусом", "skipped", "Антивирус не опрашивается."),
    diag("logs", "Папка логов доступна", "ok", "Действий не требуется."),
    diag("snapshot", "Snapshot можно создать", "ok", "Действий не требуется."),
    diag("revert", "Revert можно выполнить", "ok", "Действий не требуется."),
    diag("strategy_integrity", "Последняя стратегия не повреждена", "ok", "Действий не требуется."),
  ];
  return { overall: "warning", items };
}

function diag(id: string, title: string, status: DiagnosticStatus, action: string): DiagnosticItem {
  return {
    id,
    title,
    status,
    problem: status === "ok" ? null : `Проблема: ${title}.`,
    action,
  };
}

export const tauriCommands = {
  getAppStatus: () => call<AppStatus>("get_app_status", undefined, () => clone(mockStatus)),
  listProfiles: () => call<Profile[]>("list_profiles", undefined, () => clone(mockProfiles)),
  setProfileEnabled: (id: string, enabled: boolean) =>
    call<string[]>("set_profile_enabled", { id, enabled }, () => {
      if (enabled && !mockStatus.enabled_profiles.includes(id)) mockStatus.enabled_profiles.push(id);
      if (!enabled) mockStatus.enabled_profiles = mockStatus.enabled_profiles.filter((profileId) => profileId !== id);
      return clone(mockStatus.enabled_profiles);
    }),
  toggleEnabled: (profileIds: string[]) =>
    call<AppStatus>("toggle_enabled", { profileIds }, () => {
      if (mockStatus.status === "running") {
        mockStatus = { ...mockStatus, status: "disabled", enabled_profiles: [], message: "Отключено" };
        addLog("Режим выключен. Система восстановлена.");
      } else {
        mockStatus = { ...mockStatus, status: "running", enabled_profiles: profileIds, message: "Работает" };
        addLog(`Режим включён: ${profileIds.join(", ")}.`);
      }
      return clone(mockStatus);
    }),
  enableSelectedProfiles: (profileIds: string[]) =>
    call<AppStatus>("enable_selected_profiles", { profileIds }, () => {
      mockStatus = { ...mockStatus, status: "running", enabled_profiles: profileIds, message: "Работает" };
      addLog(`Режим включён: ${profileIds.join(", ")}.`);
      return clone(mockStatus);
    }),
  disableAll: () =>
    call<AppStatus>("disable_all", undefined, () => {
      mockStatus = { ...mockStatus, status: "disabled", enabled_profiles: [], message: "Отключено" };
      addLog("Режим выключен. Система восстановлена.");
      return clone(mockStatus);
    }),
  runDiagnostics: () => call<DiagnosticReport>("run_diagnostics", undefined, () => clone(mockDiagnostics())),
  runDnsCheck: () =>
    call<DiagnosticReport>("run_dns_check", undefined, () => {
      const report = mockDiagnostics();
      return { overall: "ok", items: report.items.filter((item) => item.id.includes("dns") || item.id === "internet") };
    }),
  runServiceConnectivityTests: () =>
    call<DiagnosticReport>("run_service_connectivity_tests", undefined, () => {
      const report = mockDiagnostics();
      return { overall: "ok", items: report.items.filter((item) => ["internet", "discord", "youtube", "telegram"].includes(item.id)) };
    }),
  readUserLogs: () => call<string>("read_user_logs", undefined, () => userLog),
  exportDebugLogs: () => call<string>("export_debug_logs", undefined, () => "logs/debug-export.jsonl"),
  checkStrategyUpdates: () =>
    call<StrategyUpdateStatus>("check_strategy_updates", undefined, () => ({
      app_version: "0.1.0",
      strategy_version: "1.0.0",
      last_checked: now(),
      channel: "stable",
      message: "Mock manifest checked, updates not required.",
    })),
  applyStrategyUpdate: () =>
    call<StrategyUpdateStatus>("apply_strategy_update", undefined, () => ({
      app_version: "0.1.0",
      strategy_version: "1.0.0",
      last_checked: now(),
      channel: "stable",
      message: "Mock strategy update applied with backup.",
    })),
  rollbackStrategyUpdate: () =>
    call<StrategyUpdateStatus>("rollback_strategy_update", undefined, () => ({
      app_version: "0.1.0",
      strategy_version: "1.0.0",
      last_checked: now(),
      channel: "stable",
      message: "Mock strategy rollback completed.",
    })),
  repairDriver: () => call<string>("repair_driver", undefined, () => "Mock: драйвер не используется."),
  repairService: () => call<string>("repair_service", undefined, () => "Mock: служба проверена."),
  restartEngine: () => call<string>("restart_engine", undefined, () => "Mock: engine перезапущен."),
  emergencyDisable: () => call<AppStatus>("emergency_disable", undefined, () => tauriCommands.disableAll()),
  createSnapshot: () => call<unknown>("create_snapshot", undefined, () => ({ timestamp: now(), active_profiles: mockStatus.enabled_profiles })),
  restoreSnapshot: () => call<AppStatus>("restore_snapshot", undefined, () => tauriCommands.disableAll()),
  getSettings: () => call<AppSettings>("get_settings", undefined, () => clone(mockSettings)),
  saveSettings: (settings: AppSettings) =>
    call<AppSettings>("save_settings", { settings }, () => {
      mockSettings = clone(settings);
      addLog("Настройки сохранены.");
      return clone(mockSettings);
    }),
};
