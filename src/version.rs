//! Deteccion de la version del ejecutable.
//!
//! Usa las mismas "firmas" que plugin-sdk: un valor conocido (0x94BF) en una direccion
//! que cambia segun la version. Las lecturas se protegen con VirtualQuery para no
//! crashear si la direccion no esta mapeada (p. ej. en la version de Steam con ASLR).

use core::ffi::c_void;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Memory::{
    VirtualQuery, MEMORY_BASIC_INFORMATION, MEM_COMMIT, PAGE_GUARD, PAGE_NOACCESS,
};
use windows_sys::Win32::System::SystemServices::IMAGE_DOS_HEADER;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameVersion {
    Us10,
    Eu10,
    Us101,
    Eu101,
    Steam30,
    Unknown,
}

impl GameVersion {
    pub fn describe(self) -> &'static str {
        match self {
            GameVersion::Us10 => "1.0 US (soportada)",
            GameVersion::Eu10 => "1.0 EU (no soportada)",
            GameVersion::Us101 => "1.01 US (no soportada)",
            GameVersion::Eu101 => "1.01 EU (no soportada)",
            GameVersion::Steam30 => "3.0 Steam (no soportada, hay que hacer downgrade)",
            GameVersion::Unknown => "desconocida",
        }
    }
}

/// Lee un u32 si la direccion esta mapeada y es legible.
pub fn try_read_u32(addr: usize) -> Option<u32> {
    unsafe {
        let mut mbi: MEMORY_BASIC_INFORMATION = core::mem::zeroed();
        let n = VirtualQuery(
            addr as *const c_void,
            &mut mbi,
            core::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
        );
        if n == 0 || mbi.State != MEM_COMMIT || mbi.Protect & (PAGE_NOACCESS | PAGE_GUARD) != 0 {
            return None;
        }
        let end = mbi.BaseAddress as usize + mbi.RegionSize;
        if addr + 4 > end {
            return None;
        }
        Some(core::ptr::read_unaligned(addr as *const u32))
    }
}

/// Timestamp de compilacion del ejecutable (cabecera PE). Util para el log.
pub fn exe_timestamp() -> u32 {
    unsafe {
        let base = GetModuleHandleW(core::ptr::null()) as usize;
        let dos = &*(base as *const IMAGE_DOS_HEADER);
        // IMAGE_NT_HEADERS: firma (4) + IMAGE_FILE_HEADER { Machine(2), NumberOfSections(2), TimeDateStamp(4) }
        core::ptr::read_unaligned((base + dos.e_lfanew as usize + 8) as *const u32)
    }
}

pub fn exe_base() -> usize {
    unsafe { GetModuleHandleW(core::ptr::null()) as usize }
}

pub fn detect() -> GameVersion {
    crate::log!(
        "Ejecutable: base=0x{:08X} timestamp=0x{:08X}",
        exe_base(),
        exe_timestamp()
    );
    const SIG: u32 = 0x94BF;
    let checks = [
        (0x0082_457C, GameVersion::Us10),
        (0x0082_45BC, GameVersion::Eu10),
        (0x0082_52FC, GameVersion::Us101),
        (0x0082_533C, GameVersion::Eu101),
        (0x0085_EC4A, GameVersion::Steam30),
    ];
    for (addr, ver) in checks {
        if try_read_u32(addr) == Some(SIG) {
            return ver;
        }
    }
    // Steam 2014: compilado el 2014-12-01 y con ASLR. Lo reconocemos por timestamp
    // para dar un mensaje claro aunque no coincida ninguna firma.
    if exe_timestamp() == 0x547C_D2E9 {
        return GameVersion::Steam30;
    }
    GameVersion::Unknown
}
