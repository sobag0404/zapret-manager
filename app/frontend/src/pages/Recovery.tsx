import { RecoveryAction } from "../components/RecoveryAction";
import { appActions, useAppStore } from "../store/appStore";

const actions = [
  ["repair_driver", "Проверить драйвер", "Ничего не меняет в системе. Подсказывает, что WinDivert проверяется только при запуске engine."],
  ["restart_engine", "Остановить engine", "Безопасный flow: нажать Выключить, дождаться cleanup, затем снова Включить."],
  ["disable_all", "Очистить runtime state", "Останавливает только engine, запущенный из ZapretManager engine-runtime, и проверяет что процесс не остался."],
  ["create_snapshot", "Создать snapshot", "Сохраняет mock snapshot в локальную папку данных пользователя, не в Program Files."],
  ["restore_snapshot", "Безопасный restore", "В v1.2 выполняет только фактическую безопасную часть: остановку engine и cleanup runtime. DNS/proxy не менялись."],
  ["emergency_disable", "Аварийно отключить всё", "Повторяет disable/cleanup для управляемого engine. Не трогает чужие процессы winws вне runtime ZapretManager."],
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
