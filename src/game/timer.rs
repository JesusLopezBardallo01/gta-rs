//! CTimer: reloj del juego (tiempo en ms, paso de frame, pausas).
//!
//! Clase completa reescrita en Rust a partir del desensamblado de 1.0 US
//! (bloque 0x5617A0-0x561C6E). Solo queda sin hook 0x5617D0, una funcion vacia de
//! 1 byte (`ret`) que no cabe un `jmp` y no hace nada.
//!
//! Las llamadas internas (Update -> UpdateVariables, Update -> funcion de reloj) pasan
//! por las direcciones del juego. Asi, si un hook se desactiva en `gta_rs.ini`, se usa la
//! version original de esa funcion y el resto sigue siendo Rust.
//!
//! Los `bool` del juego se leen como `u8` (`!= 0`): un byte con otro valor que 0/1 seria
//! comportamiento indefinido como `bool` de Rust.
//!
//! Unidades: el reloj da "ciclos" (ticks de QueryPerformanceCounter, o ms si no hay QPC).
//! `TIMER_DIVIDER` = ciclos por ms. `TIME_STEP` 1.0 = 1/50 s (20 ms).

use super::global;
use crate::hooks::{Hook, Kind};
use std::sync::atomic::{AtomicU32, Ordering};
use windows_sys::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};

pub mod addr {
    // Funciones
    pub const TIMER_QPC: usize = 0x0056_17A0; // u64 GetTimerQPC()
    pub const TIMER_RS: usize = 0x0056_17C0; // u64 GetTimerRs()
    pub const INITIALISE: usize = 0x0056_17E0; // void CTimer::Initialise()
    pub const SHUTDOWN: usize = 0x0056_18C0; // void CTimer::Shutdown()
    pub const UPDATE_VARIABLES: usize = 0x0056_18D0; // void CTimer::UpdateVariables(float)
    pub const SUSPEND: usize = 0x0056_19D0; // void CTimer::Suspend()
    pub const RESUME: usize = 0x0056_1A00; // void CTimer::Resume()
    pub const GET_CYCLES_PER_MS: usize = 0x0056_1A40; // u32 CTimer::GetCyclesPerMillisecond()
    pub const GET_CYCLES_PER_FRAME: usize = 0x0056_1A50; // u32 CTimer::GetCyclesPerFrame()
    pub const GET_CURRENT_TIME_IN_CYCLES: usize = 0x0056_1A80; // u32 CTimer::GetCurrentTimeInCycles()
    pub const STOP: usize = 0x0056_1AA0; // void CTimer::Stop()
    pub const IS_SLOW_MOTION: usize = 0x0056_1AD0; // bool CTimer::GetIsSlowMotionActive()
    pub const START_USER_PAUSE: usize = 0x0056_1AF0; // void CTimer::StartUserPause()
    pub const END_USER_PAUSE: usize = 0x0056_1B00; // void CTimer::EndUserPause()
    pub const UPDATE: usize = 0x0056_1B10; // void CTimer::Update()

    pub const RS_TIMER: usize = 0x0074_5270; // u32 RsTimer() (RenderWare skeleton, ms)

