//! CFileLoader: carga de los ficheros de nivel (`DATA\DEFAULT.DAT`, `DATA\GTA.DAT`).
//!
//! `CFileLoader::LoadLevel` (0x5B9030) esta reescrita en Rust, traducida del desensamblado
//! de 1.0 US. Lee el `.dat` linea a linea y despacha cada entrada:
//!
//! | Entrada      | Accion                                                    |
//! |--------------|-----------------------------------------------------------|
//! | `#...`       | comentario                                                |
//! | `EXIT`       | deja de leer                                              |
//! | `TEXDICTION` | carga un .txd y pasa sus texturas al diccionario actual   |
//! | `IMG`        | anade un .img al streaming (salvo `MODELS\GTA_INT.IMG`)   |
//! | `COLFILE`    | carga un fichero de colisiones (.col)                     |
//! | `MODELFILE`  | carga un .dff como atomic                                 |
//! | `HIERFILE`   | carga un .dff como clump                                  |
//! | `IDE`        | definiciones de objetos (`LoadObjectTypes`)               |
//! | `IPL`        | colocacion en el mapa (`LoadScene`). Antes del primer IPL |
//! |              | se inicializan object.dat, vehiculos, streaming y colisiones |
//! | `SPLASH`     | se ignora                                                 |
//!
//! Las funciones a las que llama siguen siendo las originales. Los nombres marcados con
//! "probable" salen de gta-reversed y de los textos de la pantalla de carga; ver NOTES.md.
//!
//! Diferencias con el original (a proposito):
//!   - Las rutas no se copian a un buffer de 64 bytes en la pila (el original desborda si
//!     la ruta es larga).
//!   - Las lineas vacias no se registran en el log (el original las ignora igualmente).
//!   - Si el `.dat` no se puede abrir, se registra en el log y se sale (el original crashea).

use super::{call_virtual0, global};
use crate::hooks::{Hook, Kind};
use crate::log;
use core::ffi::{c_char, c_void, CStr};
use std::ffi::CString;

pub mod addr {
    pub const LOAD_LEVEL: usize = 0x005B_9030; // void CFileLoader::LoadLevel(const char*)

    pub const G_STRING: usize = 0x00B7_1670; // char gString[256]
    pub const MODEL_INFO_PTRS: usize = 0x00A9_B0C8; // CBaseModelInfo* CModelInfo::ms_modelInfoPtrs[20000]
    pub const MODEL_INFO_PTRS_END: usize = 0x00AA_E948;
}

/// Slot de `CBaseModelInfo::ConvertAnimFileIndex` en la vtable (offset 0x34).
const VT_CONVERT_ANIM_FILE_INDEX: usize = 0x34 / 4;

crate::original_cdecl! {
    // RenderWare
    fn rw_tex_dictionary_get_current() -> *mut c_void = 0x7F3A90;
    fn rw_tex_dictionary_create() -> *mut c_void = 0x7F3600;
    fn rw_tex_dictionary_set_current(txd: *mut c_void) -> *mut c_void = 0x7F3A70;
    fn rw_tex_dictionary_for_all_textures(txd: *mut c_void, cb: usize, data: *mut c_void) -> *mut c_void = 0x7F3730;
    fn rw_tex_dictionary_destroy(txd: *mut c_void) -> i32 = 0x7F36A0;

    // Ficheros
    fn file_mgr_open_file(path: *const c_char, mode: *const c_char) -> *mut c_void = 0x538900;
    fn file_mgr_close_file(f: *mut c_void) -> i32 = 0x5389D0;
    fn load_line(f: *mut c_void) -> *mut c_char = 0x536F80;

    // Pantalla de carga
    fn loading_screen(title: *const c_char, msg: *const c_char, msg2: *const c_char) = 0x53DED0;
    fn loading_screen_loading_file(path: *const c_char) = 0x5B3680;

    // Cargadores
    fn load_tex_dictionary(path: *const c_char) -> *mut c_void = 0x5B3860;
    fn add_image_to_list(path: *const c_char, not_player_img: bool) -> i32 = 0x407610;
    fn load_collision_file(path: *const c_char, col_id: u8) = 0x5B4E60;
    fn load_atomic_file(path: *const c_char) = 0x5B39D0;
    fn load_clump_file(path: *const c_char) = 0x5B3A30;
    fn load_object_types(path: *const c_char) = 0x5B8400;
    fn load_scene(path: *const c_char) = 0x5B8700;

    // Inicializacion antes del primer IPL (nombres probables)
    fn match_all_model_strings() = 0x5B57C0;
    fn object_data_initialise(path: *const c_char, b: bool) = 0x5B5360;
    fn vehicle_setup_common_data() = 0x5B8F00;
    fn streaming_init2() = 0x5B8AD0;
    fn loading_screen_new_chunk_loaded() = 0x590D00;
    fn col_store_load_all_bounding_boxes() = 0x4113D0;

    // Al terminar, si se cargo algun IPL (nombres probables)
    fn ipl_store_load_all_remaining_ipls() = 0x405780;
    fn col_store_bounding_boxes_post_process() = 0x410EC0;
    fn train_init_trains() = 0x6F7440;
    fn col_store_remove_all_collision() = 0x410E00;
}

