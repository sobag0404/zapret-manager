import { UpdateStatus } from "../components/UpdateStatus";
import { appActions, useAppStore } from "../store/appStore";

export function Updates() {
  const { appUpdateStatus, strategyUpdateStatus, loading } = useAppStore();
  return (
    <div className="page-stack">
      <header className="page-header">
        <div>
          <span className="eyebrow">Обновления</span>
          <h1>Версии и стратегии</h1>
        </div>
      </header>
      <UpdateStatus
        appInfo={appUpdateStatus}
        strategyInfo={strategyUpdateStatus}
        appLoading={loading["app-update-check"] || loading["app-update-install"]}
        strategyLoading={loading["strategy-updates"] || loading["apply-strategy-update"] || loading["rollback-strategy-update"]}
        onCheckApp={appActions.checkAppUpdate}
        onInstallApp={appActions.installAppUpdate}
        onCheckStrategy={appActions.checkStrategyUpdates}
        onApplyStrategy={appActions.applyStrategyUpdate}
        onRollbackStrategy={appActions.rollbackStrategyUpdate}
      />
    </div>
  );
}
