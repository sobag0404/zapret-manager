use std::fs;
use std::path::Path;
use zapret_manager_core::{load_profile, Profile, Result};

pub fn load_profiles(root: &Path) -> Result<Vec<Profile>> {
    let profiles_dir = root.join("profiles");
    let mut profiles = Vec::new();
    if profiles_dir.exists() {
        for entry in fs::read_dir(&profiles_dir)
            .map_err(|source| zapret_manager_core::io_error(&profiles_dir, source))?
        {
            let path = entry
                .map_err(|source| zapret_manager_core::io_error(&profiles_dir, source))?
                .path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("json")
                && path.file_name().and_then(|name| name.to_str()) != Some("profile.schema.json")
            {
                profiles.push(load_profile(&path)?);
            }
        }
    }
    profiles.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(profiles)
}
