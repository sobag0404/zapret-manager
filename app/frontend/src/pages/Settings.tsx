import { Save } from "lucide-react";
import { FormEvent, useEffect, useState } from "react";
import { AppSettings } from "../api/tauriCommands";
import { appActions, useAppStore } from "../store/appStore";

export function Settings() {
  const { settings, loading } = useAppStore();
  const [draft, setDraft] = useState<AppSettings | null>(settings);

  useEffect(() => setDraft(settings), [settings]);

  if (!draft) return <p className="empty-state">Настройки загружаются.</p>;

  const submit = (event: FormEvent) => {
    event.preventDefault();
    appActions.saveSettings(draft);
  };

  return (
    <form className="page-stack" onSubmit={submit}>
      <header className="page-header with-action">
        <div>
          <span className="eyebrow">Настройки</span>
          <h1>Параметры</h1>
        </div>
        <button className="primary-button" disabled={loading.settings} type="submit">
          <Save size={17} aria-hidden="true" />
          Сохранить
        </button>
      </header>
      <section className="settings-grid">
        <label className="switch-row">
          <input checked={draft.autostart} onChange={(event) => setDraft({ ...draft, autostart: event.target.checked })} type="checkbox" />
          <span>Запускать вместе с Windows</span>
        </label>
        <label className="switch-row">
          <input checked={draft.safety_mode} onChange={(event) => setDraft({ ...draft, safety_mode: event.target.checked })} type="checkbox" />
          <span>Режим безопасности</span>
        </label>
        <label className="switch-row">
          <input checked={draft.allow_vpn_conflict} onChange={(event) => setDraft({ ...draft, allow_vpn_conflict: event.target.checked })} type="checkbox" />
          <span>Разрешить запуск при активном VPN</span>
        </label>
        <label>
          Канал стратегий
          <select value={draft.strategy_channel} onChange={(event) => setDraft({ ...draft, strategy_channel: event.target.value })}>
            <option value="stable">stable</option>
            <option value="experimental">experimental</option>
          </select>
        </label>
        <label>
          Engine strategy
          <select value={draft.engine_strategy} onChange={(event) => setDraft({ ...draft, engine_strategy: event.target.value })}>
            <option value="general">General</option>
            <option value="alt">ALT</option>
            <option value="alt2">ALT2</option>
            <option value="alt3">ALT3</option>
            <option value="simple_fake">Simple Fake</option>
            <option value="fake_tls_auto">Fake TLS Auto</option>
          </select>
        </label>
        <label>
          Путь к логам
          <input value={draft.logs_path} onChange={(event) => setDraft({ ...draft, logs_path: event.target.value })} />
        </label>
        <label>
          Путь к engine
          <input value={draft.engine_path} onChange={(event) => setDraft({ ...draft, engine_path: event.target.value })} />
        </label>
      </section>
    </form>
  );
}
