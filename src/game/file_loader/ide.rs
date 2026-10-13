//! `CFileLoader::LoadObjectTypes` (0x5B8400): lectura de ficheros IDE (definiciones).
//!
//! Un IDE tiene secciones `nombre` ... `end`. Cada linea de datos de una seccion se pasa
//! a la funcion original que la interpreta y registra el modelo:
//!
//! | Seccion | Funcion original                        |
//! |---------|-----------------------------------------|
//! | `objs`  | `LoadObject` (0x5B3C60)                 |
//! | `tobj`  | `LoadTimeObject` (0x5B3DE0)             |
//! | `weap`  | `LoadWeaponObject` (0x5B3FB0)           |
//! | `hier`  | `LoadClumpObject` (0x5B4040)            |
//! | `anim`  | `LoadAnimatedClumpObject` (0x5B40C0)    |
//! | `cars`  | `LoadVehicleObject` (0x5B6F30)          |
//! | `peds`  | `LoadPedObject` (0x5B7420)              |
//! | `path`  | cabecera + 12 nodos (formato antiguo)   |
//! | `2dfx`  | `Load2dEffect` (0x5B7670)               |
//! | `txdp`  | `LoadTXDParent` (0x5B75E0)              |
//!
//! El original calcula el id minimo y maximo de los modelos cargados y no los usa
//! (restos de depuracion). Aqui se usan para el log.

use super::{cstr, file_mgr_close_file, file_mgr_open_file, next_line};
use crate::hooks::{Hook, Kind};
use crate::log;
use core::ffi::{c_char, CStr};

pub const LOAD_OBJECT_TYPES: usize = 0x005B_8400;

crate::original_cdecl! {
    fn load_object(line: *const c_char) -> i32 = 0x5B3C60;
    fn load_time_object(line: *const c_char) -> i32 = 0x5B3DE0;
    fn load_weapon_object(line: *const c_char) -> i32 = 0x5B3FB0;
    fn load_clump_object(line: *const c_char) -> i32 = 0x5B4040;
    fn load_animated_clump_object(line: *const c_char) -> i32 = 0x5B40C0;
    fn load_vehicle_object(line: *const c_char) -> i32 = 0x5B6F30;
    fn load_ped_object(line: *const c_char) -> i32 = 0x5B7420;
    fn load_2d_effect(line: *const c_char) = 0x5B7670;
    fn load_txd_parent(line: *const c_char) = 0x5B75E0;

    // Nodos de caminos en formato antiguo
    fn load_path_header(line: *const c_char, path_type: *mut i32) -> i32 = 0x5B41C0;
    fn load_ped_path_node(line: *const c_char, id: i32, node: i32) = 0x5B41F0;
    fn load_car_path_node(line: *const c_char, id: i32, node: i32, water: bool) = 0x5B4380;
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    None,
    Objs,
    Tobj,
    Weap,
    Hier,
    Anim,
    Cars,
    Peds,
    Path,
    TwoDfx,
    Txdp,
}

fn section_header(line: &[u8]) -> Option<Section> {
    let s = match line.get(..4)? {
        b"objs" => Section::Objs,
        b"tobj" => Section::Tobj,
        b"weap" => Section::Weap,
        b"hier" => Section::Hier,
        b"anim" => Section::Anim,
        b"cars" => Section::Cars,
        b"peds" => Section::Peds,
        b"path" => Section::Path,
        b"2dfx" => Section::TwoDfx,
        b"txdp" => Section::Txdp,
        _ => return None,
    };
    Some(s)
}

/// Estado de una seccion `path` (formato antiguo): una cabecera y 12 nodos.
struct PathState {
    node: i32, // -1 = se espera cabecera
    id: i32,
    path_type: i32,
}

impl PathState {
    fn new() -> Self {
        PathState {
            node: -1,
            id: 0,
            path_type: 0,
        }
    }

    unsafe fn line(&mut self, line: *const c_char) {
        if self.node == -1 {
            self.id = load_path_header(line, &mut self.path_type);
            self.node = 0;
            return;
        }
        let id = self.id;
        match self.path_type {
            0 => load_ped_path_node(line, id, self.node),
            1 => load_car_path_node(line, id, self.node, false),
            2 => load_car_path_node(line, id, self.node, true),
            _ => {}
        }
        self.node += 1;
        if self.node == 12 {
            self.node = -1;
        }
    }
}

/// Reimplementacion de `CFileLoader::LoadObjectTypes` (0x5B8400).
pub extern "C" fn load_object_types(filename: *const c_char) {
    unsafe {
        let file = file_mgr_open_file(filename, c"rb".as_ptr());
        if file.is_null() {
            log!(
                "LoadObjectTypes: no se puede abrir {}",
                CStr::from_ptr(filename).to_string_lossy()
            );
            return;
        }

        let mut section = Section::None;
        let mut path = PathState::new();
        let (mut min_id, mut max_id) = (i32::MAX, -1i32);
        let mut lines = 0u32;

        while let Some(line) = next_line(file) {
            if line.is_empty() || line[0] == b'#' {
                continue;
            }
            if section == Section::None {
                if let Some(s) = section_header(&line) {
                    section = s;
                }
                continue;
            }
            if line.starts_with(b"end") {
                section = Section::None;
                continue;
            }

            let c = cstr(&line);
            let p = c.as_ptr();
            lines += 1;
            let id = match section {
                Section::Objs => load_object(p),
                Section::Tobj => load_time_object(p),
                Section::Weap => load_weapon_object(p),
                Section::Hier => load_clump_object(p),
                Section::Anim => load_animated_clump_object(p),
                Section::Cars => load_vehicle_object(p),
                Section::Peds => load_ped_object(p),
                Section::Path => {
                    path.line(p);
                    -1
                }
                Section::TwoDfx => {
                    load_2d_effect(p);
                    -1
                }
                Section::Txdp => {
                    load_txd_parent(p);
                    -1
                }
                Section::None => -1,
            };
            if id != -1 {
                max_id = max_id.max(id);
                min_id = min_id.min(id);
            }
        }
        file_mgr_close_file(file);

        let name = CStr::from_ptr(filename).to_string_lossy();
        if max_id >= 0 {
            log!("IDE {name}: {lines} lineas, modelos {min_id}..{max_id}");
        } else {
            log!("IDE {name}: {lines} lineas");
        }
    }
}

pub fn register(v: &mut Vec<Hook>) {
    v.push(Hook {
        name: "CFileLoader::LoadObjectTypes",
        addr: LOAD_OBJECT_TYPES,
        kind: Kind::Replace {
            func: load_object_types as *const () as usize,
        },
        enabled_by_default: true,
    });
}
