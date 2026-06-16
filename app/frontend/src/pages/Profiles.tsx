import { ProfileCard } from "../components/ProfileCard";
import { appActions, useAppStore } from "../store/appStore";

export function Profiles() {
  const { profiles, selectedProfiles, loading } = useAppStore();
  return (
    <div className="page-stack">
      <header className="page-header">
        <div>
          <span className="eyebrow">Профили</span>
          <h1>Режимы</h1>
        </div>
      </header>
      {profiles.length === 0 ? (
        <p className="empty-state">Профили не найдены. Переустановите приложение или проверьте bundled resources.</p>
      ) : (
        <section className="card-grid">
          {profiles.map((profile) => (
            <ProfileCard
              key={profile.id}
              profile={profile}
              selected={selectedProfiles.includes(profile.id)}
              loading={loading[`profile:${profile.id}`]}
              onToggle={appActions.setProfileSelected}
            />
          ))}
        </section>
      )}
    </div>
  );
}
