import { Activity, Layers3, ShieldCheck } from "lucide-react";
import { MainToggle } from "../components/MainToggle";
import { StatusCard } from "../components/StatusCard";
import { appActions, useAppStore } from "../store/appStore";

const recommendedStrategies: Record<string, { name: string; detail: string }> = {
  discord: {
    name: "2 ALT",
    detail: "Экспериментально: проверено для Discord Web и Desktop на тестовой сети.",
  },
  youtube: {
    name: "Fake TLS Auto",
    detail: "Экспериментально: проверено для YouTube Web и воспроизведения видео на тестовой сети.",
  },
  "discord+youtube": {
    name: "Discord + YouTube",
    detail: "Один управляемый engine с раздельными Discord и YouTube фильтрами.",
  },
};

export function Dashboard() {
  const { status, profiles, selectedProfiles, diagnostics, loading } = useAppStore();
  const errors = diagnostics.filter((item) => item.status === "error").length;
  const warnings = diagnostics.filter((item) => item.status === "warning").length;
  const engineIssue = diagnostics.find((item) => item.id === "engine_found" && item.status !== "ok");
  const selectedKey = [...selectedProfiles].sort().join("+");
  const recommendation = recommendedStrategies[selectedKey] ?? null;
  const selectionLocked = Boolean(loading["profile-selection"]) || status?.status === "running" || status?.status === "starting" || status?.status === "stopping" || status?.status === "error";

  return (
    <div className="page-stack">
      <section className="dashboard-hero">
        <div>
          <span className="eyebrow">Zapret Manager</span>
          <h1>Статус: {status?.message ?? "Загрузка"}</h1>
          <p>Локальное управление Discord и YouTube. Перед включением создаётся snapshot; при выключении останавливается управляемый engine и очищается его runtime.</p>
        </div>
        <MainToggle status={status?.status ?? "disabled"} loading={loading.toggle} onToggle={appActions.toggleEnabled} />
      </section>

      <section className="dashboard-section">
        <div className="section-heading">
          <span className="eyebrow">Режимы</span>
          <h2>Выберите один или оба режима</h2>
          <p>Discord и YouTube могут работать отдельно или одновременно через один управляемый engine.</p>
        </div>
        {profiles.length === 0 ? (
          <p className="empty-state">Режимы не найдены. Переустановите приложение или проверьте папку profiles рядом с .exe.</p>
        ) : (
          <div className="mode-grid">
            {profiles.map((profile) => {
              const selected = selectedProfiles.includes(profile.id);
              return (
                <label className={`mode-option ${selected ? "is-selected" : ""}`} key={profile.id}>
                  <input checked={selected} disabled={selectionLocked} onChange={(event) => appActions.setProfileSelected(profile.id, event.target.checked)} type="checkbox" />
                  <span>
                    <strong>{profile.name}</strong>
                    <small>{profile.status} / {profile.version} / риск {profile.risk_level}</small>
                  </span>
                </label>
              );
            })}
          </div>
        )}
      </section>

      <section className="dashboard-section">
        <div className="section-heading">
          <span className="eyebrow">Рекомендованная стратегия</span>
          <h2>{recommendation ? recommendation.name : "Выберите Discord, YouTube или оба"}</h2>
          <p>{recommendation ? recommendation.detail : "Стратегия выбирается автоматически по выбранным режимам."}</p>
        </div>
        <p className="hint-line">Ручной выбор и непроверенные варианты скрыты в этой тестовой сборке.</p>
      </section>

      <section className="status-grid">
        <StatusCard
          icon={ShieldCheck}
          label="Диагностика"
          value={errors > 0 ? "Ошибка" : warnings > 0 ? "Внимание" : "OK"}
          detail={errors > 0 ? `${errors} ошибок` : warnings > 0 ? `${warnings} предупреждений` : "Блокеров нет"}
          tone={errors > 0 ? "error" : warnings > 0 ? "warning" : "ok"}
        />
        <StatusCard icon={Layers3} label="Режимы" value={String(selectedProfiles.length)} detail={selectedProfiles.length ? selectedProfiles.join(" + ") : "Не выбраны"} />
        <StatusCard
          icon={Activity}
          label="Engine"
          value={engineIssue ? "Ошибка" : recommendation?.name ?? "ожидание"}
          detail={engineIssue?.action ?? engineIssue?.problem ?? "Manifest и hash проверены"}
          tone={engineIssue ? "error" : "ok"}
        />
      </section>
    </div>
  );
}
