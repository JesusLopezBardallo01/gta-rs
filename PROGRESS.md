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

## Siguiente

- [x] Probar la primera DLL en el juego
- [x] Probar CTimer::Update reescrita (primer hook `Replace`)
- [ ] Reescribir CTimer completo (Update, Initialise, Suspend/Resume, GetCurrentTimeInCycles...)
- [ ] Elegir el primer sistema grande a atacar
