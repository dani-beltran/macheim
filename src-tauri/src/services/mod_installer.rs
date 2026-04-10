use std::path::Path;

use tracing::{debug, info};

use crate::error::{AppError, AppResult};
use crate::models::InstalledMod;
use crate::services::gatekeeper;
use crate::services::thunderstore_client;

/// Install a mod from a ZIP downloaded from Thunderstore.
/// Returns the InstalledMod metadata.
pub async fn install_mod(
    author: &str,
    name: &str,
    version: &str,
    download_url: &str,
    description: &str,
    icon: &str,
    dependencies: &[String],
    game_root: &Path,
) -> AppResult<InstalledMod> {
    let full_name = format!("{}-{}", author, name);
    info!("Installing mod: {} v{}", full_name, version);

    // Download the mod ZIP
    let zip_bytes = thunderstore_client::download_mod(download_url).await?;

    // Install from bytes
    install_mod_from_bytes(
        author,
        name,
        version,
        description,
        icon,
        dependencies,
        &zip_bytes,
        game_root,
    )
}

/// Install a mod from ZIP bytes (used by both direct install and dependency install).
pub fn install_mod_from_bytes(
    author: &str,
    name: &str,
    version: &str,
    description: &str,
    icon: &str,
    dependencies: &[String],
    zip_bytes: &[u8],
    game_root: &Path,
) -> AppResult<InstalledMod> {
    let full_name = format!("{}-{}", author, name);

    // Extract ZIP to temp directory for analysis
    let temp_dir = tempfile::tempdir()?;
    extract_zip(zip_bytes, temp_dir.path())?;

    // Analyze the mod structure and copy to appropriate locations
    let bepinex_dir = game_root.join("BepInEx");
    install_mod_files(temp_dir.path(), &full_name, &bepinex_dir)?;

    // Remove quarantine from any .dylib files
    let plugins_dir = bepinex_dir.join("plugins").join(&full_name);
    if plugins_dir.exists() {
        gatekeeper::remove_quarantine_from_dylibs(&plugins_dir)?;
    }

    let installed = InstalledMod {
        full_name,
        author: author.to_string(),
        name: name.to_string(),
        version: version.to_string(),
        description: description.to_string(),
        enabled: true,
        dependencies: dependencies.to_vec(),
        installed_at: chrono::Utc::now().to_rfc3339(),
        icon: icon.to_string(),
    };

    info!("Mod installed: {} v{}", installed.full_name, installed.version);
    Ok(installed)
}

/// Analyze mod structure and copy files to the correct BepInEx directories.
fn install_mod_files(extracted_dir: &Path, mod_name: &str, bepinex_dir: &Path) -> AppResult<()> {
    let plugins_dir = bepinex_dir.join("plugins").join(mod_name);
    let patchers_dir = bepinex_dir.join("patchers");
    let config_dir = bepinex_dir.join("config");

    // Check for standard mod structure
    let has_plugins = extracted_dir.join("plugins").exists();
    let has_patchers = extracted_dir.join("patchers").exists();
    let has_config = extracted_dir.join("config").exists();

    if has_plugins || has_patchers || has_config {
        // Standard Thunderstore mod structure
        if has_plugins {
            std::fs::create_dir_all(&plugins_dir)?;
            copy_dir_contents(&extracted_dir.join("plugins"), &plugins_dir)?;
            debug!("Copied plugins/ to {}", plugins_dir.display());
        }

        if has_patchers {
            std::fs::create_dir_all(&patchers_dir)?;
            copy_dir_contents(&extracted_dir.join("patchers"), &patchers_dir)?;
            debug!("Copied patchers/ to {}", patchers_dir.display());
        }

        if has_config {
            std::fs::create_dir_all(&config_dir)?;
            copy_dir_contents(&extracted_dir.join("config"), &config_dir)?;
            debug!("Copied config/ to {}", config_dir.display());
        }
    } else {
        // No standard structure -- check for DLLs at root level
        let has_root_dlls = has_dll_files(extracted_dir);

        if has_root_dlls {
            // Treat root DLLs as plugins
            std::fs::create_dir_all(&plugins_dir)?;
            copy_files_by_extension(extracted_dir, &plugins_dir, &["dll", "dylib", "so"])?;
            debug!("Copied root DLLs to {}", plugins_dir.display());
        }

        // Also copy any remaining asset files
        copy_non_metadata_files(extracted_dir, &plugins_dir)?;
    }

    Ok(())
}