/// Callback original `AddTextureCB` (0x5B38F0): anade una textura al diccionario `data`.
const ADD_TEXTURE_CB: usize = 0x5B38F0;

/// Si la linea empieza por `kw`, devuelve el resto saltando `skip` bytes desde el inicio
/// (el original usa `strncmp(kw, linea, len(kw))` y luego `linea + skip`).
fn arg<'a>(line: &'a [u8], kw: &[u8], skip: usize) -> Option<&'a [u8]> {
    if line.starts_with(kw) {
        Some(line.get(skip..).unwrap_or(&[]))
    } else {
        None
    }
}

fn cstr(b: &[u8]) -> CString {
    CString::new(b).unwrap_or_default()
}

/// Escribe en el `gString` global del juego (char[256]).
unsafe fn set_g_string(s: &str) {
    let dst = global::<[u8; 256]>(addr::G_STRING);
    let n = s.len().min(255);
    dst[..n].copy_from_slice(&s.as_bytes()[..n]);
    dst[n] = 0;
}

/// Reimplementacion de `CFileLoader::LoadLevel` (0x5B9030).
pub extern "C" fn load_level(filename: *const c_char) {
    unsafe {
        let name = CStr::from_ptr(filename).to_string_lossy().into_owned();
        log!("LoadLevel(Rust): {name}");

        let mut saved_txd = rw_tex_dictionary_get_current();
        if saved_txd.is_null() {
            saved_txd = rw_tex_dictionary_create();
            rw_tex_dictionary_set_current(saved_txd);
        }

        let file = file_mgr_open_file(filename, c"r".as_ptr());
        if file.is_null() {
            log!("  ERROR: no se puede abrir {name}");
            rw_tex_dictionary_set_current(saved_txd);
            return;
        }

        let mut ipls_loaded = false;
        let mut counts = [0u32; 8];

        loop {
            let p = load_line(file);
            if p.is_null() {
                break;
            }
            // `LoadLine` devuelve un buffer estatico que se reutiliza: hay que copiar la
            // linea ya, porque las funciones que llamamos (p. ej. CObjectData::Initialise
            // antes del primer IPL) leen otros ficheros y lo sobrescriben.
            let owned = CStr::from_ptr(p).to_bytes().to_vec();
            let line = owned.as_slice();
            if line.first() == Some(&b'#') || line.is_empty() {
                continue;
            }
            log!("  {}", String::from_utf8_lossy(line));
            if line.starts_with(b"EXIT") {
                break;
            }

            if let Some(path) = arg(line, b"TEXDICTION", 11) {
                // Nombre del fichero sin directorios, para la pantalla de carga
                let file_name = match path.iter().rposition(|&c| c == b'\\') {
                    Some(i) => &path[i + 1..],
                    None => path,
                };
                set_g_string(&format!("Loading {}", String::from_utf8_lossy(file_name)));
                loading_screen(
                    c"Loading the Game".as_ptr(),
                    addr::G_STRING as *const c_char,
                    core::ptr::null(),
                );

                let path_c = cstr(path);
                let txd = load_tex_dictionary(path_c.as_ptr());
                rw_tex_dictionary_for_all_textures(txd, ADD_TEXTURE_CB, saved_txd);
                rw_tex_dictionary_destroy(txd);
                counts[0] += 1;
            } else if let Some(path) = arg(line, b"IMG", 4) {
                if path != b"MODELS\\GTA_INT.IMG" {
                    let path_c = cstr(path);
                    add_image_to_list(path_c.as_ptr(), true);
                }
                counts[1] += 1;
            } else if let Some(path) = arg(line, b"COLFILE", 10) {
                let path_c = cstr(path);
                loading_screen_loading_file(path_c.as_ptr());
                load_collision_file(path_c.as_ptr(), 0);
                counts[2] += 1;
            } else if let Some(path) = arg(line, b"MODELFILE", 10) {
                let path_c = cstr(path);
                loading_screen_loading_file(path_c.as_ptr());
                load_atomic_file(path_c.as_ptr());
                counts[3] += 1;
            } else if let Some(path) = arg(line, b"HIERFILE", 9) {
                let path_c = cstr(path);
                loading_screen_loading_file(path_c.as_ptr());
                load_clump_file(path_c.as_ptr());
                counts[4] += 1;
            } else if let Some(path) = arg(line, b"IDE", 4) {
                let path_c = cstr(path);
                loading_screen_loading_file(path_c.as_ptr());
                load_object_types(path_c.as_ptr());
                counts[5] += 1;
            } else if let Some(path) = arg(line, b"IPL", 4) {
                if !ipls_loaded {
                    before_first_ipl();
                    ipls_loaded = true;
                }
                let path_c = cstr(path);
                loading_screen_loading_file(path_c.as_ptr());
                load_scene(path_c.as_ptr());
                counts[6] += 1;
            } else {
                // SPLASH y lineas desconocidas: no hacen nada
                counts[7] += 1;
            }
        }

        file_mgr_close_file(file);
        rw_tex_dictionary_set_current(saved_txd);

        if ipls_loaded {
            ipl_store_load_all_remaining_ipls();
            col_store_bounding_boxes_post_process();
            train_init_trains();
            col_store_remove_all_collision();
        }

        log!(
            "  {name}: TEXDICTION={} IMG={} COLFILE={} MODELFILE={} HIERFILE={} IDE={} IPL={} otras={}",
            counts[0], counts[1], counts[2], counts[3], counts[4], counts[5], counts[6], counts[7]
        );
    }
}

