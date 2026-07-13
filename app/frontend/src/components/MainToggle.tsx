import { Power } from "lucide-react";
import { RuntimeStatus } from "../api/tauriCommands";

interface MainToggleProps {
  status: RuntimeStatus;
  loading?: boolean;
  onToggle: () => void;
}

export function MainToggle({ status, loading, onToggle }: MainToggleProps) {
  const running = status === "running" || status === "starting";
  const cleanupError = status === "error";
  const label = loading ? "Выполняется" : cleanupError ? "Повторить отключение" : running ? "Выключить" : "Включить";
  const detail = cleanupError
    ? "Отключение не завершено"
    : running
      ? "Локальный engine работает"
      : "Режим отключён";

  return (
    <button className={`main-toggle ${running || cleanupError ? "is-on" : "is-off"}`} disabled={loading} onClick={onToggle}>
      <span className="toggle-icon">
        <Power size={28} aria-hidden="true" />
      </span>
      <span>
        <strong>{label}</strong>
        <small>{detail}</small>
      </span>
    </button>
  );
}
