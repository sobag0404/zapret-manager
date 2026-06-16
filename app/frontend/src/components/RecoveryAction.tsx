import { Wrench } from "lucide-react";

interface RecoveryActionProps {
  id: string;
  title: string;
  description: string;
  loading?: boolean;
  onRun: (id: string) => void;
}

export function RecoveryAction({ id, title, description, loading, onRun }: RecoveryActionProps) {
  return (
    <article className="recovery-action">
      <Wrench size={18} aria-hidden="true" />
      <div>
        <strong>{title}</strong>
        <p>{description}</p>
      </div>
      <button className="secondary-button" disabled={loading} onClick={() => onRun(id)}>
        {loading ? "..." : "Выполнить"}
      </button>
    </article>
  );
}