    // Variables globales
    pub const TIMER_FUNCTION: usize = 0x00B7_CB28; // u64 (__cdecl *)()
    pub const TIMER_DIVIDER: usize = 0x00B7_CB2C; // u32, ciclos por ms
    pub const SUSPEND_TIMER: usize = 0x00B7_CB30; // u64, reloj al suspender
    pub const OLD_PC_TIMER: usize = 0x00B7_CB38; // u64, reloj del frame anterior
    pub const TIMER_ACTIVE: usize = 0x00B7_CB40; // bool, Update -> 1, Initialise/Shutdown/Stop -> 0
    pub const SUSPEND_DEPTH: usize = 0x00B7_CB44; // u32, anidamiento de Suspend/Resume
    pub const CODE_PAUSE: usize = 0x00B7_CB48; // bool
    pub const USER_PAUSE: usize = 0x00B7_CB49; // bool
    pub const FRAME_COUNTER: usize = 0x00B7_CB4C; // u32
    pub const GAME_FPS: usize = 0x00B7_CB50; // f32
    pub const OLD_TIME_STEP: usize = 0x00B7_CB54; // f32
    pub const TIME_STEP_NON_CLIPPED: usize = 0x00B7_CB58; // f32
    pub const TIME_STEP: usize = 0x00B7_CB5C; // f32
    pub const UNK_B7CB60: usize = 0x00B7_CB60; // f32, Initialise -> -1.0
    pub const TIME_SCALE: usize = 0x00B7_CB64; // f32
    pub const PREV_TIME_NON_CLIPPED: usize = 0x00B7_CB68; // u32
    pub const PPPP_PREV_TIME: usize = 0x00B7_CB6C; // u32
    pub const PPP_PREV_TIME: usize = 0x00B7_CB70; // u32
    pub const PP_PREV_TIME: usize = 0x00B7_CB74; // u32
    pub const PREV_TIME: usize = 0x00B7_CB78; // u32
    pub const TIME_PAUSE_MODE: usize = 0x00B7_CB7C; // u32
    pub const TIME_NON_CLIPPED: usize = 0x00B7_CB80; // u32
    pub const TIME_IN_MILLISECONDS: usize = 0x00B7_CB84; // u32
    pub const UNK_B7CB88: usize = 0x00B7_CB88; // bool, Initialise -> 0
    pub const UNK_B7CB89: usize = 0x00B7_CB89; // bool, Initialise -> 0
    pub const UNK_C7C714: usize = 0x00C7_C714; // bool, si esta activo no se fuerza el timestep minimo
}

/// Limites del paso de frame (constantes del exe: 0x858C28, 0x858C58, 0x858B3C, 0x858C14).
const TIME_STEP_PER_FRAME_DELTA: f32 = 0.05; // 1 ms = 0.05 timestep
const MIN_TIME_STEP_UNPAUSED: f32 = 0.01;
const MAX_TIME_STEP: f32 = 3.0;
const MIN_TIME_STEP: f32 = 0.00001;
/// Un frame "nominal" a 50 Hz son 20 ms (0x858BA4).
const MS_PER_FRAME: f32 = 20.0;
/// Recorte del avance del reloj de juego por frame (ms).
const MAX_FRAME_MS: u32 = 300;

macro_rules! g {
    ($t:ty, $a:ident) => {
        *global::<$t>(addr::$a)
    };
}

pub fn time_in_ms() -> u32 {
    unsafe { g!(u32, TIME_IN_MILLISECONDS) }
}
pub fn frame_counter() -> u32 {
    unsafe { g!(u32, FRAME_COUNTER) }
}
pub fn time_step() -> f32 {
    unsafe { g!(f32, TIME_STEP) }
}
pub fn game_fps() -> f32 {
    unsafe { g!(f32, GAME_FPS) }
}
pub fn is_paused() -> bool {
    unsafe { (g!(u8, USER_PAUSE) != 0) || (g!(u8, CODE_PAUSE) != 0) }
}

/// Conversion de float a entero como `_ftol` (trunca hacia cero, 64 bits).
fn ftol(x: f32) -> u32 {
    x as i64 as u32
}

fn qpc() -> u64 {
    let mut v = 0i64;
    unsafe { QueryPerformanceCounter(&mut v) };
    v as u64
}

// ---------------------------------------------------------------------------------------
// Funciones de reloj
// ---------------------------------------------------------------------------------------

/// 0x5617A0: reloj de alta resolucion.
pub extern "C" fn timer_qpc() -> u64 {
    qpc()
}

/// 0x5617C0: reloj de reserva si no hay QPC (milisegundos del skeleton de RenderWare).
pub extern "C" fn timer_rs() -> u64 {
    let rs_timer: extern "C" fn() -> u32 = unsafe { core::mem::transmute(addr::RS_TIMER) };
    rs_timer() as u64
}

/// Llama a la funcion de reloj activa (`ms_fnTimerFunction`).
unsafe fn read_timer() -> u64 {
    let f: extern "C" fn() -> u64 = core::mem::transmute(g!(usize, TIMER_FUNCTION));
    f()
}

