import { RecoveryAction } from "../components/RecoveryAction";
import { appActions, useAppStore } from "../store/appStore";

const actions = [
  ["repair_driver", "Проверить драйвер", "Mock-проверка: драйвер в v1 не используется."],
  ["repair_service", "Переустановить службу", "Проверить регистрацию mock-службы."],
  ["restart_engine", "Остановить engine", "Остановить mock-адаптер engine."],
  ["disable_all", "Удалить временные правила", "Выполнить безопасный mock cleanup."],
  ["restore_snapshot", "Восстановить состояние системы", "Применить последний snapshot."],
  ["emergency_disable", "Аварийно отключить всё", "Остановить engine, убрать временные правила и восстановить snapshot."],
] as const;

export function Recovery() {
  const { loading } = useAppStore();
  return (
    <div className="page-stack">
      <header className="page-header">
        <div>
          <span className="eyebrow">Восстановление</span>
          <h1>Восстановление</h1>
        </div>
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
