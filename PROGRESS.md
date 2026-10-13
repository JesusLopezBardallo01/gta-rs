# Progreso

Estados: `original` → `envuelta` (Wrap, llama a la original) → `reescrita` (Replace) → `verificada` (probada en juego)

## Infraestructura

| Pieza | Estado |
|---|---|
| DLL `.asi` + log | hecho, pendiente de probar en juego |
| Detección de versión | hecho, pendiente de probar en juego |
| Hooks `Replace` (jmp) | hecho, pendiente de probar en juego |
| Hooks `Wrap` (trampolín iced-x86) | hecho, pendiente de probar en juego |
| `gta_rs.ini` para activar/desactivar | hecho |

## Funciones del juego

| Clase | Función | Dirección 1.0 US | Estado | Notas |
|---|---|---|---|---|
| CTimer | Update | 0x561B10 | envuelta | prueba del sistema de hooks |

## Siguiente

- [ ] Probar la primera DLL en el juego (requiere downgrade a 1.0 US)
- [ ] Reescribir CTimer completo (Update, Initialise, Suspend/Resume, GetCurrentTimeInCycles...)
- [ ] Elegir el primer sistema grande a atacar