// ---------------------------------------------------------------------------------------
// Ciclo de vida
// ---------------------------------------------------------------------------------------

/// 0x5617E0
pub extern "C" fn initialise() {
    unsafe {
        g!(u8, USER_PAUSE) = 0;
        g!(u8, CODE_PAUSE) = 0;
        g!(u8, UNK_B7CB88) = 0;
        g!(u8, UNK_B7CB89) = 0;
        g!(u32, PPPP_PREV_TIME) = 0;
        g!(u32, PPP_PREV_TIME) = 0;
        g!(u32, PP_PREV_TIME) = 0;
        g!(u32, PREV_TIME) = 0;
        g!(u32, PREV_TIME_NON_CLIPPED) = 0;
        g!(u32, TIME_IN_MILLISECONDS) = 0;
        g!(u32, FRAME_COUNTER) = 0;
        g!(u32, SUSPEND_DEPTH) = 0;
        g!(u8, TIMER_ACTIVE) = 0;
        g!(f32, GAME_FPS) = 0.0;

        g!(u32, TIME_NON_CLIPPED) = 1;
        g!(u32, TIME_PAUSE_MODE) = 1;
        g!(f32, TIME_SCALE) = 1.0;
        g!(f32, TIME_STEP) = 1.0;
        g!(f32, OLD_TIME_STEP) = 1.0;
        g!(f32, UNK_B7CB60) = -1.0;

        // Como el original: solo la parte baja de la frecuencia, dividida entre 1000
        let mut freq = 0i64;
        if QueryPerformanceFrequency(&mut freq) != 0 {
            g!(u32, TIMER_DIVIDER) = (freq as u64 as u32) / 1000;
            g!(usize, TIMER_FUNCTION) = addr::TIMER_QPC;
        } else {
            g!(u32, TIMER_DIVIDER) = 1;
            g!(usize, TIMER_FUNCTION) = addr::TIMER_RS;
        }
        g!(u64, OLD_PC_TIMER) = read_timer();

        crate::log!(
            "CTimer::Initialise (Rust): {} ciclos/ms, reloj {}",
            g!(u32, TIMER_DIVIDER),
            if g!(usize, TIMER_FUNCTION) == addr::TIMER_QPC {
                "QPC"
            } else {
                "RsTimer"
            }
        );
    }
}

/// 0x5618C0
pub extern "C" fn shutdown() {
    unsafe { g!(u8, TIMER_ACTIVE) = 0 };
}

/// 0x561AA0: iguala el historial al instante actual (evita un salto de tiempo al volver).
pub extern "C" fn stop() {
    unsafe {
        let now = g!(u32, TIME_IN_MILLISECONDS);
        g!(u32, PPPP_PREV_TIME) = now;
        g!(u32, PPP_PREV_TIME) = now;
        g!(u32, PP_PREV_TIME) = now;
        g!(u32, PREV_TIME) = now;
        g!(u8, TIMER_ACTIVE) = 0;
        g!(u32, PREV_TIME_NON_CLIPPED) = g!(u32, TIME_NON_CLIPPED);
    }
}

/// 0x5619D0: congela el reloj (p. ej. al perder el foco). Admite anidamiento.
pub extern "C" fn suspend() {
    unsafe {
        if g!(u8, TIMER_ACTIVE) == 0 {
            return;
        }
        let depth = g!(u32, SUSPEND_DEPTH).wrapping_add(1);
        g!(u32, SUSPEND_DEPTH) = depth;
        if depth <= 1 {
            g!(u64, SUSPEND_TIMER) = read_timer();
        }
    }
}

/// 0x561A00: descongela; el tiempo suspendido no cuenta para el juego.
pub extern "C" fn resume() {
    unsafe {
        if g!(u8, TIMER_ACTIVE) == 0 {
            return;
        }
        let depth = g!(u32, SUSPEND_DEPTH).wrapping_sub(1);
        g!(u32, SUSPEND_DEPTH) = depth;
        if depth == 0 {
            let elapsed = read_timer().wrapping_sub(g!(u64, SUSPEND_TIMER));
            g!(u64, OLD_PC_TIMER) = g!(u64, OLD_PC_TIMER).wrapping_add(elapsed);
        }
    }
}

