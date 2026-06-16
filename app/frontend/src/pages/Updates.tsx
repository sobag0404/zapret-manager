import { UpdateStatus } from "../components/UpdateStatus";
import { appActions, useAppStore } from "../store/appStore";

export function Updates() {
  const { updateStatus, loading } = useAppStore();
  return (
    <div className="page-stack">
      <header className="page-header">
        <span className="eyebrow">Обновления</span>
        <h1>App / Strategies / Engine</h1>
      </header>
      <UpdateStatus
        info={updateStatus}
        loading={loading.updates || loading["apply-update"] || loading["rollback-update"]}
        onCheck={appActions.checkUpdates}
        onApply={appActions.applyUpdate}
        onRollback={appActions.rollbackUpdate}
      />
    </div>
  );
}
