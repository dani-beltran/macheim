use serde::{Deserialize, Serialize};

use super::installed_mod::InstalledMod;

/// A mod profile containing a set of mods and their configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub mods: Vec<InstalledMod>,
    pub created_at: String,
    pub updated_at: String,
}

impl Profile {
    pub fn new(name: String, description: String) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            name,
            description,
            mods: Vec::new(),
            created_at: now.clone(),
            updated_at: now,
        }
    }

    pub fn touch(&mut self) {
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }
}
