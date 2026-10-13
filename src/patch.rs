//! Primitivas para parchear el ejecutable en memoria.
//!
//! - `write_bytes`: escribe bytes cambiando temporalmente la proteccion de la pagina.
//! - `inject_jmp`: sustituye una funcion original por la nuestra con un `jmp rel32`
//!   (reemplazo completo, como `InjectHook` de re3/gta-reversed).
//!
//! - `wrap`: igual, pero generando antes un trampolin para poder llamar a la original.

use core::ffi::c_void;
use windows_sys::Win32::System::Diagnostics::Debug::FlushInstructionCache;
use windows_sys::Win32::System::Memory::{
    VirtualAlloc, VirtualProtect, MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READWRITE,
};
use windows_sys::Win32::System::Threading::GetCurrentProcess;

pub unsafe fn write_bytes(addr: usize, bytes: &[u8]) -> bool {
    let mut old = 0u32;
    if VirtualProtect(
        addr as *const c_void,
        bytes.len(),
        PAGE_EXECUTE_READWRITE,
        &mut old,
    ) == 0
    {
        return false;
    }
    core::ptr::copy_nonoverlapping(bytes.as_ptr(), addr as *mut u8, bytes.len());
    let mut tmp = 0u32;
    VirtualProtect(addr as *const c_void, bytes.len(), old, &mut tmp);
    FlushInstructionCache(GetCurrentProcess(), addr as *const c_void, bytes.len());
    true
}

/// Escribe `jmp dest` en `at`. La funcion original queda inutilizada desde ese punto.
pub unsafe fn inject_jmp(at: usize, dest: usize) -> bool {
    let rel = (dest as i64 - (at as i64 + 5)) as i32;
    let mut buf = [0xE9u8, 0, 0, 0, 0];
    buf[1..].copy_from_slice(&rel.to_le_bytes());
    write_bytes(at, &buf)
}

/// Crea un trampolin para `at`: copia (reubicando) las instrucciones que va a pisar
/// el `jmp` de 5 bytes y anade un salto de vuelta a la original. Llamar al trampolin
/// equivale a llamar a la funcion original sin hook.
///
/// Devuelve la direccion del trampolin y cuantos bytes del original se han copiado.
pub unsafe fn make_trampoline(at: usize) -> Result<(usize, usize), String> {
    use iced_x86::{
        BlockEncoder, BlockEncoderOptions, Code, Decoder, DecoderOptions, Instruction,
        InstructionBlock, Mnemonic,
    };

    // Decodifica instrucciones hasta cubrir al menos 5 bytes.
    let code = core::slice::from_raw_parts(at as *const u8, 32);
    let mut dec = Decoder::with_ip(32, code, at as u64, DecoderOptions::NONE);
    let mut instrs = Vec::new();
    let mut len = 0usize;
    while len < 5 {
        let ins = dec.decode();
        if ins.is_invalid() {
            return Err(format!("instruccion invalida en 0x{:08X}", at + len));
        }
        if ins.mnemonic() == Mnemonic::Ret && len + ins.len() < 5 {
            return Err("funcion demasiado corta para un jmp de 5 bytes".into());
        }
        len += ins.len();
        instrs.push(ins);
    }
    // Salto de vuelta a la original, justo despues de lo copiado.
    instrs.push(
        Instruction::with_branch(Code::Jmp_rel32_32, (at + len) as u64)
            .map_err(|e| e.to_string())?,
    );

    let mem = VirtualAlloc(
        core::ptr::null(),
        64,
        MEM_COMMIT | MEM_RESERVE,
        PAGE_EXECUTE_READWRITE,
    ) as usize;
    if mem == 0 {
        return Err("VirtualAlloc fallo".into());
    }
    let block = InstructionBlock::new(&instrs, mem as u64);
    let res =
        BlockEncoder::encode(32, block, BlockEncoderOptions::NONE).map_err(|e| e.to_string())?;
    if res.code_buffer.len() > 64 {
        return Err("trampolin demasiado grande".into());
    }
    core::ptr::copy_nonoverlapping(
        res.code_buffer.as_ptr(),
        mem as *mut u8,
        res.code_buffer.len(),
    );
    FlushInstructionCache(
        GetCurrentProcess(),
        mem as *const c_void,
        res.code_buffer.len(),
    );
    Ok((mem, len))
}

/// Envuelve la funcion en `at` con `dest`. Devuelve el trampolin a la original.
pub unsafe fn wrap(at: usize, dest: usize) -> Result<usize, String> {
    let (tramp, copied) = make_trampoline(at)?;
    if !inject_jmp(at, dest) {
        return Err("VirtualProtect fallo".into());
    }
    // Rellena con int3 los bytes sobrantes de la instruccion pisada (solo estetico).
    if copied > 5 {
        write_bytes(at + 5, &vec![0xCC; copied - 5]);
    }
    Ok(tramp)
}

/// Lee `n` bytes (para el log de diagnostico antes de parchear).
pub unsafe fn peek(addr: usize, n: usize) -> Vec<u8> {
    core::slice::from_raw_parts(addr as *const u8, n).to_vec()
}
