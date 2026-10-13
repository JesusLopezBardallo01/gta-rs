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

## Mapa de funciones de CTimer (desensamblado, sin verificar)

Bloque 0x5617A0-0x561B07. Para la tarea "terminar CTimer".

| Direccion | Que hace (segun el ensamblador) | Nombre probable |
|---|---|---|
| 0x5617A0 | QueryPerformanceCounter -> u64 | `GetCurrentTimeInCyclesQPC` (timer function) |
| 0x5617C0 | call 0x619410 (timeGetTime?), edx=0 | timer function sin QPC |
| 0x5617E0 | pone a 0 contadores/historial, escalas a 1.0, elige funcion de reloj (QPC o la otra), divisor = freq/1000 | `Initialise` |
| 0x5618C0 | `[B7CB40] = 0` | `Shutdown` / fin de frame |
| 0x5618D0 | delta->ms, timestep (min 0.00001, max 3.0?), recorte a 300 ms | `UpdateVariables(float)` |
| 0x5619D0 | si B7CB40: ++B7CB44; si ==1 guarda reloj en B7CB30 | `Suspend` |
| 0x561A00 | si B7CB40 y --B7CB44==0: oldPcTimer += reloj - B7CB30 | `Resume` |
| 0x561A40 | devuelve divisor (B7CB2C) | `GetCyclesPerMillisecond` |
| 0x561A50 | ftol(divisor * const 0x858BA4) | `GetCyclesPerFrame` |
| 0x561A80 | QPC - oldPcTimer (u32) | `GetCurrentTimeInCycles` |
| 0x561AA0 | historial = tiempo actual, B7CB40 = 0 | `StartUserPause`? / `ResetHistory` |
| 0x561AD0 | `ms_fTimeScale < const 0x858624` | `GetIsSlowMotionActive` |
| 0x561AF0 / 0x561B00 | `m_UserPause = 1 / 0` | `StartUserPause` / `EndUserPause` |

## CFileLoader::LoadLevel (0x5B9030), sin verificar

Funciones a las que llama (nombres de gta-reversed; los "probable" deducidos por contexto
y por los textos de la pantalla de carga):

| Direccion | Funcion |
|---|---|
| 0x538900 / 0x5389D0 | `CFileMgr::OpenFile(path, "r")` / `CloseFile` |
| 0x536F80 | `CFileLoader::LoadLine(FILE*)` -> buffer estatico ya limpio |
| 0x53DED0 | `LoadingScreen(titulo, msg, msg2)` |
| 0x5B3680 | `LoadingScreenLoadingFile(path)` |
| 0x5B3860 / 0x5B38F0 | `LoadTexDictionary(path)` / `AddTextureCB` |
| 0x407610 | `CStreaming::AddImageToList(path, notPlayerImg)` |
| 0x5B4E60 | `LoadCollisionFile(path, colId)` |
| 0x5B39D0 / 0x5B3A30 | `LoadAtomicFile` / `LoadClumpFile` |
| 0x5B8400 / 0x5B8700 | `LoadObjectTypes` (IDE) / `LoadScene` (IPL) |
| 0x5B57C0 | `MatchAllModelStrings` (probable) |
| 0x5B5360 | `CObjectData::Initialise("DATA\\OBJECT.DAT", false)` |
| 0x5B8F00 | `CVehicleModelInfo::SetupCommonData` (probable, "Setup vehicle info data") |
| 0x5B8AD0 | `CStreaming::Init2` (probable, "Streaming Init") |
| 0x590D00 | pantalla de carga, avance (probable) |
| 0x4113D0 | `CColStore::LoadAllBoundingBoxes` (probable, "Collision") |
| 0x405780 / 0x410EC0 / 0x6F7440 / 0x410E00 | al final si hubo IPL: `CIplStore::LoadAllRemainingIpls`, `CColStore::BoundingBoxesPostProcess`, `CTrain::InitTrains`, `CColStore::RemoveAllCollision` (probables) |
| 0x7F3A90 / 0x7F3600 / 0x7F3A70 / 0x7F3730 / 0x7F36A0 | RenderWare: `RwTexDictionaryGetCurrent` / `Create` / `SetCurrent` / `ForAllTextures` / `Destroy` |
| 0xB71670 | `gString[256]` |
| 0xA9B0C8..0xAAE948 | `CModelInfo::ms_modelInfoPtrs`; vtable +0x34 = `ConvertAnimFileIndex` |

**Ojo:** `LoadLine` devuelve un buffer estatico compartido. Cualquier funcion que lea otro
fichero (p. ej. `CObjectData::Initialise` antes del primer IPL) lo sobrescribe: hay que
copiar la linea nada mas leerla. El primer crash del proyecto (2026-10-13) fue por esto.
Las lineas del .dat llegan con un espacio final; Windows lo ignora al abrir ficheros.

`IMG MODELS\GTA_INT.IMG` se ignora a proposito en el original (comparacion exacta).

## LoadObjectTypes (0x5B8400) y LoadScene (0x5B8700), sin verificar

- Ambas abren con `"rb"` (0x859C6C). Cabeceras de 4 letras solo fuera de seccion; `end` (3) cierra.
- IDE: `objs`=1, `tobj`=3, `weap`=4, `hier`=5, `anim`=6, `cars`=7, `peds`=8, `path`=9, `2dfx`=10,
  `txdp`=11 (2 sin usar). Calcula id min/max de modelos sin usarlos. `path`: `LoadPathHeader` 0x5B41C0
  y 12 nodos con `LoadPedPathNode` 0x5B41F0 / `LoadCarPathNode` 0x5B4380 (agua = 4o arg).
- IPL: `path`=1 (la primera linea de datos termina la lectura), `inst`=2, `mult`=3, `zone`=4,
  `cull`=5, `occl`=6, (7 sin cabecera, codigo muerto), `grge`=8, `enex`=9, `pick`=10, `cars`=11,
  `jump`=12, `tcyc`=13, `auzo`=14.
- Instancias de `inst` en `0xBCC0E0` (CEntity*[]), contador `0xBCC0D8`. Sin comprobacion de limite.
- Final de LoadScene: `CIplStore::GetNewIplEntityIndexArray` 0x404780, `GetIplEntityIndexArray`
  0x4047B0, `SetupRelatedIpls` 0x404DE0, `LinkLods` 0x5B51E0, `RemoveRelatedIpls` 0x405110.
- El tramo final de LoadObjectTypes salta a 0x403990 (`add esp,0x110; jmp ret`): el exe HOODLUM
  tiene trozos de codigo movidos a 0x401000-0x40xxxx (restos del SecuROM). Cuidado al desensamblar.

## Observaciones

- `CTimer::Update` en Rust probada (2026-10-13): tiempo, frames y pausa correctos. Con el
  limitador activado: fps=30, timestep=1.667 (33 ms / 20 ms). Sin limitador, `game_FPS` sale
  1000 o `inf` porque varios frames caen en el mismo milisegundo (el original hace lo mismo).

- El juego iba a ~275 fps (limitador de frames desactivado). SA tiene fallos de fisica por
  encima de 30 fps: conviene activar el limitador en Opciones > Pantalla.
- El ASI Loader tiene que ser la version Win32 (`dinput8.dll`); la x64 se ignora en silencio.
- Raton y resoluciones altas en Windows moderno: SilentPatch.

## Referencias

- gta-reversed (C++), mapa de nombres, estructuras y direcciones de SA 1.0 US.
- plugin-sdk, cabeceras y detección de versión.
- re3 / reVC, cómo funciona por dentro el motor de III/VC (para la fase Vice City).
