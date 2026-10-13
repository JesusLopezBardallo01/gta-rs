//! Configuracion en `gta_rs.ini` (en la carpeta del juego).
//!
//! Formato minimo, una linea por hook:
//!     CTimer::Update=1
//!     CTimer::Update=0   ; desactivado: el juego usa la funcion original
//!
//! Si un hook no aparece en el fichero, se usa su valor por defecto del registro.
//! Asi, cuando una funcion reescrita falle, se puede desactivar sin recompilar.

use std::collections::HashMap;

#[derive(Default)]
pub struct Config {
    hooks: HashMap<String, bool>,
}

impl Config {
    pub fn load(path: &str) -> Self {
        let mut cfg = Config::default();
        let Ok(text) = std::fs::read_to_string(path) else {
            crate::log!("{path} no encontrado: se usan los valores por defecto");
            return cfg;
        };
        for line in text.lines() {
            let line = line.split(';').next().unwrap_or("").trim();
            if line.is_empty() || line.starts_with('[') {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                cfg.hooks.insert(k.trim().to_string(), v.trim() != "0");
            }
        }
        cfg
    }

    pub fn hook_enabled(&self, name: &str, default: bool) -> bool {
        *self.hooks.get(name).unwrap_or(&default)
    }
}