/// Uninstall a mod by removing its files.
pub fn uninstall_mod(mod_full_name: &str, game_root: &Path) -> AppResult<()> {
    info!("Uninstalling mod: {}", mod_full_name);

    let bepinex_dir = game_root.join("BepInEx");

    // Remove from plugins
    let plugins_dir = bepinex_dir.join("plugins").join(mod_full_name);
    if plugins_dir.exists() {
        std::fs::remove_dir_all(&plugins_dir)?;
        debug!("Removed plugins: {}", plugins_dir.display());
    }

    // Also check plugins_disabled
    let disabled_dir = bepinex_dir.join("plugins_disabled").join(mod_full_name);
    if disabled_dir.exists() {
        std::fs::remove_dir_all(&disabled_dir)?;
        debug!("Removed disabled plugins: {}", disabled_dir.display());
    }

    info!("Mod uninstalled: {}", mod_full_name);
    Ok(())
}

/// Enable or disable a mod by moving it between plugins/ and plugins_disabled/.
pub fn toggle_mod(mod_full_name: &str, enable: bool, game_root: &Path) -> AppResult<bool> {
    let bepinex_dir = game_root.join("BepInEx");
    let plugins_dir = bepinex_dir.join("plugins").join(mod_full_name);
    let disabled_dir = bepinex_dir.join("plugins_disabled").join(mod_full_name);

    if enable {
        // Move from disabled to plugins
        if disabled_dir.exists() {
            std::fs::create_dir_all(bepinex_dir.join("plugins"))?;
            move_dir(&disabled_dir, &plugins_dir)?;
            info!("Enabled mod: {}", mod_full_name);
            Ok(true)
        } else if plugins_dir.exists() {
            // Already enabled
            Ok(true)
        } else {
            Err(AppError::Mod(format!(
                "Mod '{}' files not found",
                mod_full_name
            )))
        }
    } else {
        // Move from plugins to disabled
        if plugins_dir.exists() {
            std::fs::create_dir_all(bepinex_dir.join("plugins_disabled"))?;
            move_dir(&plugins_dir, &disabled_dir)?;
            info!("Disabled mod: {}", mod_full_name);
            Ok(false)
        } else if disabled_dir.exists() {
            // Already disabled
            Ok(false)
        } else {
            Err(AppError::Mod(format!(
                "Mod '{}' files not found",
                mod_full_name
            )))
        }
    }
}

/// Get list of installed mods by scanning the BepInEx/plugins directory.
pub fn scan_installed_mods(game_root: &Path) -> AppResult<Vec<String>> {
    let plugins_dir = game_root.join("BepInEx").join("plugins");
    let mut mods = Vec::new();

    if plugins_dir.exists() {
        for entry in std::fs::read_dir(&plugins_dir)? {
            let entry = entry?;
            if entry.path().is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                // Skip internal BepInEx directories
                if !name.starts_with('.') {
                    mods.push(name);
                }
            }
        }
    }

    Ok(mods)
}

// --- Helper functions ---

fn extract_zip(zip_bytes: &[u8], target: &Path) -> AppResult<()> {
    let cursor = std::io::Cursor::new(zip_bytes);
    let mut archive = zip::ZipArchive::new(cursor)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = target.join(file.mangled_name());

        if file.is_dir() {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut outfile = std::fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }
    }

    Ok(())
}

fn copy_dir_contents(src: &Path, dst: &Path) -> AppResult<()> {
    if !src.is_dir() {
        return Ok(());
    }

    std::fs::create_dir_all(dst)?;

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_contents(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

fn move_dir(src: &Path, dst: &Path) -> AppResult<()> {
    // Try rename first (fast, same filesystem)
    if std::fs::rename(src, dst).is_ok() {
        return Ok(());
    }

    // Fallback: copy then remove
    copy_dir_contents(src, dst)?;
    std::fs::remove_dir_all(src)?;
    Ok(())
}

fn has_dll_files(dir: &Path) -> bool {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Some(ext) = entry.path().extension() {
                if ext == "dll" || ext == "dylib" || ext == "so" {
                    return true;
                }
            }
        }
    }
    false
}

fn copy_files_by_extension(src: &Path, dst: &Path, extensions: &[&str]) -> AppResult<()> {
    std::fs::create_dir_all(dst)?;

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if extensions.iter().any(|e| ext == *e) {
                    let dst_file = dst.join(entry.file_name());
                    std::fs::copy(&path, &dst_file)?;
                }
            }
        }
    }

    Ok(())
}

fn copy_non_metadata_files(src: &Path, dst: &Path) -> AppResult<()> {
    let skip_files = ["manifest.json", "icon.png", "README.md", "CHANGELOG.md", "LICENSE"];

    std::fs::create_dir_all(dst)?;

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if path.is_file() && !skip_files.contains(&name.as_str()) {
            let dst_file = dst.join(&name);
            if !dst_file.exists() {
                std::fs::copy(&path, &dst_file)?;
            }
        } else if path.is_dir() {
            let dir_name = name.to_lowercase();
            // Skip standard Thunderstore directories already handled
            if dir_name != "plugins" && dir_name != "patchers" && dir_name != "config" {
                let dst_subdir = dst.join(&name);
                copy_dir_contents(&path, &dst_subdir)?;
            }
        }
    }

    Ok(())
}