/// Lo que hace el original justo antes de cargar el primer IPL.
unsafe fn before_first_ipl() {
    log!("  -- inicializacion previa al primer IPL --");
    match_all_model_strings();

    loading_screen_loading_file(c"Object Data".as_ptr());
    object_data_initialise(c"DATA\\OBJECT.DAT".as_ptr(), false);

    loading_screen_loading_file(c"Setup vehicle info data".as_ptr());
    vehicle_setup_common_data();

    loading_screen_loading_file(c"Streaming Init".as_ptr());
    streaming_init2();
    loading_screen_new_chunk_loaded();

    loading_screen_loading_file(c"Collision".as_ptr());
    col_store_load_all_bounding_boxes();
    log!("  -- ConvertAnimFileIndex --");

    // CBaseModelInfo::ConvertAnimFileIndex() para todos los modelos registrados
    let mut p = addr::MODEL_INFO_PTRS;
    while p < addr::MODEL_INFO_PTRS_END {
        let mi = *(p as *const *mut c_void);
        if !mi.is_null() {
            call_virtual0(mi, VT_CONVERT_ANIM_FILE_INDEX);
        }
        p += 4;
    }
    log!("  -- fin de la inicializacion --");
}

pub fn register(v: &mut Vec<Hook>) {
    v.push(Hook {
        name: "CFileLoader::LoadLevel",
        addr: addr::LOAD_LEVEL,
        kind: Kind::Replace {
            func: load_level as *const () as usize,
        },
        enabled_by_default: true,
    });
}
