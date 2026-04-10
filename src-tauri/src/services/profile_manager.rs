use std::path::{Path, PathBuf};

use tracing::{debug, info, warn};

use crate::error::{AppError, AppResult};
use crate::models::{InstalledMod, Profile};
use crate::services::thunderstore_client;

const DEFAULT_PROFILE_NAME: &str = "Default";

/// Get the profiles base directory.
pub fn get_profiles_dir() -> PathBuf {
    thunderstore_client::get_app_data_dir().join("profiles")
}

/// Get a specific profile's directory.
pub fn get_profile_dir(name: &str) -> PathBuf {
    get_profiles_dir().join(name)
}

/// Get the path to a profile's metadata file.
fn get_profile_file(name: &str) -> PathBuf {
    get_profile_dir(name).join("profile.json")
}

/// Ensure the default profile exists.
pub fn ensure_default_profile() -> AppResult<()> {
    let default_dir = get_profile_dir(DEFAULT_PROFILE_NAME);
    if !default_dir.exists() {
        create_profile(DEFAULT_PROFILE_NAME, "Default mod profile")?;
    }
    Ok(())
}

/// Create a new profile.
pub fn create_profile(name: &str, description: &str) -> AppResult<Profile> {
    let profile_dir = get_profile_dir(name);

    if profile_dir.exists() {
        return Err(AppError::Profile(format!(
            "Profile '{}' already exists",
            name
        )));
    }

    // Create directory structure
    std::fs::create_dir_all(&profile_dir)?;
    std::fs::create_dir_all(profile_dir.join("BepInEx/plugins"))?;
    std::fs::create_dir_all(profile_dir.join("BepInEx/config"))?;
    std::fs::create_dir_all(profile_dir.join("BepInEx/patchers"))?;

    let profile = Profile::new(name.to_string(), description.to_string());
    save_profile(&profile)?;

    info!("Created profile: {}", name);
    Ok(profile)
}

/// Load a profile's metadata.
pub fn load_profile(name: &str) -> AppResult<Profile> {
    let profile_file = get_profile_file(name);

    if !profile_file.exists() {
        return Err(AppError::Profile(format!(
            "Profile '{}' not found",
            name
        )));
    }

    let content = std::fs::read_to_string(&profile_file)?;
    let profile: Profile = serde_json::from_str(&content)?;
    Ok(profile)
}

/// Save a profile's metadata.
pub fn save_profile(profile: &Profile) -> AppResult<()> {
    let profile_file = get_profile_file(&profile.name);
    let profile_dir = get_profile_dir(&profile.name);
    std::fs::create_dir_all(&profile_dir)?;

    let content = serde_json::to_string_pretty(profile)?;
    std::fs::write(&profile_file, content)?;
    debug!("Saved profile: {}", profile.name);
    Ok(())
}

/// List all profiles.
pub fn list_profiles() -> AppResult<Vec<Profile>> {
    let profiles_dir = get_profiles_dir();
    let mut profiles = Vec::new();

    if !profiles_dir.exists() {
        // Create default profile
        ensure_default_profile()?;
    }

    if profiles_dir.exists() {
        for entry in std::fs::read_dir(&profiles_dir)? {
            let entry = entry?;
            if entry.path().is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                match load_profile(&name) {
                    Ok(profile) => profiles.push(profile),
                    Err(e) => {
                        warn!("Failed to load profile '{}': {}", name, e);
                    }
                }
            }
        }
    }

    // Sort by name, with Default first
    profiles.sort_by(|a, b| {
        if a.name == DEFAULT_PROFILE_NAME {
            std::cmp::Ordering::Less
        } else if b.name == DEFAULT_PROFILE_NAME {
            std::cmp::Ordering::Greater
        } else {
            a.name.cmp(&b.name)
        }
    });

    Ok(profiles)
}

/// Delete a profile.
pub fn delete_profile(name: &str) -> AppResult<()> {
    if name == DEFAULT_PROFILE_NAME {
        return Err(AppError::Profile(
            "Cannot delete the default profile".to_string(),
        ));
    }

    let profile_dir = get_profile_dir(name);
    if !profile_dir.exists() {
        return Err(AppError::Profile(format!(
            "Profile '{}' not found",
            name
        )));
    }

    std::fs::remove_dir_all(&profile_dir)?;
    info!("Deleted profile: {}", name);
    Ok(())
}

/// Clone a profile.
pub fn clone_profile(source_name: &str, new_name: &str) -> AppResult<Profile> {
    let source_dir = get_profile_dir(source_name);
    let new_dir = get_profile_dir(new_name);

    if !source_dir.exists() {
        return Err(AppError::Profile(format!(
            "Source profile '{}' not found",
            source_name
        )));
    }

    if new_dir.exists() {
        return Err(AppError::Profile(format!(
            "Profile '{}' already exists",
            new_name
        )));
    }

    // Copy directory
    copy_dir_recursive(&source_dir, &new_dir)?;

    // Update the profile metadata
    let mut profile = load_profile(new_name).unwrap_or_else(|_| {
        Profile::new(new_name.to_string(), format!("Cloned from {}", source_name))
    });
    profile.name = new_name.to_string();
    profile.description = format!("Cloned from {}", source_name);
    profile.touch();
    save_profile(&profile)?;

    info!("Cloned profile '{}' to '{}'", source_name, new_name);
    Ok(profile)
}

