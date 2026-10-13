//! CTimer: reloj del juego (tiempo en ms, paso de frame, pausas).
//!
//! Primer hook del proyecto. De momento solo *envuelve* `CTimer::Update`:
//! llama a la original y escribe en el log cada cierto numero de frames.
//! Si en el log aparecen esas lineas mientras juegas, el sistema de hooks funciona.
//!
//! Direcciones (1.0 US) tomadas de gta-reversed; marcadas como pendientes de
//! verificar en NOTES.md hasta confirmarlas en el juego.

use super::global;
use crate::hooks::{Hook, Kind};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

pub mod addr {
    pub const UPDATE: usize = 0x0056_1B10; // static void CTimer::Update()

    pub const SN_TIME_IN_MILLISECONDS: usize = 0x00B7_CB84; // uint32
    pub const FRAME_COUNTER: usize = 0x00B7_CB4C; // uint32
    pub const TIME_STEP: usize = 0x00B7_CB5C; // float
}

pub fn time_in_ms() -> u32 {
    unsafe { *global::<u32>(addr::SN_TIME_IN_MILLISECONDS) }
}
pub fn frame_counter() -> u32 {
    unsafe { *global::<u32>(addr::FRAME_COUNTER) }
}
pub fn time_step() -> f32 {
    unsafe { *global::<f32>(addr::TIME_STEP) }
}

static ORIG_UPDATE: AtomicUsize = AtomicUsize::new(0);
static CALLS: AtomicU32 = AtomicU32::new(0);

/// Cada cuantas llamadas escribimos en el log (~1 min a 30 fps).
const LOG_EVERY: u32 = 1800;

extern "C" fn update() {
    let orig: extern "C" fn() =
        unsafe { core::mem::transmute(ORIG_UPDATE.load(Ordering::Acquire)) };
    orig();

    let n = CALLS.fetch_add(1, Ordering::Relaxed);
    if n == 0 || n.is_multiple_of(LOG_EVERY) {
        crate::log!(
            "CTimer::Update #{n}: time={} ms, frame={}, timestep={:.3}",
            time_in_ms(),
            frame_counter(),
            time_step()
        );
    }
}

pub fn register(v: &mut Vec<Hook>) {
    v.push(Hook {
        name: "CTimer::Update",
        addr: addr::UPDATE,
        kind: Kind::Wrap {
            func: update as *const () as usize,
            original: &ORIG_UPDATE,
        },
        enabled_by_default: true,
    });
}
