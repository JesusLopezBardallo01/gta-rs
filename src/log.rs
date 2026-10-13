//! Log sencillo a fichero. Se abre una vez al cargar la DLL y cada linea se vuelca
//! inmediatamente (si el juego crashea, el log queda completo hasta el ultimo mensaje).

use std::fs::File;
use std::io::Write;
use std::sync::Mutex;

static LOG: Mutex<Option<File>> = Mutex::new(None);

pub fn init(path: &str) {
    if let Ok(f) = File::create(path) {
        *LOG.lock().unwrap_or_else(|e| e.into_inner()) = Some(f);
    }
}

pub fn write(msg: std::fmt::Arguments) {
    let mut guard = LOG.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(f) = guard.as_mut() {
        let _ = writeln!(f, "{msg}");
        let _ = f.flush();
    }
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => { $crate::log::write(format_args!($($arg)*)) };
}
