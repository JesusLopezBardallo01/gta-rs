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
| CTimer | UpdateVariables | 0x5618D0 | original | la llama Update |
| CFileLoader | LoadLevel | 0x5B9030 | verificada | parser de DEFAULT.DAT / GTA.DAT (54 IDE, 52 IPL cargados) |
| CFileLoader | LoadObjectTypes | 0x5B8400 | reescrita | lector de IDE por secciones, pendiente de probar |
| CFileLoader | LoadScene | 0x5B8700 | reescrita | lector de IPL de texto + LinkLods, pendiente de probar |

## Siguiente

- [x] Probar la primera DLL en el juego
- [x] Probar CTimer::Update reescrita (primer hook `Replace`)
- [x] Probar CFileLoader::LoadLevel reescrita
- [ ] Probar LoadObjectTypes (IDE) y LoadScene (IPL)
- [ ] Carga de datos: LoadObject, LoadObjectInstance, LoadLine, CFileMgr
- [ ] Reescribir CTimer completo (Update, Initialise, Suspend/Resume, GetCurrentTimeInCycles...)
- [x] Elegir el primer sistema grande: carga de datos y mapa (camino hacia Vice City)