// ---------------------------------------------------------------------------------------
// Consultas y pausas
// ---------------------------------------------------------------------------------------

/// 0x561A40
pub extern "C" fn get_cycles_per_millisecond() -> u32 {
    unsafe { g!(u32, TIMER_DIVIDER) }
}

/// 0x561A50
pub extern "C" fn get_cycles_per_frame() -> u32 {
    unsafe { ftol(g!(u32, TIMER_DIVIDER) as f32 * MS_PER_FRAME) }
}

/// 0x561A80: ciclos desde el ultimo frame (usa QPC directamente, 32 bits bajos).
pub extern "C" fn get_current_time_in_cycles() -> u32 {
    unsafe { (qpc() as u32).wrapping_sub(g!(u64, OLD_PC_TIMER) as u32) }
}

/// 0x561AD0
pub extern "C" fn get_is_slow_motion_active() -> bool {
    unsafe { g!(f32, TIME_SCALE) < 1.0 }
}

/// 0x561AF0
pub extern "C" fn start_user_pause() {
    unsafe { g!(u8, USER_PAUSE) = 1 };
}

/// 0x561B00
pub extern "C" fn end_user_pause() {
    unsafe { g!(u8, USER_PAUSE) = 0 };
}

// ---------------------------------------------------------------------------------------
// Bucle por frame
// ---------------------------------------------------------------------------------------

/// 0x5618D0: convierte ciclos transcurridos en ms y timestep.
pub extern "C" fn update_variables(time_elapsed: f32) {
    unsafe {
        let frame_delta = time_elapsed / g!(u32, TIMER_DIVIDER) as f32; // ms

        g!(u32, TIME_NON_CLIPPED) = ftol(g!(u32, TIME_NON_CLIPPED) as f32 + frame_delta);
        g!(f32, TIME_STEP_NON_CLIPPED) = TIME_STEP_PER_FRAME_DELTA * frame_delta;

        g!(u32, TIME_IN_MILLISECONDS) =
            g!(u32, TIME_IN_MILLISECONDS).wrapping_add(ftol(frame_delta).min(MAX_FRAME_MS));

        // Sin pausa (ni la de 0xC7C714) el paso nunca baja de 0.01
        if g!(f32, TIME_STEP_NON_CLIPPED) < MIN_TIME_STEP_UNPAUSED
            && g!(u8, USER_PAUSE) == 0
            && g!(u8, CODE_PAUSE) == 0
            && g!(u8, UNK_C7C714) == 0
        {
            g!(f32, TIME_STEP_NON_CLIPPED) = MIN_TIME_STEP_UNPAUSED;
        }

        g!(f32, OLD_TIME_STEP) = g!(f32, TIME_STEP);

        // min(paso, 3.0); con NaN el original elige 3.0
        let nc = g!(f32, TIME_STEP_NON_CLIPPED);
        let step = if nc < MAX_TIME_STEP {
            nc
        } else {
            MAX_TIME_STEP
        };
        g!(f32, TIME_STEP) = if step > MIN_TIME_STEP {
            step
        } else {
            MIN_TIME_STEP
        };
    }
}

/// Llama a UpdateVariables a traves de la direccion del juego (respeta gta_rs.ini).
unsafe fn call_update_variables(time_elapsed: f32) {
    let f: extern "C" fn(f32) = core::mem::transmute(addr::UPDATE_VARIABLES);
    f(time_elapsed)
}

