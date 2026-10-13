# gta-rs: instrucciones para Claude

Reimplementacion incremental de GTA SA 1.0 US en Rust (DLL `.asi` con hooks). Claude escribe el codigo;
el usuario (Windows x64, juego en `E:\SteamLibrary\steamapps\common\Grand Theft Auto San Andreas`)
compila con `build.ps1`, prueba en el juego y devuelve `gta_rs.log`.

- Idioma del proyecto: espanol (comentarios, docs, commits).
- Antes de trabajar, lee `PROGRESS.md` y `NOTES.md`. Actualizalos en el mismo commit que el codigo.
- Solo 1.0 US. Direcciones y estructuras: referencia gta-reversed / plugin-sdk. Toda direccion nueva va a
  la tabla "pendientes de verificar" de NOTES.md hasta que el log del juego la confirme.
- Una clase del juego por modulo en `src/game/`; cada hook se registra con `register()` y se puede
  desactivar en `gta_rs.ini`.
- No se puede compilar para Windows en el entorno de Claude: como minimo `cargo check --target
  x86_64-unknown-linux-gnu` y `cargo clippy` antes de cada commit.
- Nunca incluir assets ni codigo del juego en el repositorio.
