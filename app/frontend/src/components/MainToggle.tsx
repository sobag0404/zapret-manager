import { Power } from "lucide-react";
import { RuntimeStatus } from "../api/tauriCommands";

interface MainToggleProps {
  status: RuntimeStatus;
  loading?: boolean;
  onToggle: () => void;
}

export function MainToggle({ status, loading, onToggle }: MainToggleProps) {
  const running = status === "running" || status === "starting";
  return (
    <button className={`main-toggle ${running ? "is-on" : "is-off"}`} disabled={loading} onClick={onToggle}>
      <span className="toggle-icon">
        <Power size={28} aria-hidden="true" />
      </span>
      <span>
        <strong>{loading ? "Выполняется" : running ? "Выключить" : "Включить"}</strong>
        <small>{running ? "Локальный engine работает" : "Режим отключён"}</small>
      </span>
    </button>
  );
}
