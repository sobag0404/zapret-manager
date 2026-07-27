import { Activity, Layers3, ShieldCheck } from "lucide-react";
import { MainToggle } from "../components/MainToggle";
import { StatusCard } from "../components/StatusCard";
import { appActions, useAppStore } from "../store/appStore";

const engineStrategies = [
  {
    id: "telegram_web_runtime_syndata",
    name: "Telegram Web: Runtime IP / Snd",
    status: "experimental",
    detail: "DNS-ответы перед запуском; только TCP 443",
  },
  {
    id: "telegram_web_runtime_wssize",
    name: "Telegram Web: Runtime IP / WS",
    status: "experimental",
    detail: "DNS-ответы перед запуском; только TCP 443",
  },
  {
    id: "whatsapp_web_runtime_syndata",
    name: "WhatsApp Web: Runtime IP / Snd",
    status: "experimental",
    detail: "DNS-ответы перед запуском; только TCP 443",
  },
  {
    id: "whatsapp_web_runtime_wssize",
    name: "WhatsApp Web: Runtime IP / WS",
    status: "experimental",
    detail: "DNS-ответы перед запуском; только TCP 443",
  },
  { id: "alt", name: "2 ALT", status: "unknown", detail: "Fake + fakedsplit" },
  { id: "alt3", name: "4 ALT3", status: "unknown", detail: "HostFakeSplit" },
  { id: "simple_fake", name: "5 Simple Fake", status: "unknown", detail: "Fake TLS без split" },
  { id: "general", name: "1 General", status: "experimental", detail: "Базовая стратегия Flowseal" },
  {
    id: "fake_tls_auto",
    name: "6 Fake TLS Auto",
    status: "experimental",
    detail: "Проверено на тестовой сети: YouTube Web",
  },
];

const telegramCandidates = ["telegram_web_runtime_syndata", "telegram_web_runtime_wssize", "telegram_web_runtime_dup"];
const whatsappCandidates = ["whatsapp_web_runtime_syndata", "whatsapp_web_runtime_wssize"];

export function Dashboard() {
  const { status, profiles, selectedProfiles, diagnostics, loading, settings } = useAppStore();
  const errors = diagnostics.filter((item) => item.status === "error").length;
  const warnings = diagnostics.filter((item) => item.status === "warning").length;
  const engineIssue = diagnostics.find((item) => item.id === "engine_found" && item.status !== "ok");
  const running = status?.status === "running";
  const singleMessagingProfile = selectedProfiles.length === 1 && ["telegram", "whatsapp"].includes(selectedProfiles[0])
    ? selectedProfiles[0]
    : null;
  const activeStrategy = settings?.engine_strategy ?? "general";
  const candidateLabel = singleMessagingProfile
    ? `${singleMessagingProfile === "telegram" ? "Telegram" : "WhatsApp"}: ${activeStrategy} (${(singleMessagingProfile === "telegram" ? telegramCandidates : whatsappCandidates).includes(activeStrategy) ? "A/B" : "другая"})`
    : null;

  return (
    <div className="page-stack">
      <section className="dashboard-hero">
        <div>
          <span className="eyebrow">Zapret Manager</span>
          <h1>Статус: {status?.message ?? "Загрузка"}</h1>
          <p>Локальное управление профилями через backend. Перед включением создаётся snapshot; при выключении останавливается управляемый engine и очищается его runtime.</p>
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
                  <input checked={selected} onChange={(event) => appActions.setProfileSelected(profile.id, event.target.checked)} type="checkbox" />
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
          <span className="eyebrow">Стратегия engine</span>
          <h2>Все стратегии экспериментальные. ALT5 и legacy ALT6 скрыты; «6 Fake TLS Auto» является отдельным вариантом.</h2>
        </div>
        {singleMessagingProfile && (
          <div className="inline-action">
            <span>{candidateLabel}</span>
            <button className="secondary-button" disabled={running || loading.settings} onClick={appActions.nextProfileStrategy} type="button">
              Следующая стратегия
            </button>
          </div>
        )}
        <div className="strategy-grid">
          {engineStrategies.map((strategy) => {
            const selected = (settings?.engine_strategy ?? "general") === strategy.id;
            const profileRestricted =
              (strategy.id.startsWith("telegram_web_runtime_") && singleMessagingProfile !== "telegram") ||
              (strategy.id.startsWith("whatsapp_web_runtime_") && singleMessagingProfile !== "whatsapp");
            return (
              <button
                className={`strategy-option ${selected ? "is-selected" : ""}`}
                disabled={running || loading.settings || profileRestricted}
                key={strategy.id}
                onClick={() => appActions.setEngineStrategy(strategy.id)}
                type="button"
              >
                <strong>{strategy.name}</strong>
                <small>{strategy.status} / {strategy.detail}</small>
              </button>
            );
          })}
        </div>
        {running && <p className="hint-line">Чтобы сменить стратегию, сначала нажмите «Выключить».</p>}
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
