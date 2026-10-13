//! Reimplementacion de las clases del juego, una por modulo.
//!
//! Convencion: cada modulo tiene
//!   - `addr`: direcciones (1.0 US) de funciones y variables globales originales,
//!   - las variables globales como accesores sobre esas direcciones,
//!   - las funciones reescritas,
//!   - `register()`, que anade sus hooks al registro.

pub mod file_loader;
pub mod timer;

/// Acceso a una variable global del ejecutable original.
///
/// # Safety
/// `addr` tiene que ser una direccion valida de 1.0 US con el tipo `T`.
#[inline(always)]
pub unsafe fn global<T>(addr: usize) -> &'static mut T {
    &mut *(addr as *mut T)
}

/// Declara funciones originales del juego (`__cdecl`) como funciones Rust.
///
/// ```ignore
/// original_cdecl! {
///     /// CFileMgr::CloseFile
///     pub fn close_file(f: *mut c_void) -> i32 = 0x5389D0;
/// }
/// ```
#[macro_export]
macro_rules! original_cdecl {
    ($( $(#[$m:meta])* $vis:vis fn $name:ident($($a:ident: $t:ty),* $(,)?) $(-> $r:ty)? = $addr:expr; )*) => {
        $(
            $(#[$m])*
            #[allow(clippy::missing_safety_doc)]
            $vis unsafe fn $name($($a: $t),*) $(-> $r)? {
                let f: extern "C" fn($($t),*) $(-> $r)? = core::mem::transmute($addr as usize);
                f($($a),*)
            }
        )*
    };
}

/// Llama a un metodo virtual `__thiscall` sin argumentos (`this` en ECX).
///
/// # Safety
/// `obj` tiene que ser un objeto C++ valido del juego cuya vtable tenga `slot`.
pub unsafe fn call_virtual0(obj: *mut core::ffi::c_void, slot: usize) {
    #[cfg(target_arch = "x86")]
    type Method = extern "thiscall" fn(*mut core::ffi::c_void);
    // En otras arquitecturas solo compilamos para `cargo check`; nunca se ejecuta.
    #[cfg(not(target_arch = "x86"))]
    type Method = extern "C" fn(*mut core::ffi::c_void);

    let vtable = *(obj as *const *const usize);
    let f: Method = core::mem::transmute(*vtable.add(slot));
    f(obj)
}
