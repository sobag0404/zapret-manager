import { CheckCircle2, CircleSlash, TriangleAlert, XCircle } from "lucide-react";
import { DiagnosticItem as DiagnosticItemModel } from "../api/tauriCommands";

const icons = {
  ok: CheckCircle2,
  warning: TriangleAlert,
  error: XCircle,
  skipped: CircleSlash,
};

const labels = {
  ok: "OK",
  warning: "Внимание",
  error: "Ошибка",
  skipped: "Пропущено",
};

export function DiagnosticItem({ item }: { item: DiagnosticItemModel }) {
  const Icon = icons[item.status];
  return (
    <article className={`diagnostic-item status-${item.status}`}>
      <Icon size={18} aria-hidden="true" />
      <div>
        <strong>{item.title}</strong>
        <p>{item.problem ? `${item.problem} ${item.action ?? ""}` : item.action ?? "OK"}</p>
      </div>
      <span>{labels[item.status]}</span>
    </article>
  );
}
