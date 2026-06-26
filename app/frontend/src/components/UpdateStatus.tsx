import { Download, RotateCcw } from "lucide-react";
import { AppUpdateStatus, StrategyUpdateStatus } from "../api/tauriCommands";

interface UpdateStatusProps {
  appInfo: AppUpdateStatus | null;
  strategyInfo: StrategyUpdateStatus | null;
  appLoading?: boolean;
  strategyLoading?: boolean;
  onCheckApp: () => void;
  onInstallApp: () => void;
  onCheckStrategy: () => void;
  onApplyStrategy: () => void;
  onRollbackStrategy: () => void;
}

export function UpdateStatus({
  appInfo,
  strategyInfo,
  appLoading,
  strategyLoading,
  onCheckApp,
  onInstallApp,
  onCheckStrategy,
  onApplyStrategy,
  onRollbackStrategy,
}: UpdateStatusProps) {
  const hasAppUpdate = appInfo?.state === "available";

  return (
    <div className="page-stack">
      <section className="update-panel">
        <Download size={22} aria-hidden="true" />
        <div>
          <h2>Приложение</h2>
          <p>{appInfo?.message ?? "Обновления приложения ещё не проверялись."}</p>
          <dl>
            <div><dt>Текущая версия</dt><dd>{appInfo?.current_version ?? "1.2.0"}</dd></div>
            <div><dt>Доступная версия</dt><dd>{appInfo?.available_version ?? "нет"}</dd></div>
            <div><dt>Канал</dt><dd>stable</dd></div>
            <div><dt>Установка</dt><dd>только вручную</dd></div>
          </dl>
          {appInfo?.notes ? <p className="release-notes">{appInfo.notes}</p> : null}
        </div>
        <div className="button-row">
          <button className="primary-button" disabled={appLoading} onClick={onCheckApp}>Проверить приложение</button>
          <button className="secondary-button" disabled={appLoading || !hasAppUpdate} onClick={onInstallApp}>Установить и перезапустить</button>
        </div>
      </section>

      <section className="update-panel">
        <RotateCcw size={22} aria-hidden="true" />
        <div>
          <h2>Стратегии</h2>
          <p>{strategyInfo?.message ?? "Статус обновлений стратегий ещё не проверен."}</p>
          <dl>
            <div><dt>Приложение</dt><dd>{strategyInfo?.app_version ?? "1.2.0"}</dd></div>
            <div><dt>Стратегии</dt><dd>{strategyInfo?.strategy_version ?? "1.0.0"}</dd></div>
            <div><dt>Канал</dt><dd>{strategyInfo?.channel ?? "stable"}</dd></div>
            <div><dt>Проверка</dt><dd>{strategyInfo?.last_checked ? new Date(strategyInfo.last_checked).toLocaleString() : "не было"}</dd></div>
          </dl>
        </div>
        <div className="button-row">
          <button className="primary-button" disabled={strategyLoading} onClick={onCheckStrategy}>Проверить стратегии</button>
          <button className="secondary-button" disabled={strategyLoading} onClick={onApplyStrategy}>Применить</button>
          <button className="secondary-button" disabled={strategyLoading} onClick={onRollbackStrategy}>Откатить</button>
        </div>
      </section>
    </div>
  );
}
