//! gta-rs: reimplementacion incremental de GTA San Andreas en Rust.
//!
//! Esta DLL se carga dentro del juego original (renombrada a `.asi`, via ASI Loader).
//! Al arrancar:
//!   1. abre `gta_rs.log` en la carpeta del juego,
//!   2. comprueba que el ejecutable es GTA SA 1.0 US (la unica version soportada),
//!   3. instala los hooks activados en `gta_rs.ini`.
//!
//! Cada funcion del juego que reescribimos en Rust se registra en `hooks::registry()`.

#![allow(clippy::missing_safety_doc, clippy::not_unsafe_ptr_arg_deref)]

pub mod config;
pub mod game;
pub mod hooks;
pub mod log;
pub mod patch;
pub mod version;

use core::ffi::c_void;
use windows_sys::Win32::Foundation::{BOOL, HINSTANCE, TRUE};
use windows_sys::Win32::System::LibraryLoader::DisableThreadLibraryCalls;
use windows_sys::Win32::System::SystemServices::DLL_PROCESS_ATTACH;

#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn DllMain(module: HINSTANCE, reason: u32, _reserved: *mut c_void) -> BOOL {
    if reason == DLL_PROCESS_ATTACH {
        unsafe { DisableThreadLibraryCalls(module) };
        init();
    }
    TRUE
}

fn init() {
    log::init("gta_rs.log");
    log!("gta-rs {} cargado", env!("CARGO_PKG_VERSION"));

    let ver = version::detect();
    log!("Version del juego detectada: {}", ver.describe());

    if ver != version::GameVersion::Us10 {
        log!("ERROR: solo se soporta GTA SA 1.0 US. No se instala ningun hook.");
        log!("       Haz downgrade del ejecutable (ver README).");
        return;
    }

    let cfg = config::Config::load("gta_rs.ini");
    hooks::install_all(&cfg);
}
