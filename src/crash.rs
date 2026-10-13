//! Registro de crashes: si el juego revienta, escribe en `gta_rs.log` donde y por que.
//!
//! Se instala como filtro de excepciones no controladas. Solo escribe en el log y deja
//! que Windows siga con su comportamiento normal (cerrar el juego).

use crate::log;
use windows_sys::Win32::Foundation::{EXCEPTION_ACCESS_VIOLATION, HMODULE};
use windows_sys::Win32::System::Diagnostics::Debug::{
    SetUnhandledExceptionFilter, EXCEPTION_POINTERS,
};
use windows_sys::Win32::System::LibraryLoader::{
    GetModuleFileNameW, GetModuleHandleExW, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
    GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
};

const EXCEPTION_CONTINUE_SEARCH: i32 = 0;

pub fn install() {
    unsafe { SetUnhandledExceptionFilter(Some(filter)) };
}

/// Nombre del modulo (exe o dll) que contiene `addr` y desplazamiento dentro de el.
fn module_of(addr: usize) -> String {
    unsafe {
        let mut module: HMODULE = core::ptr::null_mut();
        let flags =
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT;
        if GetModuleHandleExW(flags, addr as *const u16, &mut module) == 0 {
            return "modulo desconocido".into();
        }
        let mut buf = [0u16; 260];
        let n = GetModuleFileNameW(module, buf.as_mut_ptr(), buf.len() as u32) as usize;
        let path = String::from_utf16_lossy(&buf[..n]);
        let name = path.rsplit('\\').next().unwrap_or(&path).to_string();
        format!("{name}+0x{:X}", addr - module as usize)
    }
}

unsafe extern "system" fn filter(info: *const EXCEPTION_POINTERS) -> i32 {
    let rec = &*(*info).ExceptionRecord;
    let addr = rec.ExceptionAddress as usize;
    log!("=== CRASH ===");
    log!(
        "Excepcion 0x{:08X} en 0x{:08X} ({})",
        rec.ExceptionCode as u32,
        addr,
        module_of(addr)
    );
    if rec.ExceptionCode == EXCEPTION_ACCESS_VIOLATION && rec.NumberParameters >= 2 {
        let kind = match rec.ExceptionInformation[0] {
            0 => "leer",
            1 => "escribir",
            8 => "ejecutar",
            _ => "acceder a",
        };
        log!(
            "Intento de {kind} la direccion 0x{:08X}",
            rec.ExceptionInformation[1]
        );
    }
    log_registers(info);
    EXCEPTION_CONTINUE_SEARCH
}

#[cfg(target_arch = "x86")]
unsafe fn log_registers(info: *const EXCEPTION_POINTERS) {
    let c = &*(*info).ContextRecord;
    log!(
        "EAX={:08X} EBX={:08X} ECX={:08X} EDX={:08X} ESI={:08X} EDI={:08X}",
        c.Eax,
        c.Ebx,
        c.Ecx,
        c.Edx,
        c.Esi,
        c.Edi
    );
    log!("EBP={:08X} ESP={:08X} EIP={:08X}", c.Ebp, c.Esp, c.Eip);
    // Direcciones de retorno candidatas en la pila (las que caen en el exe o en nuestra dll)
    let mut found = 0;
    for i in 0..256usize {
        let p = c.Esp as usize + i * 4;
        let Some(v) = crate::version::try_read_u32(p) else {
            break;
        };
        let v = v as usize;
        if (0x401000..0x857000).contains(&v) || module_of(v).starts_with("gta_rs") {
            log!("  pila[+0x{:03X}] = 0x{:08X} ({})", i * 4, v, module_of(v));
            found += 1;
            if found >= 16 {
                break;
            }
        }
    }
}

#[cfg(not(target_arch = "x86"))]
unsafe fn log_registers(_info: *const EXCEPTION_POINTERS) {}
