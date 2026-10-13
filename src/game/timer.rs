//! CTimer: reloj del juego (tiempo en ms, paso de frame, pausas).
//!
//! `CTimer::Update` esta reescrita entera en Rust a partir del desensamblado de 1.0 US
//! (0x561B10-0x561C6E). `UpdateVariables` sigue siendo la original de momento.
//!
//! Funcionamiento (traducido del ensamblador):
//!   1. Si no hay funcion de reloj, no hace nada.
//!   2. Calcula los FPS a partir del tiempo "no recortado" del frame anterior.
//!   3. Desplaza el historial de tiempos (Previous, PPrevious, ...).
//!   4. Lee el contador de alta resolucion y calcula los ticks transcurridos.
//!   5. Si el juego no esta en pausa, aplica la escala de tiempo (camara lenta).
//!   6. Acumula el tiempo en modo pausa (avanza siempre).
//!   7. Si esta en pausa, el tiempo de juego no avanza (delta = 0).
//!   8. `UpdateVariables(delta)` actualiza ms, timestep, etc. y suma un frame.

use super::global;
use crate::hooks::{Hook, Kind};
use std::sync::atomic::{AtomicU32, Ordering};

pub mod addr {
    // Funciones
    pub const UPDATE: usize = 0x0056_1B10; // static void CTimer::Update()
    pub const UPDATE_VARIABLES: usize = 0x0056_18D0; // static void CTimer::UpdateVariables(float)

    // Variables globales
    pub const TIMER_FUNCTION: usize = 0x00B7_CB28; // u64 (__cdecl *)()
    pub const TIMER_DIVIDER: usize = 0x00B7_CB2C; // u32, ticks por ms
    pub const OLD_PC_TIMER: usize = 0x00B7_CB38; // u64, ultimo valor del contador
    pub const UNK_B7CB40: usize = 0x00B7_CB40; // u8, Update lo pone a 1 y 0x5618C0 a 0
    pub const CODE_PAUSE: usize = 0x00B7_CB48; // bool
    pub const USER_PAUSE: usize = 0x00B7_CB49; // bool
    pub const FRAME_COUNTER: usize = 0x00B7_CB4C; // u32
    pub const GAME_FPS: usize = 0x00B7_CB50; // f32
    pub const TIME_STEP: usize = 0x00B7_CB5C; // f32
    pub const TIME_SCALE: usize = 0x00B7_CB64; // f32
    pub const PREV_TIME_NON_CLIPPED: usize = 0x00B7_CB68; // u32
    pub const PPPP_PREV_TIME: usize = 0x00B7_CB6C; // u32
    pub const PPP_PREV_TIME: usize = 0x00B7_CB70; // u32
    pub const PP_PREV_TIME: usize = 0x00B7_CB74; // u32
    pub const PREV_TIME: usize = 0x00B7_CB78; // u32
    pub const TIME_PAUSE_MODE: usize = 0x00B7_CB7C; // u32
    pub const TIME_NON_CLIPPED: usize = 0x00B7_CB80; // u32
    pub const TIME_IN_MILLISECONDS: usize = 0x00B7_CB84; // u32
}

pub fn time_in_ms() -> u32 {
    unsafe { *global::<u32>(addr::TIME_IN_MILLISECONDS) }
}
pub fn frame_counter() -> u32 {
    unsafe { *global::<u32>(addr::FRAME_COUNTER) }
}
pub fn time_step() -> f32 {
    unsafe { *global::<f32>(addr::TIME_STEP) }
}
pub fn game_fps() -> f32 {
    unsafe { *global::<f32>(addr::GAME_FPS) }
}
pub fn is_paused() -> bool {
    unsafe { *global::<bool>(addr::USER_PAUSE) || *global::<bool>(addr::CODE_PAUSE) }
}

/// `CTimer::UpdateVariables` original (pendiente de reescribir).
fn update_variables(time_elapsed: f32) {
    let f: extern "C" fn(f32) = unsafe { core::mem::transmute(addr::UPDATE_VARIABLES) };
    f(time_elapsed)
}

/// Reimplementacion de `CTimer::Update` (0x561B10).
pub extern "C" fn update() {
    unsafe {
        let timer_fn = *global::<usize>(addr::TIMER_FUNCTION);
        if timer_fn == 0 {
            return;
        }
        let timer_fn: extern "C" fn() -> u64 = core::mem::transmute(timer_fn);

        // FPS del frame anterior (resta sin signo, como el original)
        let non_clipped = *global::<u32>(addr::TIME_NON_CLIPPED);
        let prev_non_clipped = global::<u32>(addr::PREV_TIME_NON_CLIPPED);
        let fps = 1000.0f32 / non_clipped.wrapping_sub(*prev_non_clipped) as f32;
        *global::<u8>(addr::UNK_B7CB40) = 1;

        // Historial de tiempos
        *global::<u32>(addr::PPPP_PREV_TIME) = *global::<u32>(addr::PPP_PREV_TIME);
        *global::<u32>(addr::PPP_PREV_TIME) = *global::<u32>(addr::PP_PREV_TIME);
        *global::<u32>(addr::PP_PREV_TIME) = *global::<u32>(addr::PREV_TIME);
        *global::<u32>(addr::PREV_TIME) = *global::<u32>(addr::TIME_IN_MILLISECONDS);
        *prev_non_clipped = non_clipped;
        *global::<f32>(addr::GAME_FPS) = fps;

        // Ticks transcurridos desde el frame anterior
        let now = timer_fn();
        let old = global::<u64>(addr::OLD_PC_TIMER);
        let mut delta = now.wrapping_sub(*old) as f32;
        if !is_paused() {
            delta *= *global::<f32>(addr::TIME_SCALE);
        }
        *old = now;

        // El tiempo en modo pausa avanza siempre (truncado, como _ftol)
        let divider = *global::<u32>(addr::TIMER_DIVIDER) as f32;
        let pause_mode = global::<u32>(addr::TIME_PAUSE_MODE);
        *pause_mode = (delta / divider + *pause_mode as f32) as i64 as u32;

        if is_paused() {
            delta = 0.0;
        }
        update_variables(delta);
        *global::<u32>(addr::FRAME_COUNTER) += 1;
    }

    log_periodic();
}

static CALLS: AtomicU32 = AtomicU32::new(0);
/// Cada cuantos frames escribimos en el log.
const LOG_EVERY: u32 = 1800;

fn log_periodic() {
    let n = CALLS.fetch_add(1, Ordering::Relaxed);
    if n == 0 || n.is_multiple_of(LOG_EVERY) {
        crate::log!(
            "CTimer::Update (Rust) #{n}: time={} ms, frame={}, timestep={:.3}, fps={:.0}, pausa={}",
            time_in_ms(),
            frame_counter(),
            time_step(),
            game_fps(),
            is_paused()
        );
    }
}

pub fn register(v: &mut Vec<Hook>) {
    v.push(Hook {
        name: "CTimer::Update",
        addr: addr::UPDATE,
        kind: Kind::Replace {
            func: update as *const () as usize,
        },
        enabled_by_default: true,
    });
}
