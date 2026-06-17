import { Activity, Layers3, ShieldCheck } from "lucide-react";
import { MainToggle } from "../components/MainToggle";
import { StatusCard } from "../components/StatusCard";
import { appActions, useAppStore } from "../store/appStore";

const engineStrategies = [
  { id: "general", name: "General", detail: "Основная Flowseal strategy" },
  { id: "alt", name: "ALT", detail: "Fake + fakedsplit" },
  { id: "alt2", name: "ALT2", detail: "Multisplit seqovl 652" },
  { id: "alt3", name: "ALT3", detail: "HostFakeSplit" },
  { id: "simple_fake", name: "Simple Fake", detail: "Fake TLS без split" },
  { id: "fake_tls_auto", name: "Fake TLS Auto", detail: "Auto fake TLS" },
];

export function Dashboard() {
  const { status, profiles, selectedProfiles, diagnostics, loading, settings } = useAppStore();
  const errors = diagnostics.filter((item) => item.status === "error").length;
  const warnings = diagnostics.filter((item) => item.status === "warning").length;
  const engineIssue = diagnostics.find((item) => item.id === "engine_found" && item.status !== "ok");
  const running = status?.status === "running";

  return (
    <div className="page-stack">
      <section className="dashboard-hero">
        <div>
          <span className="eyebrow">Zapret Manager</span>
          <h1>Статус: {status?.message ?? "Загрузка"}</h1>
          <p>Локальное управление профилями через службу. GUI не запускает engine напрямую.</p>
        </div>
        <MainToggle status={status?.status ?? "disabled"} loading={loading.toggle} onToggle={appActions.toggleEnabled} />
      </section>
      <section className="dashboard-section">
        <div className="section-heading">
          <span className="eyebrow">Режимы</span>
          <h2>Выберите один или несколько</h2>
        </div>
        {profiles.length === 0 ? (
          <p className="empty-state">Профили не найдены. Переустановите приложение или проверьте папку profiles рядом с .exe.</p>
        ) : (
          <div className="mode-grid">
            {profiles.map((profile) => {
            const selected = selectedProfiles.includes(profile.id);
            return (
              <label className={`mode-option ${selected ? "is-selected" : ""}`} key={profile.id}>
                <input
                  checked={selected}
                  onChange={(event) => appActions.setProfileSelected(profile.id, event.target.checked)}
                  type="checkbox"
                />
                <span>
                  <strong>{profile.name}</strong>
                  <small>{profile.status} · {profile.version} · риск {profile.risk_level}</small>
                </span>
              </label>
            );
            })}
          </div>
        )}
      </section>
      <section className="dashboard-section">
        <div className="section-heading">
          <span className="eyebrow">Стратегия engine</span>
          <h2>Если не заработало, переключите вариант и включите снова</h2>
        </div>
        <div className="strategy-grid">
          {engineStrategies.map((strategy) => {
            const selected = (settings?.engine_strategy ?? "general") === strategy.id;
            return (
              <button
                className={`strategy-option ${selected ? "is-selected" : ""}`}
                disabled={running || loading.settings}
                key={strategy.id}
                onClick={() => appActions.setEngineStrategy(strategy.id)}
                type="button"
              >
                <strong>{strategy.name}</strong>
                <small>{strategy.detail}</small>
              </button>
            );
          })}
        </div>
        {running && <p className="hint-line">Чтобы сменить стратегию, сначала нажмите “Выключить”.</p>}
      </section>
      <section className="status-grid">
        <StatusCard
          icon={ShieldCheck}
          label="Диагностика"
          value={errors > 0 ? "Ошибка" : warnings > 0 ? "Внимание" : "OK"}
          detail={errors > 0 ? `${errors} ошибок` : warnings > 0 ? `${warnings} предупреждений` : "Блокеров нет"}
          tone={errors > 0 ? "error" : warnings > 0 ? "warning" : "ok"}
        />
        <StatusCard icon={Layers3} label="Режимы" value={selectedProfiles.length.toString()} detail={selectedProfiles.join(", ") || "Не выбрано"} />
        <StatusCard
          icon={Activity}
          label="Engine"
          value={engineIssue ? "Ошибка" : settings?.engine_strategy ?? "general"}
          detail={engineIssue?.action ?? engineIssue?.problem ?? "Manifest и hash проверены"}
          tone={engineIssue ? "error" : "ok"}
        />
      </section>
    </div>
  );
}
