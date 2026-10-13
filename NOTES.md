# Notas del motor

## Versión del ejecutable

- Steam (la que había instalada): `gta-sa.exe`, 5.971.456 bytes, compilado 2014-12-01 20:43:21 UTC
  (timestamp PE `0x547CD2E9`), con ASLR (`DYNAMIC_BASE`) y `.reloc`. SHA-256
  `15e3cfedba9a841df67d8194e7249afb493b0e10d6138fb8ebab2c136e543efb`.
  No coincide con ninguna firma de plugin-sdk. **No soportada.**
- Objetivo: `gta_sa.exe` 1.0 US, base fija `0x400000`. Firma: `*(u32*)0x82457C == 0x94BF`.
- **Instalado tras el downgrade (2026-10-13):** `gta_sa.exe` (y copia `gta-sa.exe`, que es la que lanza Steam),
  14.383.616 bytes, timestamp `0x427101CA` (2005-04-28), sin ASLR, seccion `.HOODLUM` (exe "compact" sin cifrar).
  MD5 `170b3a9108687b26da2d8901c6948a18`. Firma 1.0 US comprobada en el fichero: OK.
- Bytes en `CTimer::Update` (0x561B10): `8B0D28CBB700 83EC0C 85C9 0F84...`
  (`mov ecx,[0xB7CB28]` / `sub esp,0Ch` / `test ecx,ecx` / `jz`): empieza en limite de instruccion,
  la primera instruccion (6 bytes) cubre el `jmp` de 5. Encaja con el inicio de una funcion.

## Convenciones de llamada (x86, MSVC 2003 del juego original)

| Original | Rust |
|---|---|
| `__cdecl` (funciones libres, `static`) | `extern "C" fn` |
| `__thiscall` (métodos, `this` en ECX) | `extern "thiscall" fn(this: *mut T, ...)` |
| `__stdcall` (callbacks de Windows/RenderWare) | `extern "system" fn` |

Las estructuras compartidas con el código original van en `#[repr(C)]` con tamaño
comprobado (`const _: () = assert!(size_of::<CPed>() == 0x7A4);`).

## Direcciones verificadas

Confirmadas en el juego con `gta_rs.log` (2026-10-13): el hook envuelto se ejecuto en cada
frame, `m_FrameCounter` == llamadas + 1, el tiempo se congela con el juego en pausa.

| Simbolo | Direccion | Como se verifico |
|---|---|---|
| `CTimer::Update` | 0x561B10 | hook ejecutado cada frame |
| `CTimer::m_snTimeInMilliseconds` | 0xB7CB84 | avanza en juego, se para en pausa |
| `CTimer::m_FrameCounter` | 0xB7CB4C | coincide con el numero de llamadas |
| `CTimer::ms_fTimeStep` | 0xB7CB5C | ~0.15 a ~275 fps (1.0 = 1/50 s), 0 en pausa |

## Direcciones pendientes de verificar en juego

Sacadas del desensamblado de `CTimer::Update` (0x561B10-0x561C6E), ver `src/game/timer.rs`.

| Simbolo | Direccion | Notas |
|---|---|---|
| `CTimer::UpdateVariables(float)` | 0x5618D0 | cdecl, arg en [esp+4] |
| `ms_fnTimerFunction` | 0xB7CB28 | devuelve u64 en edx:eax |
| `m_snTimerDivider` | 0xB7CB2C | ticks por ms |
| `m_snPreviousTimer` (oldPcTimer) | 0xB7CB38 | u64 |
| `unk_B7CB40` | 0xB7CB40 | u8: Update pone 1, 0x5618C0 pone 0 |
| `m_CodePause` / `m_UserPause` | 0xB7CB48 / 0xB7CB49 | bool |
| `game_FPS` | 0xB7CB50 | f32 = 1000 / (NonClipped - PrevNonClipped) |
| `ms_fTimeScale` | 0xB7CB64 | f32 |
| `m_snPreviousTimeInMillisecondsNonClipped` | 0xB7CB68 | |
| `m_snPPPP/PPP/PP/PreviousTimeInMilliseconds` | 0xB7CB6C / 70 / 74 / 78 | historial |
| `m_snTimeInMillisecondsPauseMode` | 0xB7CB7C | |
| `m_snTimeInMillisecondsNonClipped` | 0xB7CB80 | |
| constantes 1000.0 / 2^32 | 0x858C4C / 0x858C54 | leidas del exe |

## Observaciones

- El juego iba a ~275 fps (limitador de frames desactivado). SA tiene fallos de fisica por
  encima de 30 fps: conviene activar el limitador en Opciones > Pantalla.
- El ASI Loader tiene que ser la version Win32 (`dinput8.dll`); la x64 se ignora en silencio.
- Raton y resoluciones altas en Windows moderno: SilentPatch.

## Referencias

- gta-reversed (C++), mapa de nombres, estructuras y direcciones de SA 1.0 US.
- plugin-sdk, cabeceras y detección de versión.
- re3 / reVC, cómo funciona por dentro el motor de III/VC (para la fase Vice City).
