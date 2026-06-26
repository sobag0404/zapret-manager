import { PlayCircle, Wifi } from "lucide-react";
import { DiagnosticItem } from "../components/DiagnosticItem";
import { appActions, useAppStore } from "../store/appStore";

export function Diagnostics() {
  const { diagnostics, loading } = useAppStore();
  return (
    <div className="page-stack">
      <header className="page-header with-action">
        <div>
          <span className="eyebrow">Диагностика</span>
          <h1>Почему не работает</h1>
        </div>
        <div className="button-row">
          <button className="secondary-button" disabled={loading.dns} onClick={appActions.runDnsCheck}>
            <Wifi size={17} aria-hidden="true" />
            DNS
          </button>
          <button className="secondary-button" disabled={loading.connectivity} onClick={appActions.runConnectivity}>
            Проверить доступность
          </button>
          <button className="primary-button" disabled={loading.diagnostics} onClick={appActions.runDiagnostics}>
            <PlayCircle size={17} aria-hidden="true" />
            Проверить всё
          </button>
        </div>
      </header>
      <section className="list-panel">
        {diagnostics.map((item) => (
          <DiagnosticItem item={item} key={item.id} />
        ))}
      </section>
    </div>
  );
}
