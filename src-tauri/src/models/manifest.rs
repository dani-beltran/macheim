use serde::{Deserialize, Serialize};

/// Thunderstore mod manifest (manifest.json inside ZIP)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub name: String,
    pub version_number: String,
    pub description: String,
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub website_url: String,
    #[serde(default)]
    pub author: Option<String>,
}
