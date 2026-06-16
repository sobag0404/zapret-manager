import { RecoveryAction } from "../components/RecoveryAction";
import { appActions, useAppStore } from "../store/appStore";

const actions = [
  ["repair_driver", "Проверить драйвер", "Mock-проверка: драйвер в v1 не используется."],
  ["repair_service", "Переустановить службу", "Проверить mock service registration flow."],
  ["restart_engine", "Остановить engine", "Остановить mock engine adapter."],
  ["disable_all", "Удалить временные правила", "Выполнить безопасный mock cleanup."],
  ["restore_snapshot", "Восстановить состояние системы", "Применить последний snapshot."],
  ["emergency_disable", "Аварийно отключить всё", "Остановить engine, убрать временные правила и восстановить snapshot."],
] as const;

export function Recovery() {
  const { loading } = useAppStore();
  return (
    <div className="page-stack">
      <header className="page-header">
        <span className="eyebrow">Восстановление</span>
        <h1>Repair flow</h1>
      </header>
      <section className="list-panel">
        {actions.map(([id, title, description]) => (
          <RecoveryAction
            key={id}
            id={id}
            title={title}
            description={description}
            loading={loading[`recovery:${id}`]}
            onRun={appActions.recoveryAction}
          />
        ))}
      </section>
    </div>
  );
}
