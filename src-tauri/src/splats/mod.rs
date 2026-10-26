// =============================================================================
// DBV Unreal AI — Gaussian Splats: lectura y malla de vista previa para Unreal
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-41
//
// Spike P-10 (plugin propio «sencillo»): un splat se lleva a Unreal como UNA malla estática con un quad por
// gaussiana. Cada quad ya está en su sitio (centro ± radio en el plano XY de glTF); el material (creado por Python
// en el editor) lo gira hacia la cámara con World Position Offset y aplica la caída gaussiana con opacidad
// enmascarada y dither (sin ordenar por profundidad). Por quad:
//   · TEXCOORD_0 = esquina (−1/+1, −1/+1): de ella salen la caída y el giro;
//   · TEXCOORD_1 = (radio en cm, 0);
//   · TEXCOORD_2 = color base (SH0) R y G, en lineal; TEXCOORD_3 = B y opacidad. Van en UV y no en COLOR_0
//     porque Interchange no trae el color de vértice con sus opciones por defecto (visto en vivo en el spike).
// Calidad de vista previa: gaussianas isótropas (radio = mediana de las tres escalas), solo el color base.

use std::io::{BufRead, BufReader, Read};
use std::path::Path;

use crate::unreal::{io_error, UnrealError};

/// Una gaussiana lista para la malla: centro en metros (glTF, Y arriba), color+opacidad y radio en metros.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Splat {
    pub center: [f32; 3],
    pub rgba: [u8; 4],
    pub radius: f32,
}

/// Coeficiente del armónico esférico de grado 0 (color base de 3DGS).
const SH_C0: f32 = 0.282_094_8;
/// El quad cubre ±`SIGMAS` desviaciones: más allá la gaussiana ya no se ve.
pub const SIGMAS: f32 = 2.5;
/// Radio mínimo (m): por debajo el importador tomaría el quad por degenerado.
const MIN_RADIUS: f32 = 0.0005;

fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

/// Lee un `.ply` de 3D Gaussian Splatting (binario little endian, propiedades `float`). Los de World Labs ya vienen
/// con Y arriba, como glTF (su `ground_plane_offset` se resta a Y): se copian tal cual. Suponer la convención de
/// OpenCV (Y abajo) dejó el mundo boca abajo en el spike P-10.
pub fn read_ply(path: &Path) -> Result<Vec<Splat>, UnrealError> {
    let file = std::fs::File::open(path).map_err(|error| io_error(path, &error))?;
    let mut reader = BufReader::new(file);
    let mut count = 0usize;
    let mut names: Vec<String> = Vec::new();
    let mut binary = false;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).map_err(|error| io_error(path, &error))? == 0 {
            return Err(UnrealError::Format("el .ply no tiene «end_header»".into()));
        }
        let words: Vec<&str> = line.split_whitespace().collect();
        match words.as_slice() {
            ["format", "binary_little_endian", ..] => binary = true,
            ["element", "vertex", n] => count = n.parse().map_err(|_| UnrealError::Format("número de vértices no válido".into()))?,
            ["property", "float", name] => names.push((*name).to_string()),
            ["property", kind, _] => return Err(UnrealError::Format(format!("propiedad «{kind}» no admitida: solo .ply de 3DGS con floats"))),
            ["end_header"] => break,
            _ => {}
        }
    }
    let index = |name: &str| names.iter().position(|n| n == name).ok_or_else(|| UnrealError::Format(format!("al .ply le falta «{name}»: no es un splat de 3DGS")));
    if !binary {
        return Err(UnrealError::Format("solo .ply binarios little endian".into()));
    }
    let [x, y, z, r, g, b, o, s0, s1, s2] = ["x", "y", "z", "f_dc_0", "f_dc_1", "f_dc_2", "opacity", "scale_0", "scale_1", "scale_2"].map(index);
    let (x, y, z, r, g, b, o, s0, s1, s2) = (x?, y?, z?, r?, g?, b?, o?, s0?, s1?, s2?);
    let stride = names.len() * 4;
    let mut row = vec![0u8; stride];
    let mut splats = Vec::with_capacity(count);
    for _ in 0..count {
        reader.read_exact(&mut row).map_err(|error| io_error(path, &error))?;
        let f = |i: usize| f32::from_le_bytes([row[i * 4], row[i * 4 + 1], row[i * 4 + 2], row[i * 4 + 3]]);
        let channel = |i: usize| ((0.5 + SH_C0 * f(i)).clamp(0.0, 1.0) * 255.0).round() as u8;
        let mut scales = [f(s0).exp(), f(s1).exp(), f(s2).exp()];
        scales.sort_by(f32::total_cmp);
        let radius = scales[1] * SIGMAS;
        if radius.is_finite() && radius >= MIN_RADIUS {
            splats.push(Splat { center: [f(x), f(y), f(z)], rgba: [channel(r), channel(g), channel(b), (sigmoid(f(o)) * 255.0).round() as u8], radius });
        }
    }
    Ok(splats)
}

