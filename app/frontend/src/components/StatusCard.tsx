import type { LucideIcon } from "lucide-react";

interface StatusCardProps {
  icon: LucideIcon;
  label: string;
  value: string;
  detail: string;
  tone?: "ok" | "warning" | "error";
}

export function StatusCard({ icon: Icon, label, value, detail, tone = "ok" }: StatusCardProps) {
  return (
    <article className={`status-card tone-${tone}`}>
      <Icon size={20} aria-hidden="true" />
      <span>{label}</span>
      <strong>{value}</strong>
      <small>{detail}</small>
    </article>
  );
}
