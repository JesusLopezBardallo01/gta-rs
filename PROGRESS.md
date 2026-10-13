# Progreso

Estados: `original` → `envuelta` (Wrap, llama a la original) → `reescrita` (Replace) → `verificada` (probada en juego)

## Infraestructura

| Pieza | Estado |
|---|---|
| DLL `.asi` + log | verificado en juego |
| Detección de versión | verificado en juego (1.0 US) |
| Hooks `Replace` (jmp) | verificado en juego |
| Hooks `Wrap` (trampolín iced-x86) | verificado en juego |
| `gta_rs.ini` para activar/desactivar | hecho |

## Funciones del juego

| Clase | Función | Dirección 1.0 US | Estado | Notas |
|---|---|---|---|---|
| CTimer | Update | 0x561B10 | verificada | traducida del ensamblador; probada jugando, en pausa y con limitador |
| CTimer | UpdateVariables | 0x5618D0 | verificada | ms y timestep (limites 0.01 / 3.0 / 0.00001, recorte 300 ms) |
| CTimer | Initialise | 0x5617E0 | verificada | elige QPC o RsTimer, divisor = freq/1000 |
| CTimer | Shutdown / Stop | 0x5618C0 / 0x561AA0 | verificada | |
| CTimer | Suspend / Resume | 0x5619D0 / 0x561A00 | verificada | anidables, compensan el tiempo suspendido |
| CTimer | GetTimerQPC / GetTimerRs | 0x5617A0 / 0x5617C0 | verificada | funciones de reloj |
| CTimer | GetCyclesPerMillisecond / GetCyclesPerFrame / GetCurrentTimeInCycles | 0x561A40 / 0x561A50 / 0x561A80 | verificada | |
| CTimer | GetIsSlowMotionActive / StartUserPause / EndUserPause | 0x561AD0 / 0x561AF0 / 0x561B00 | verificada | |
| CTimer | (vacia) | 0x5617D0 | sin hook | `ret` de 1 byte, no cabe un jmp y no hace nada |
| CFileLoader | LoadLevel | 0x5B9030 | verificada | parser de DEFAULT.DAT / GTA.DAT (54 IDE, 52 IPL cargados) |
| CFileLoader | LoadObjectTypes | 0x5B8400 | verificada | lector de IDE por secciones (57 IDE) |
| CFileLoader | LoadScene | 0x5B8700 | verificada | lector de IPL de texto + LinkLods (~9.300 instancias) |

## Siguiente

- [x] Probar la primera DLL en el juego
- [x] Probar CTimer::Update reescrita (primer hook `Replace`)
- [x] Probar CFileLoader::LoadLevel reescrita
- [x] Probar LoadObjectTypes (IDE) y LoadScene (IPL)
- [ ] Carga de datos: LoadObject, LoadObjectInstance, LoadLine, CFileMgr
- [x] Reescribir CTimer completo
- [x] Probar CTimer completo (18 hooks ok, reloj QPC a 10000 ciclos/ms)
- [x] Elegir el primer sistema grande: carga de datos y mapa (camino hacia Vice City)