/// Los `max` splats más opacos: una vista previa ligera de un splat de millones. Elegir por área se quedaba con las
/// manchas grandes del fondo y perdía el detalle (spike P-10, en vivo).
pub fn most_visible(mut splats: Vec<Splat>, max: usize) -> Vec<Splat> {
    let weight = |s: &Splat| f32::from(s.rgba[3]);
    if splats.len() > max {
        splats.select_nth_unstable_by(max, |a, b| weight(b).total_cmp(&weight(a)));
        splats.truncate(max);
    }
    splats
}

/// Canal sRGB de 8 bits a lineal.
fn srgb_to_linear(channel: u8) -> f32 {
    let c = f32::from(channel) / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn pad4(bytes: &mut Vec<u8>, fill: u8) {
    while !bytes.len().is_multiple_of(4) {
        bytes.push(fill);
    }
}

/// Escribe la malla de vista previa como GLB (un quad por splat, ver la cabecera del módulo).
pub fn write_glb(splats: &[Splat], path: &Path) -> Result<(), UnrealError> {
    const CORNERS: [[f32; 2]; 4] = [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]];
    let vertices = splats.len() * 4;
    let (mut positions, mut corners, mut radii, mut red_green, mut blue_alpha, mut indices) = (Vec::with_capacity(vertices * 12), Vec::with_capacity(vertices * 8), Vec::with_capacity(vertices * 8), Vec::with_capacity(vertices * 8), Vec::with_capacity(vertices * 8), Vec::with_capacity(splats.len() * 24));
    let (mut min, mut max) = ([f32::MAX; 3], [f32::MIN; 3]);
    for (n, splat) in splats.iter().enumerate() {
        for corner in CORNERS {
            let p = [splat.center[0] + corner[0] * splat.radius, splat.center[1] + corner[1] * splat.radius, splat.center[2]];
            for axis in 0..3 {
                min[axis] = min[axis].min(p[axis]);
                max[axis] = max[axis].max(p[axis]);
                positions.extend_from_slice(&p[axis].to_le_bytes());
            }
            corners.extend(corner.iter().flat_map(|c| c.to_le_bytes()));
            radii.extend([splat.radius * 100.0, 0.0].iter().flat_map(|c| c.to_le_bytes()));
            // glTF guarda el color de vértice en lineal (el de 3DGS es sRGB): en float, que es lo que lee Interchange
            // (con u8 normalizado el color llegaba negro, visto en vivo en el spike).
            let rgba = [srgb_to_linear(splat.rgba[0]), srgb_to_linear(splat.rgba[1]), srgb_to_linear(splat.rgba[2]), f32::from(splat.rgba[3]) / 255.0];
            red_green.extend(rgba[..2].iter().flat_map(|c| c.to_le_bytes()));
            blue_alpha.extend(rgba[2..].iter().flat_map(|c| c.to_le_bytes()));
        }
        let base = (n * 4) as u32;
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3].iter().flat_map(|i| i.to_le_bytes()));
    }
    let mut bin = Vec::new();
    let mut views = Vec::new();
    for (data, target) in [(&positions, 34962), (&corners, 34962), (&radii, 34962), (&red_green, 34962), (&blue_alpha, 34962), (&indices, 34963)] {
        views.push(serde_json::json!({ "buffer": 0, "byteOffset": bin.len(), "byteLength": data.len(), "target": target }));
        bin.extend_from_slice(data);
        pad4(&mut bin, 0);
    }
    let json = serde_json::json!({
        "asset": { "version": "2.0", "generator": "DBV Unreal AI splat preview" },
        "scene": 0,
        "scenes": [{ "nodes": [0] }],
        "nodes": [{ "mesh": 0, "name": "Splats" }],
        "meshes": [{ "name": "Splats", "primitives": [{ "attributes": { "POSITION": 0, "TEXCOORD_0": 1, "TEXCOORD_1": 2, "TEXCOORD_2": 3, "TEXCOORD_3": 4 }, "indices": 5, "mode": 4 }] }],
        "buffers": [{ "byteLength": bin.len() }],
        "bufferViews": views,
        "accessors": [
            { "bufferView": 0, "componentType": 5126, "count": vertices, "type": "VEC3", "min": min, "max": max },
            { "bufferView": 1, "componentType": 5126, "count": vertices, "type": "VEC2" },
            { "bufferView": 2, "componentType": 5126, "count": vertices, "type": "VEC2" },
            { "bufferView": 3, "componentType": 5126, "count": vertices, "type": "VEC2" },
            { "bufferView": 4, "componentType": 5126, "count": vertices, "type": "VEC2" },
            { "bufferView": 5, "componentType": 5125, "count": splats.len() * 6, "type": "SCALAR" }
        ]
    });
    let mut text = json.to_string().into_bytes();
    pad4(&mut text, b' ');
    let total = 12 + 8 + text.len() + 8 + bin.len();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(b"glTF");
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&(text.len() as u32).to_le_bytes());
    out.extend_from_slice(b"JSON");
    out.extend_from_slice(&text);
    out.extend_from_slice(&(bin.len() as u32).to_le_bytes());
    out.extend_from_slice(b"BIN\0");
    out.extend_from_slice(&bin);
    std::fs::write(path, out).map_err(|error| io_error(path, &error))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ply(rows: &[[f32; 10]]) -> Vec<u8> {
        let mut out = format!("ply\nformat binary_little_endian 1.0\nelement vertex {}\n", rows.len()).into_bytes();
        for name in ["x", "y", "z", "f_dc_0", "f_dc_1", "f_dc_2", "opacity", "scale_0", "scale_1", "scale_2"] {
            out.extend(format!("property float {name}\n").bytes());
        }
        out.extend(b"end_header\n");
        for row in rows {
            out.extend(row.iter().flat_map(|v| v.to_le_bytes()));
        }
        out
    }

    /// @spec REQ-41
    #[test]
    fn lee_un_splat_de_3dgs_en_la_convencion_de_gltf() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s.ply");
        std::fs::write(&path, ply(&[[1.0, 2.0, 3.0, 0.0, 10.0, -10.0, 0.0, -2.0, -3.0, -4.0], [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -20.0, -20.0, -20.0]])).unwrap();
        let splats = read_ply(&path).unwrap();
        assert_eq!(splats.len(), 1, "un splat diminuto se descarta");
        assert_eq!(splats[0].center, [1.0, 2.0, 3.0]);
        assert_eq!(splats[0].rgba, [128, 255, 0, 128]);
        assert!((splats[0].radius - (-3.0f32).exp() * SIGMAS).abs() < 1e-6, "radio = mediana de las escalas × sigmas");
        std::fs::write(&path, b"ply\nformat ascii 1.0\nelement vertex 0\nend_header\n").unwrap();
        assert!(read_ply(&path).is_err());
    }

    /// @spec REQ-41
    #[test]
    fn la_vista_previa_se_queda_con_lo_que_mas_se_ve_y_escribe_un_glb_valido() {
        let splat = |opacity: u8, radius: f32| Splat { center: [0.0, 0.0, 0.0], rgba: [1, 2, 3, opacity], radius };
        let kept = most_visible(vec![splat(10, 1.0), splat(255, 1.0), splat(255, 0.1), splat(200, 2.0)], 2);
        assert_eq!(kept.iter().map(|s| (s.rgba[3], s.radius)).collect::<Vec<_>>().len(), 2);
        assert!(kept.iter().all(|s| s.rgba[3] >= 200), "se quedan los más opacos");
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s.glb");
        write_glb(&kept, &path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[0..4], b"glTF");
        assert_eq!(u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize, bytes.len());
        let json_len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
        let json: serde_json::Value = serde_json::from_slice(&bytes[20..20 + json_len]).unwrap();
        assert_eq!(json["accessors"][0]["count"], 8);
        assert_eq!(json["accessors"][5]["count"], 12);
        assert_eq!(json["meshes"][0]["primitives"][0]["attributes"]["TEXCOORD_1"], 2);
        assert_eq!(json["meshes"][0]["primitives"][0]["attributes"]["TEXCOORD_3"], 4, "el color va en UV: Interchange no trae COLOR_0 por defecto");
        assert!((srgb_to_linear(255) - 1.0).abs() < 1e-6 && srgb_to_linear(0) == 0.0 && (srgb_to_linear(128) - 0.2158).abs() < 1e-3);
    }

    /// Spike P-10 con el splat real de World Labs que guardó Wordsmith: `DBV_SPLAT_IN` (.ply) y `DBV_SPLAT_OUT` (.glb).
    #[test]
    #[ignore = "spike manual con un fichero local"]
    fn spike_convierte_un_splat_real() {
        let input = std::env::var("DBV_SPLAT_IN").unwrap();
        let output = std::env::var("DBV_SPLAT_OUT").unwrap();
        let max: usize = std::env::var("DBV_SPLAT_MAX").ok().and_then(|m| m.parse().ok()).unwrap_or(50_000);
        let started = std::time::Instant::now();
        let all = read_ply(Path::new(&input)).unwrap();
        let total = all.len();
        let kept = most_visible(all, max);
        write_glb(&kept, Path::new(&output)).unwrap();
        println!("{total} splats leídos, {} en la vista previa, {:?}", kept.len(), started.elapsed());
    }
}
