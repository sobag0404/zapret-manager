import { RotateCcw } from "lucide-react";
import { StrategyUpdateStatus } from "../api/tauriCommands";

interface UpdateStatusProps {
  info: StrategyUpdateStatus | null;
  loading?: boolean;
  onCheck: () => void;
  onApply: () => void;
  onRollback: () => void;
}

export function UpdateStatus({ info, loading, onCheck, onApply, onRollback }: UpdateStatusProps) {
  return (
    <section className="update-panel">
      <RotateCcw size={22} aria-hidden="true" />
      <div>
        <h2>Стратегии</h2>
        <p>{info?.message ?? "Статус обновлений ещё не проверен."}</p>
        <dl>
          <div><dt>Приложение</dt><dd>{info?.app_version ?? "0.1.0"}</dd></div>
          <div><dt>Стратегии</dt><dd>{info?.strategy_version ?? "1.0.0"}</dd></div>
          <div><dt>Канал</dt><dd>{info?.channel ?? "stable"}</dd></div>
          <div><dt>Проверка</dt><dd>{info?.last_checked ? new Date(info.last_checked).toLocaleString() : "не было"}</dd></div>
        </dl>
      </div>
      <div className="button-row">
        <button className="primary-button" disabled={loading} onClick={onCheck}>Проверить обновления</button>
        <button className="secondary-button" disabled={loading} onClick={onApply}>Применить mock-обновление</button>
        <button className="secondary-button" disabled={loading} onClick={onRollback}>Откатить стратегию</button>
      </div>
    </section>
  );
}