/// Switch active profile: copy profile's BepInEx content to the game directory.
pub fn switch_profile(profile_name: &str, game_root: &Path) -> AppResult<()> {
    let profile_dir = get_profile_dir(profile_name);

    if !profile_dir.exists() {
        return Err(AppError::Profile(format!(
            "Profile '{}' not found",
            profile_name
        )));
    }

    info!("Switching to profile: {}", profile_name);

    let game_bepinex = game_root.join("BepInEx");
    let profile_bepinex = profile_dir.join("BepInEx");

    // Save current game state to outgoing profile would require knowing
    // the current profile -- handled by the command layer.

    // Clear current BepInEx plugins, patchers, config
    let dirs_to_sync = ["plugins", "patchers", "config"];
    for dir_name in &dirs_to_sync {
        let game_sub = game_bepinex.join(dir_name);
        let profile_sub = profile_bepinex.join(dir_name);

        // Remove current game content
        if game_sub.exists() {
            std::fs::remove_dir_all(&game_sub)?;
        }

        // Copy profile content to game
        if profile_sub.exists() {
            copy_dir_recursive(&profile_sub, &game_sub)?;
        } else {
            std::fs::create_dir_all(&game_sub)?;
        }
    }

    // Also copy plugins_disabled if it exists
    let game_disabled = game_bepinex.join("plugins_disabled");
    let profile_disabled = profile_bepinex.join("plugins_disabled");
    if game_disabled.exists() {
        std::fs::remove_dir_all(&game_disabled)?;
    }
    if profile_disabled.exists() {
        copy_dir_recursive(&profile_disabled, &game_disabled)?;
    }

    info!("Switched to profile: {}", profile_name);
    Ok(())
}

/// Save the current game BepInEx state back to a profile.
pub fn save_game_state_to_profile(profile_name: &str, game_root: &Path) -> AppResult<()> {
    let profile_dir = get_profile_dir(profile_name);
    let game_bepinex = game_root.join("BepInEx");
    let profile_bepinex = profile_dir.join("BepInEx");

    if !game_bepinex.exists() {
        return Ok(());
    }

    let dirs_to_sync = ["plugins", "patchers", "config", "plugins_disabled"];
    for dir_name in &dirs_to_sync {
        let game_sub = game_bepinex.join(dir_name);
        let profile_sub = profile_bepinex.join(dir_name);

        if game_sub.exists() {
            if profile_sub.exists() {
                std::fs::remove_dir_all(&profile_sub)?;
            }
            copy_dir_recursive(&game_sub, &profile_sub)?;
        }
    }

    debug!("Saved game state to profile: {}", profile_name);
    Ok(())
}

/// Add a mod to a profile's metadata.
pub fn add_mod_to_profile(profile_name: &str, installed_mod: InstalledMod) -> AppResult<()> {
    let mut profile = load_profile(profile_name)?;

    // Remove existing entry if upgrading
    profile
        .mods
        .retain(|m| m.full_name != installed_mod.full_name);

    profile.mods.push(installed_mod);
    profile.touch();
    save_profile(&profile)?;

    Ok(())
}

/// Remove a mod from a profile's metadata.
pub fn remove_mod_from_profile(profile_name: &str, mod_full_name: &str) -> AppResult<()> {
    let mut profile = load_profile(profile_name)?;
    profile.mods.retain(|m| m.full_name != mod_full_name);
    profile.touch();
    save_profile(&profile)?;
    Ok(())
}

/// Update mod enabled status in profile metadata.
pub fn update_mod_enabled(
    profile_name: &str,
    mod_full_name: &str,
    enabled: bool,
) -> AppResult<()> {
    let mut profile = load_profile(profile_name)?;

    if let Some(mod_entry) = profile.mods.iter_mut().find(|m| m.full_name == mod_full_name) {
        mod_entry.enabled = enabled;
        profile.touch();
        save_profile(&profile)?;
    }

    Ok(())
}

/// Export a profile to a JSON file (metadata only, mods can be re-downloaded).
pub fn export_profile(profile_name: &str) -> AppResult<String> {
    let profile = load_profile(profile_name)?;
    let json = serde_json::to_string_pretty(&profile)?;
    Ok(json)
}

/// Import a profile from JSON.
pub fn import_profile(json: &str, new_name: Option<&str>) -> AppResult<Profile> {
    let mut profile: Profile = serde_json::from_str(json)?;

    if let Some(name) = new_name {
        profile.name = name.to_string();
    }

    let profile_dir = get_profile_dir(&profile.name);
    if profile_dir.exists() {
        return Err(AppError::Profile(format!(
            "Profile '{}' already exists",
            profile.name
        )));
    }

    // Create directory structure
    std::fs::create_dir_all(&profile_dir)?;
    std::fs::create_dir_all(profile_dir.join("BepInEx/plugins"))?;
    std::fs::create_dir_all(profile_dir.join("BepInEx/config"))?;
    std::fs::create_dir_all(profile_dir.join("BepInEx/patchers"))?;

    profile.touch();
    save_profile(&profile)?;

    info!("Imported profile: {}", profile.name);
    Ok(profile)
}

// --- Helpers ---

fn copy_dir_recursive(src: &Path, dst: &Path) -> AppResult<()> {
    std::fs::create_dir_all(dst)?;

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}
