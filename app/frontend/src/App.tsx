import { Activity, FileText, Gauge, LifeBuoy, RotateCcw, Settings as SettingsIcon, SlidersHorizontal } from "lucide-react";
import { useEffect } from "react";
import type { ReactElement } from "react";
import { Dashboard } from "./pages/Dashboard";
import { Diagnostics } from "./pages/Diagnostics";
import { Logs } from "./pages/Logs";
import { Profiles } from "./pages/Profiles";
import { Recovery } from "./pages/Recovery";
import { Settings } from "./pages/Settings";
import { Updates } from "./pages/Updates";
import { appActions, PageId, useAppStore } from "./store/appStore";

const pages = [
  { id: "dashboard", label: "Главная", icon: Gauge },
  { id: "profiles", label: "Профили", icon: SlidersHorizontal },
  { id: "diagnostics", label: "Диагностика", icon: Activity },
  { id: "recovery", label: "Восстановление", icon: LifeBuoy },
  { id: "updates", label: "Обновления", icon: RotateCcw },
  { id: "logs", label: "Логи", icon: FileText },
  { id: "settings", label: "Настройки", icon: SettingsIcon },
] satisfies Array<{ id: PageId; label: string; icon: typeof Gauge }>;

const pageComponents: Record<PageId, () => ReactElement> = {
  dashboard: Dashboard,
  profiles: Profiles,
  diagnostics: Diagnostics,
  recovery: Recovery,
  updates: Updates,
  logs: Logs,
  settings: Settings,
};

export default function App() {
  const { selectedPage, status, error, loading } = useAppStore();
  const Page = pageComponents[selectedPage];

  useEffect(() => {
    appActions.initialize();
  }, []);

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <span>ZM</span>
          <div>
            <strong>Zapret Manager</strong>
            <small>{status?.message ?? "Загрузка"}</small>
          </div>
        </div>
        <nav>
          {pages.map(({ id, label, icon: Icon }) => (
            <button className={selectedPage === id ? "is-active" : ""} key={id} onClick={() => appActions.setPage(id)}>
              <Icon size={18} aria-hidden="true" />
              {label}
            </button>
          ))}
        </nav>
      </aside>
      <main>
        {loading.initialize ? <div className="loading-bar" /> : null}
        {error ? <div className="error-banner">{error}</div> : null}
        <Page />
      </main>
    </div>
  );
}
