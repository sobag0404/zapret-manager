import { LogViewer } from "../components/LogViewer";
import { appActions, useAppStore } from "../store/appStore";

export function Logs() {
  const { userLog, exportPath, loading } = useAppStore();
  return (
    <div className="page-stack">
      <header className="page-header">
        <span className="eyebrow">Логи</span>
        <h1>Пользовательский лог</h1>
      </header>
      <LogViewer log={userLog} exportPath={exportPath} loading={loading.logs || loading["export-logs"]} onRefresh={appActions.refreshLogs} onExport={appActions.exportLogs} />
    </div>
  );
}
