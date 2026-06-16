import { UpdateStatus } from "../components/UpdateStatus";
import { appActions, useAppStore } from "../store/appStore";

export function Updates() {
  const { updateStatus, loading } = useAppStore();
  return (
    <div className="page-stack">
      <header className="page-header">
        <div>
          <span className="eyebrow">Обновления</span>
          <h1>Обновления</h1>
        </div>
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
