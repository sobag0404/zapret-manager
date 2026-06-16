import { CheckCircle2, FlaskConical } from "lucide-react";
import { Profile } from "../api/tauriCommands";

interface ProfileCardProps {
  profile: Profile;
  selected: boolean;
  loading?: boolean;
  onToggle: (id: string, enabled: boolean) => void;
}

export function ProfileCard({ profile, selected, loading, onToggle }: ProfileCardProps) {
  return (
    <article className={`profile-card ${selected ? "is-active" : ""}`}>
      <header>
        <label className="profile-check">
          <input
            checked={selected}
            disabled={loading}
            onChange={(event) => onToggle(profile.id, event.target.checked)}
            type="checkbox"
          />
          <span>
            <h3>{profile.name}</h3>
            <p>{profile.description}</p>
          </span>
        </label>
        {profile.status === "stable" ? <CheckCircle2 size={18} /> : <FlaskConical size={18} />}
      </header>
      <dl>
        <div><dt>Статус</dt><dd>{profile.status}</dd></div>
        <div><dt>Версия</dt><dd>{profile.version}</dd></div>
        <div><dt>Fallback</dt><dd>{profile.fallback_profiles.join(", ")}</dd></div>
        <div><dt>Риск</dt><dd>{profile.risk_level}</dd></div>
      </dl>
      <p className="muted">{profile.notes}</p>
    </article>
  );
}
