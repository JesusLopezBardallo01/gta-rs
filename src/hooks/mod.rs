//! Registro central de hooks.
//!
//! Cada funcion del juego que tocamos aparece aqui una vez. Hay dos tipos:
//!
//! - `Replace`: la funcion esta reescrita entera en Rust. Se pone un `jmp` al
//!   principio de la original y nunca vuelve a ejecutarse. Es el objetivo final.
//! - `Wrap`: nuestra funcion se ejecuta en lugar de la original, pero puede llamar
//!   a la original (trampolin generado en `patch::wrap`). Sirve para pruebas, logs, o para ir
//!   reescribiendo una funcion por partes.
//!
//! El estado de cada funcion del juego se lleva en PROGRESS.md.

use crate::config::Config;
use crate::{game, log, patch};
use std::sync::atomic::{AtomicUsize, Ordering};

pub enum Kind {
    Replace {
        func: usize,
    },
    Wrap {
        func: usize,
        original: &'static AtomicUsize,
    },
}

pub struct Hook {
    pub name: &'static str,
    pub addr: usize,
    pub kind: Kind,
    pub enabled_by_default: bool,
}

/// Lista de todos los hooks. Se rellena desde cada modulo de `game`.
fn registry() -> Vec<Hook> {
    let mut v = Vec::new();
    game::timer::register(&mut v);
    v
}

pub fn install_all(cfg: &Config) {
    let hooks = registry();
    let (mut ok, mut off, mut fail) = (0, 0, 0);
    for h in &hooks {
        if !cfg.hook_enabled(h.name, h.enabled_by_default) {
            log!("  [off ] {:<32} 0x{:08X}", h.name, h.addr);
            off += 1;
            continue;
        }
        let bytes = unsafe { patch::peek(h.addr, 8) };
        match unsafe { install(h) } {
            Ok(()) => {
                log!(
                    "  [ ok ] {:<32} 0x{:08X}  bytes originales {:02X?}",
                    h.name,
                    h.addr,
                    bytes
                );
                ok += 1;
            }
            Err(e) => {
                log!("  [FAIL] {:<32} 0x{:08X}  {e}", h.name, h.addr);
                fail += 1;
            }
        }
    }
    log!(
        "Hooks: {ok} activos, {off} desactivados, {fail} fallidos (de {})",
        hooks.len()
    );
}

unsafe fn install(h: &Hook) -> Result<(), String> {
    match h.kind {
        Kind::Replace { func } => {
            if patch::inject_jmp(h.addr, func) {
                Ok(())
            } else {
                Err("VirtualProtect fallo".into())
            }
        }
        Kind::Wrap { func, original } => {
            let tramp = patch::wrap(h.addr, func)?;
            original.store(tramp, Ordering::Release);
            Ok(())
        }
    }
}
