//! Reimplementacion de las clases del juego, una por modulo.
//!
//! Convencion: cada modulo tiene
//!   - `addr`: direcciones (1.0 US) de funciones y variables globales originales,
//!   - las variables globales como accesores sobre esas direcciones,
//!   - las funciones reescritas,
//!   - `register()`, que anade sus hooks al registro.

pub mod timer;

/// Acceso a una variable global del ejecutable original.
///
/// # Safety
/// `addr` tiene que ser una direccion valida de 1.0 US con el tipo `T`.
#[inline(always)]
pub unsafe fn global<T>(addr: usize) -> &'static mut T {
    &mut *(addr as *mut T)
}
