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
            <option value="alt4">ALT4</option>
            <option value="alt5">ALT5</option>
            <option value="alt6">ALT6</option>
            <option value="alt7">ALT7</option>
            <option value="alt8">ALT8</option>
            <option value="alt9">ALT9</option>
            <option value="alt10">ALT10</option>
            <option value="alt11">ALT11</option>
            <option value="alt12">ALT12</option>
            <option value="simple_fake">Simple Fake</option>
            <option value="simple_fake_alt">Simple Fake ALT</option>
            <option value="simple_fake_alt2">Simple Fake ALT2</option>
            <option value="fake_tls_auto">Fake TLS Auto</option>
            <option value="fake_tls_auto_alt">Fake TLS Auto ALT</option>
            <option value="fake_tls_auto_alt2">Fake TLS Auto ALT2</option>
            <option value="fake_tls_auto_alt3">Fake TLS Auto ALT3</option>
          </select>
        </label>
        <div className="settings-note wide-field">
          Автостарт, ручной путь к engine и ручной путь к логам скрыты из v1.2: эти параметры останутся выключенными, пока не будут реализованы полностью.
        </div>
      </section>
    </form>
  );
}