/// 0x561B10: avance del reloj una vez por frame.
///
///   1. Si no hay funcion de reloj, no hace nada.
///   2. Calcula los FPS a partir del tiempo "no recortado" del frame anterior.
///   3. Desplaza el historial de tiempos (Previous, PPrevious, ...).
///   4. Lee el reloj y calcula los ciclos transcurridos.
///   5. Si el juego no esta en pausa, aplica la escala de tiempo (camara lenta).
///   6. Acumula el tiempo en modo pausa (avanza siempre).
///   7. Si esta en pausa, el tiempo de juego no avanza (delta = 0).
///   8. `UpdateVariables(delta)` y un frame mas.
pub extern "C" fn update() {
    unsafe {
        if g!(usize, TIMER_FUNCTION) == 0 {
            return;
        }

        let non_clipped = g!(u32, TIME_NON_CLIPPED);
        let fps = 1000.0f32 / non_clipped.wrapping_sub(g!(u32, PREV_TIME_NON_CLIPPED)) as f32;
        g!(u8, TIMER_ACTIVE) = 1;

        g!(u32, PPPP_PREV_TIME) = g!(u32, PPP_PREV_TIME);
        g!(u32, PPP_PREV_TIME) = g!(u32, PP_PREV_TIME);
        g!(u32, PP_PREV_TIME) = g!(u32, PREV_TIME);
        g!(u32, PREV_TIME) = g!(u32, TIME_IN_MILLISECONDS);
        g!(u32, PREV_TIME_NON_CLIPPED) = non_clipped;
        g!(f32, GAME_FPS) = fps;

        let now = read_timer();
        let mut delta = now.wrapping_sub(g!(u64, OLD_PC_TIMER)) as f32;
        if !is_paused() {
            delta *= g!(f32, TIME_SCALE);
        }
        g!(u64, OLD_PC_TIMER) = now;

        let divider = g!(u32, TIMER_DIVIDER) as f32;
        g!(u32, TIME_PAUSE_MODE) = ftol(delta / divider + g!(u32, TIME_PAUSE_MODE) as f32);

        if is_paused() {
            delta = 0.0;
        }
        call_update_variables(delta);
        g!(u32, FRAME_COUNTER) = g!(u32, FRAME_COUNTER).wrapping_add(1);
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
    let hooks: [(&'static str, usize, usize); 14] = [
        (
            "CTimer::GetTimerQPC",
            addr::TIMER_QPC,
            timer_qpc as *const () as usize,
        ),
        (
            "CTimer::GetTimerRs",
            addr::TIMER_RS,
            timer_rs as *const () as usize,
        ),
        (
            "CTimer::Initialise",
            addr::INITIALISE,
            initialise as *const () as usize,
        ),
        (
            "CTimer::Shutdown",
            addr::SHUTDOWN,
            shutdown as *const () as usize,
        ),
        (
            "CTimer::UpdateVariables",
            addr::UPDATE_VARIABLES,
            update_variables as *const () as usize,
        ),
        (
            "CTimer::Suspend",
            addr::SUSPEND,
            suspend as *const () as usize,
        ),
        ("CTimer::Resume", addr::RESUME, resume as *const () as usize),
        (
            "CTimer::GetCyclesPerMillisecond",
            addr::GET_CYCLES_PER_MS,
            get_cycles_per_millisecond as *const () as usize,
        ),
        (
            "CTimer::GetCyclesPerFrame",
            addr::GET_CYCLES_PER_FRAME,
            get_cycles_per_frame as *const () as usize,
        ),
        (
            "CTimer::GetCurrentTimeInCycles",
            addr::GET_CURRENT_TIME_IN_CYCLES,
            get_current_time_in_cycles as *const () as usize,
        ),
        ("CTimer::Stop", addr::STOP, stop as *const () as usize),
        (
            "CTimer::GetIsSlowMotionActive",
            addr::IS_SLOW_MOTION,
            get_is_slow_motion_active as *const () as usize,
        ),
        (
            "CTimer::StartUserPause",
            addr::START_USER_PAUSE,
            start_user_pause as *const () as usize,
        ),
        (
            "CTimer::EndUserPause",
            addr::END_USER_PAUSE,
            end_user_pause as *const () as usize,
        ),
    ];
    for (name, addr, func) in hooks {
        v.push(Hook {
            name,
            addr,
            kind: Kind::Replace { func },
            enabled_by_default: true,
        });
    }
    v.push(Hook {
        name: "CTimer::Update",
        addr: addr::UPDATE,
        kind: Kind::Replace {
            func: update as *const () as usize,
        },
        enabled_by_default: true,
    });
}
