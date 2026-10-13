# gta-rs

Reimplementación incremental de **GTA San Andreas (PC, 1.0 US)** en Rust.

El código Rust se compila como una DLL (`gta_rs.asi`) que el ASI Loader carga dentro
del juego original. Cada función del juego se reescribe en Rust y se redirige a la
nuestra con un hook. El juego funciona en todo momento, así que cada cambio se puede
probar jugando. Cuando no quede código original, el ejecutable será nuestro.

Objetivo a largo plazo: un motor de SA en Rust que también cargue los datos de Vice City
y sirva de base para mods.

## Requisitos

1. **GTA SA 1.0 US.** La versión de Steam (`gta-sa.exe`, 3.0) **no vale**: hay que hacer
   downgrade (por ejemplo, con el *GTA SA Downgrader* de Zdanio95), que deja un
   `gta_sa.exe` 1.0 US. Haz una copia de la carpeta del juego antes.
2. **ASI Loader** (lo instala el downgrader, o el de ThirteenAG como `vorbisFile.dll`).
3. **Rust** con el target de 32 bits: `rustup target add i686-pc-windows-msvc`
   (el script `build.ps1` lo hace por ti).
4. **Visual Studio Build Tools** con "Desarrollo para el escritorio con C++" (el enlazador
   de MSVC; Rust lo necesita en Windows).

## Compilar y probar

```powershell
.\build.ps1            # compila (32 bits) y copia gta_rs.asi + gta_rs.ini a la carpeta del juego
```

Usa siempre `build.ps1` (o `cargo build --release --target i686-pc-windows-msvc`):
un `cargo build` a secas genera una DLL de 64 bits que el juego no puede cargar.

Arranca el juego y abre `gta_rs.log` en la carpeta del juego. Si todo va bien:

```
gta-rs 0.1.0 cargado
Ejecutable: base=0x00400000 timestamp=0x........
Version del juego detectada: 1.0 US (soportada)
  [ ok ] CTimer::Update                   0x00561B10  bytes originales [..]
Hooks: 1 activos, 0 desactivados, 0 fallidos (de 1)
CTimer::Update #0: time=... ms, frame=..., timestep=...
CTimer::Update #1800: ...
```

Si una función reescrita da problemas, ponla a `0` en `gta_rs.ini` (no hace falta
recompilar).

## Estructura

| Ruta | Contenido |
|---|---|
| `src/lib.rs` | `DllMain`: log, detección de versión, instalación de hooks |
| `src/version.rs` | Detecta 1.0 US / 1.01 / EU / Steam |
| `src/patch.rs` | Escritura en memoria, `jmp`, trampolines (con `iced-x86`) |
| `src/hooks/` | Registro de hooks (`Replace` = reescrita, `Wrap` = envuelve la original) |
| `src/game/` | Una clase del juego por módulo (`timer.rs` = `CTimer`, ...) |
| `PROGRESS.md` | Estado de cada función: original / envuelta / reescrita / verificada |
| `NOTES.md` | Lo que vamos aprendiendo del motor, direcciones pendientes de verificar |

## Aviso legal

Proyecto personal de interoperabilidad. No contiene ni debe contener ningún asset ni
código del juego. Requiere una copia legal de GTA San Andreas.
