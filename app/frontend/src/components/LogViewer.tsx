import { Download } from "lucide-react";

interface LogViewerProps {
  log: string;
  exportPath: string | null;
  loading?: boolean;
  onRefresh: () => void;
  onExport: () => void;
}

export function LogViewer({ log, exportPath, loading, onRefresh, onExport }: LogViewerProps) {
  return (
    <section className="log-viewer">
      <div className="button-row log-toolbar">
        <button className="secondary-button" disabled={loading} onClick={onRefresh}>Обновить</button>
        <button className="primary-button" disabled={loading} onClick={onExport}>
          <Download size={16} aria-hidden="true" />
          Экспорт логов
        </button>
      </div>
      {exportPath ? <p className="muted">Экспорт: {exportPath}</p> : null}
      <pre>{log || "Лог пуст."}</pre>
    </section>
  );
}
