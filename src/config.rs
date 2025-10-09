use std::{fs, path::PathBuf};

use crate::Result;

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct Config {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    #[serde(default, skip_serializing_if = "Upload::is_default")]
    pub upload: Upload,
}

// fn yes() -> bool {
//     true
// }

#[derive(Clone, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Upload {
    // #[serde(default = "yes")]
    // pub public_access: bool,
    #[serde(default)]
    pub public_metadata: bool,
}

impl Upload {
    fn is_default(&self) -> bool {
        *self == Default::default()
    }
}

impl Config {
    pub fn load() -> Self {
        fs::read_to_string(Self::path())
            .ok()
            .and_then(|contents| toml::from_str(&contents).unwrap())
            .unwrap_or_default()
    }

    pub fn store(&self) -> Result<()> {
        Self::ensure_dir();
        fs::write(Self::path(), toml::to_string_pretty(self).unwrap()).unwrap();
        Ok(())
    }

    pub fn dir() -> PathBuf {
        dirs::config_dir().unwrap()
    }

    fn ensure_dir() {
        fs::create_dir_all(Self::dir()).ok();
    }

    pub fn path() -> PathBuf {
        let mut path = Self::dir();
        path.push("gyazo.toml");
        path
    }
}
