# Notas del motor

## Versión del ejecutable

- Steam (la que había instalada): `gta-sa.exe`, 5.971.456 bytes, compilado 2014-12-01 20:43:21 UTC
  (timestamp PE `0x547CD2E9`), con ASLR (`DYNAMIC_BASE`) y `.reloc`. SHA-256
  `15e3cfedba9a841df67d8194e7249afb493b0e10d6138fb8ebab2c136e543efb`.
  No coincide con ninguna firma de plugin-sdk. **No soportada.**
- Objetivo: `gta_sa.exe` 1.0 US, base fija `0x400000`. Firma: `*(u32*)0x82457C == 0x94BF`.

## Convenciones de llamada (x86, MSVC 2003 del juego original)

| Original | Rust |
|---|---|
| `__cdecl` (funciones libres, `static`) | `extern "C" fn` |
| `__thiscall` (métodos, `this` en ECX) | `extern "thiscall" fn(this: *mut T, ...)` |
| `__stdcall` (callbacks de Windows/RenderWare) | `extern "system" fn` |

Las estructuras compartidas con el código original van en `#[repr(C)]` con tamaño
comprobado (`const _: () = assert!(size_of::<CPed>() == 0x7A4);`).

## Direcciones pendientes de verificar en juego

Tomadas de gta-reversed; se marcan como verificadas cuando el log lo confirme.

| Símbolo | Dirección | Verificada |
|---|---|---|
| `CTimer::Update` | 0x561B10 | no |
| `CTimer::m_snTimeInMilliseconds` | 0xB7CB84 | no |
| `CTimer::m_FrameCounter` | 0xB7CB4C | no |
| `CTimer::ms_fTimeStep` | 0xB7CB5C | no |

## Referencias

- gta-reversed (C++), mapa de nombres, estructuras y direcciones de SA 1.0 US.
- plugin-sdk, cabeceras y detección de versión.
- re3 / reVC, cómo funciona por dentro el motor de III/VC (para la fase Vice City).
