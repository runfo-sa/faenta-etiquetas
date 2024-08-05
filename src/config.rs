use serde::{Deserialize, Serialize};

/// [yama's] Config
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// Indica si las etiquetas son de 300dpi o no.
    pub is_dpi300: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self { is_dpi300: true }
    }
}
