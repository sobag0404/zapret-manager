import { RecoveryAction } from "../components/RecoveryAction";
import { appActions, useAppStore } from "../store/appStore";

const actions = [
  ["repair_driver", "Проверить драйвер", "Проверить, не блокируется ли запуск WinDivert правами или антивирусом."],
  ["restart_engine", "Остановить engine", "Остановить активный engine и проверить, что процессы не остались висеть."],
  ["disable_all", "Удалить временные правила", "Выполнить безопасную очистку временных правил и состояния."],
  ["create_snapshot", "Создать snapshot", "Сохранить текущее состояние перед ручными действиями."],
  ["restore_snapshot", "Восстановить состояние системы", "Применить последний snapshot и вернуть настройки."],
  ["emergency_disable", "Аварийно отключить всё", "Остановить engine, убрать временные правила и восстановить snapshot."],
] as const;

export function Recovery() {
  const { loading } = useAppStore();
  return (
    <div className="page-stack">
      <header className="page-header">
        <div>
          <span className="eyebrow">Восстановление</span>
          <h1>Repair flow</h1>
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
