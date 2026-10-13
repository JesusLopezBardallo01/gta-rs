//! `CFileLoader::LoadScene` (0x5B8700): lectura de ficheros IPL (colocacion en el mapa).
//!
//! | Seccion | Funcion original                           |
//! |---------|--------------------------------------------|
//! | `inst`  | `LoadObjectInstance` (0x538690) -> entidad |
//! | `mult`  | se ignora                                  |
//! | `zone`  | `LoadZone` (0x5B4AB0)                      |
//! | `cull`  | `LoadCullZone` (0x5B4B40)                  |
//! | `occl`  | `LoadOcclusionVolume(linea, fichero)` (0x5B4C80) |
//! | `grge`  | `LoadGarage` (0x5B4530)                    |
//! | `enex`  | `LoadEntryExit` (0x5B8030)                 |
//! | `pick`  | `LoadPickup` (0x5B47B0)                    |
//! | `cars`  | `LoadCarGenerator(linea, 0)` (0x5B4740)    |
//! | `jump`  | `LoadStuntJump` (0x5B45D0)                 |
//! | `tcyc`  | `LoadTimeCyclesModifier` (0x5B81D0)        |
//! | `auzo`  | `LoadAudioZone` (0x5B4D70)                 |
//! | `path`  | la primera linea de datos termina la lectura del fichero (formato obsoleto) |
//!
//! Las entidades de `inst` se guardan en el array global `gpLoadedInstances` (0xBCC0E0) porque
//! despues `SetupRelatedIpls` anade las de los IPL binarios del streaming y `LinkLods` enlaza
//! todas con sus LOD usando ese mismo array.
//!
//! En el original hay un caso para una seccion 7 (caminos) al que ninguna cabecera lleva:
//! codigo muerto, no se reproduce.

use super::{cstr, file_mgr_close_file, file_mgr_open_file, global, next_line};
use crate::hooks::{Hook, Kind};
use crate::log;
use core::ffi::{c_char, c_void, CStr};

pub const LOAD_SCENE: usize = 0x005B_8700;

const NUM_INSTANCES: usize = 0x00BC_C0D8; // i32 gNumInstances
const INSTANCES: usize = 0x00BC_C0E0; // CEntity* gpLoadedInstances[]

crate::original_cdecl! {
    fn load_object_instance(line: *const c_char) -> *mut c_void = 0x538690;
    fn load_zone(line: *const c_char) = 0x5B4AB0;
    fn load_cull_zone(line: *const c_char) = 0x5B4B40;
    fn load_occlusion_volume(line: *const c_char, filename: *const c_char) = 0x5B4C80;
    fn load_garage(line: *const c_char) = 0x5B4530;
    fn load_entry_exit(line: *const c_char) = 0x5B8030;
    fn load_pickup(line: *const c_char) = 0x5B47B0;
    fn load_car_generator(line: *const c_char, ipl_id: i32) = 0x5B4740;
    fn load_stunt_jump(line: *const c_char) = 0x5B45D0;
    fn load_time_cycles_modifier(line: *const c_char) = 0x5B81D0;
    fn load_audio_zone(line: *const c_char) = 0x5B4D70;

    // CIplStore (nombres de gta-reversed)
    fn ipl_get_new_entity_index_array(count: i32) -> i32 = 0x404780;
    fn ipl_get_entity_index_array(index: i32) -> *mut *mut c_void = 0x4047B0;
    fn ipl_setup_related_ipls(filename: *const c_char, index: i32, instances: *mut *mut c_void) -> i32 = 0x404DE0;
    fn ipl_remove_related_ipls(index: i32) = 0x405110;
    fn link_lods(num_instances: i32) = 0x5B51E0;
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    None,
    Path,
    Inst,
    Mult,
    Zone,
    Cull,
    Occl,
    Grge,
    Enex,
    Pick,
    Cars,
    Jump,
    Tcyc,
    Auzo,
}

fn section_header(line: &[u8]) -> Option<Section> {
    let s = match line.get(..4)? {
        b"inst" => Section::Inst,
        b"mult" => Section::Mult,
        b"zone" => Section::Zone,
        b"cull" => Section::Cull,
        b"path" => Section::Path,
        b"occl" => Section::Occl,
        b"grge" => Section::Grge,
        b"enex" => Section::Enex,
        b"pick" => Section::Pick,
        b"cars" => Section::Cars,
        b"jump" => Section::Jump,
        b"tcyc" => Section::Tcyc,
        b"auzo" => Section::Auzo,
        _ => return None,
    };
    Some(s)
}

/// Reimplementacion de `CFileLoader::LoadScene` (0x5B8700).
pub extern "C" fn load_scene(filename: *const c_char) {
    unsafe {
        let num = global::<i32>(NUM_INSTANCES);
        *num = 0;

        let file = file_mgr_open_file(filename, c"rb".as_ptr());
        if file.is_null() {
            log!(
                "LoadScene: no se puede abrir {}",
                CStr::from_ptr(filename).to_string_lossy()
            );
            return;
        }

        let mut section = Section::None;
        let mut data_lines = 0u32;

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
            data_lines += 1;
            match section {
                Section::Inst => {
                    let entity = load_object_instance(p);
                    *((INSTANCES + *num as usize * 4) as *mut *mut c_void) = entity;
                    *num += 1;
                }
                Section::Zone => load_zone(p),
                Section::Cull => load_cull_zone(p),
                Section::Occl => load_occlusion_volume(p, filename),
                Section::Grge => load_garage(p),
                Section::Enex => load_entry_exit(p),
                Section::Pick => load_pickup(p),
                Section::Cars => load_car_generator(p, 0),
                Section::Jump => load_stunt_jump(p),
                Section::Tcyc => load_time_cycles_modifier(p),
                Section::Auzo => load_audio_zone(p),
                Section::Mult | Section::None => {}
                // Como el original: una linea de datos en `path` corta la lectura.
                Section::Path => break,
            }
        }
        file_mgr_close_file(file);

        // Copia las entidades a un array propio del IplStore
        let mut ipl_index = -1;
        let count = *num;
        if count > 0 {
            ipl_index = ipl_get_new_entity_index_array(count);
            let dst = ipl_get_entity_index_array(ipl_index);
            core::ptr::copy_nonoverlapping(INSTANCES as *const *mut c_void, dst, *num as usize);
        }

        // Anade las entidades de los IPL binarios relacionados y enlaza los LOD
        let end = (INSTANCES + *num as usize * 4) as *mut *mut c_void;
        let total = ipl_setup_related_ipls(filename, ipl_index, end);
        link_lods(total);
        ipl_remove_related_ipls(ipl_index);

        log!(
            "IPL {}: {data_lines} lineas, {count} instancias, {total} con IPL relacionados",
            CStr::from_ptr(filename).to_string_lossy()
        );
    }
}

pub fn register(v: &mut Vec<Hook>) {
    v.push(Hook {
        name: "CFileLoader::LoadScene",
        addr: LOAD_SCENE,
        kind: Kind::Replace {
            func: load_scene as *const () as usize,
        },
        enabled_by_default: true,
    });
}
