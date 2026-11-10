# =============================================================================
# DBV Unreal AI — Piel limpia y gestión de cuerpo oculto bajo prendas MetaHuman
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""Gestión de textura de piel limpia (sin ropa interior gris pintada) y cuerpo oculto.

@spec REQ-86, REQ-81
"""

from __future__ import annotations

import os
from typing import Any

try:
    import numpy as np
    from PIL import Image, ImageFilter
    HAS_NUMPY_PIL = True
except ImportError:
    np = None  # type: ignore[assignment]
    Image = None  # type: ignore[assignment]
    ImageFilter = None  # type: ignore[assignment]
    HAS_NUMPY_PIL = False

try:
    import unreal
    HAS_UNREAL = True
except ImportError:
    unreal = None  # type: ignore[assignment]
    HAS_UNREAL = False


def detect_underwear_layout(
    specular_arr: Any,
    base_color_arr: Any,
    green_diff_threshold: float = 25.0,
    grey_saturation_threshold: float = 35.0,
    min_coverage_pct: float = 0.5,
) -> tuple[bool, Any | None, float]:
    """Detecta si las texturas del cuerpo contienen la disposición de ropa interior gris pintada.

    El mapa especular del MetaHuman presenta una clara dominancia en el canal verde
    en la tela gris, mientras que en el mapa de color base los píxeles tienen baja
    saturación cromática (gris neutro).

    @spec REQ-86
    """
    has_underwear = False
    mask = None
    pct = 0.0

    if HAS_NUMPY_PIL and specular_arr is not None and base_color_arr is not None:
        spec = np.asarray(specular_arr, dtype=np.float32)
        base = np.asarray(base_color_arr, dtype=np.float32)
        if spec.ndim >= 2 and base.ndim >= 2:
            r_s = spec[..., 0]
            g_s = spec[..., 1]
            b_s = spec[..., 2] if spec.shape[-1] > 2 else spec[..., 1]
            spec_green = (g_s - np.maximum(r_s, b_s)) > green_diff_threshold

            r_b = base[..., 0]
            g_b = base[..., 1]
            b_b = base[..., 2] if base.shape[-1] > 2 else base[..., 1]
            grey_diff = np.maximum(np.abs(r_b - g_b), np.maximum(np.abs(g_b - b_b), np.abs(r_b - b_b)))
            base_grey = (grey_diff < grey_saturation_threshold) & (base.mean(axis=-1) > 35.0) & (base.mean(axis=-1) < 190.0)

            raw_mask = spec_green & base_grey
            total_px = float(raw_mask.size)
            matched_px = float(np.sum(raw_mask))
            pct = (matched_px / max(total_px, 1.0)) * 100.0

            if pct >= min_coverage_pct:
                has_underwear = True
                mask = raw_mask

    return (has_underwear, mask, pct)


def clean_texture_maps(
    base_color_arr: Any,
    normal_arr: Any,
    specular_arr: Any,
) -> tuple[Any, Any, Any, dict[str, Any]]:
    """Genera copias limpias de los mapas de color, normal y especular eliminando la ropa interior.

    Muestrea la piel sana lejos del ombligo y lunares para evitar artefactos repetidos,
    aplicando un relleno suave y bordes difuminados.

    @spec REQ-86
    """
    info: dict[str, Any] = {
        "has_underwear": False,
        "coverage_pct": 0.0,
        "cleaned": False,
    }

    if not HAS_NUMPY_PIL or base_color_arr is None or normal_arr is None or specular_arr is None:
        info["error"] = "numpy and pillow are required for texture cleaning"
        res = (base_color_arr, normal_arr, specular_arr, info)
    else:
        has_uw, mask, pct = detect_underwear_layout(specular_arr, base_color_arr)
        info["has_underwear"] = has_uw
        info["coverage_pct"] = round(pct, 2)

        if not has_uw or mask is None:
            info["cleaned"] = False
            info["notice"] = "unrecognized_underwear_layout_skipped"
            res = (base_color_arr, normal_arr, specular_arr, info)
        else:
            base_f = np.asarray(base_color_arr, dtype=np.float32).copy()
            norm_f = np.asarray(normal_arr, dtype=np.float32).copy()
            spec_f = np.asarray(specular_arr, dtype=np.float32).copy()

            # Dilatar máscara ligeramente (MaxFilter 7x7) para cubrir bordes antialias
            raw_mask_u8 = (mask.astype(np.uint8)) * 255
            mask_img = Image.fromarray(raw_mask_u8).filter(ImageFilter.MaxFilter(7))
            dilated = np.array(mask_img) > 128

            # Transición suave en bordes (feathering)
            feather_f = np.array(mask_img.filter(ImageFilter.GaussianBlur(3)), dtype=np.float32) / 255.0
            feather_3d = feather_f[..., None] if feather_f.ndim == 2 else feather_f

            # Muestrear piel sana lejos del ombligo (ombligo suele estar cerca del centro Y: 0.35-0.5, X: 0.45-0.55)
            h, w = base_f.shape[:2]
            y_indices, x_indices = np.indices((h, w))
            avoid_navel = ~((y_indices > int(h * 0.35)) & (y_indices < int(h * 0.55)) & (x_indices > int(w * 0.42)) & (x_indices < int(w * 0.58)))

            r_b = base_f[..., 0]
            g_b = base_f[..., 1]
            b_b = base_f[..., 2] if base_f.shape[-1] > 2 else base_f[..., 1]
            is_skin = (~dilated) & avoid_navel & (base_f.mean(axis=-1) > 50.0) & (r_b > g_b) & (g_b > b_b)

            if np.sum(is_skin) > 100:
                median_skin = np.median(base_f[is_skin], axis=0)
                median_spec = np.median(spec_f[is_skin], axis=0)
            else:
                median_skin = np.array([185.0, 130.0, 105.0], dtype=np.float32)
                median_spec = np.array([200.0, 110.0, 0.0], dtype=np.float32)

            # Relleno difuso para color base
            filled_base = base_f.copy()
            filled_base[dilated] = median_skin
            base_pil = Image.fromarray(np.clip(filled_base, 0, 255).astype(np.uint8))
            smooth_base = np.array(base_pil.filter(ImageFilter.GaussianBlur(15)), dtype=np.float32)
            clean_base = base_f * (1.0 - feather_3d) + smooth_base * feather_3d

            # Relleno difuso para especular
            filled_spec = spec_f.copy()
            filled_spec[dilated] = median_spec
            spec_pil = Image.fromarray(np.clip(filled_spec, 0, 255).astype(np.uint8))
            smooth_spec = np.array(spec_pil.filter(ImageFilter.GaussianBlur(15)), dtype=np.float32)
            clean_spec = spec_f * (1.0 - feather_3d) + smooth_spec * feather_3d

            # Normal neutra en el espacio tangente [128, 128, 255]
            neutral_norm = np.array([128.0, 128.0, 255.0], dtype=np.float32)
            clean_norm = norm_f * (1.0 - feather_3d) + neutral_norm * feather_3d

            out_base = np.clip(clean_base, 0, 255).astype(np.uint8)
            out_norm = np.clip(clean_norm, 0, 255).astype(np.uint8)
            out_spec = np.clip(clean_spec, 0, 255).astype(np.uint8)

            info["cleaned"] = True
            info["clean_skin_median"] = [round(float(v), 1) for v in median_skin[:3]]
            res = (out_base, out_norm, out_spec, info)

    return res


def export_texture_to_png(texture_asset: Any, output_file_path: str) -> bool:
    """Exporta un asset ``Texture2D`` de Unreal a un archivo PNG en disco."""
    success = False
    if HAS_UNREAL and texture_asset is not None:
        try:
            task = unreal.AssetExportTask()
            task.object = texture_asset
            task.filename = output_file_path
            task.automated = True
            task.prompt = False
            task.replace_identical = True
            task.exporter = unreal.TextureExporterPNG()
            success = bool(unreal.Exporter.run_asset_export_task(task))
        except Exception:  # noqa: BLE001
            success = False
    return success


def import_png_to_texture(image_file_path: str, destination_path: str, destination_name: str) -> Any:
    """Importa un archivo PNG como asset ``Texture2D`` en Unreal Engine."""
    imported_asset = None
    if HAS_UNREAL and os.path.isfile(image_file_path):
        try:
            task = unreal.AssetImportTask()
            task.filename = image_file_path
            task.destination_path = destination_path
            task.destination_name = destination_name
            task.automated = True
            task.replace_existing = True
            task.save = False

            unreal.AssetToolsHelpers.get_asset_tools().import_asset_tasks([task])
            objs = task.get_objects()
            if objs:
                imported_asset = objs[0]
                unreal.EditorAssetLibrary.save_loaded_asset(imported_asset, False)
        except Exception:  # noqa: BLE001
            imported_asset = None
    return imported_asset


def find_body_texture_parameters(material_asset: Any) -> dict[str, Any]:
    """Extrae las texturas de color, normal y especular de una instancia de material de MetaHuman."""
    textures: dict[str, Any] = {}
    if material_asset is not None and hasattr(material_asset, "texture_parameter_values"):
        for tp in material_asset.texture_parameter_values:
            name = str(tp.parameter_info.name)
            val = tp.parameter_value
            if val is not None:
                if name in ("BaseColor", "Color_MAIN"):
                    textures["base_color"] = val
                    textures["base_color_param"] = name
                elif name in ("Normal", "Normal_MAIN"):
                    textures["normal"] = val
                    textures["normal_param"] = name
                elif name in ("Specular", "Roughness_MAIN"):
                    textures["specular"] = val
                    textures["specular_param"] = name
                elif name == "UnderwearMask":
                    textures["underwear_mask"] = val
    return textures


def prepare_clean_skin(actor_label: str, destination_folder: str | None = None) -> dict[str, Any]:
    """Crea y aplica un material hijo con texturas limpias (sin ropa interior) al cuerpo del MetaHuman.

    Se hace una sola vez por personaje y se reutiliza entre prendas.
    Si el personaje no tiene disposición reconocida de ropa interior, se omite y se avisa.

    @spec REQ-86
    """
    result: dict[str, Any] = {
        "success": False,
        "actor": actor_label,
        "cleaned": False,
        "reused": False,
        "material_path": None,
    }

    if not HAS_UNREAL:
        result["error"] = "Unreal Engine environment not available"
        outcome = result
    elif not HAS_NUMPY_PIL:
        result["error"] = "PythonFoundationPackages (numpy, Pillow) required for skin cleaning"
        outcome = result
    else:
        actors = unreal.get_editor_subsystem(unreal.EditorActorSubsystem).get_all_level_actors()
        matched = [a for a in actors if a.get_actor_label() == actor_label]
        if not matched:
            result["error"] = f"actor not found: {actor_label}"
            outcome = result
        else:
            actor = matched[0]
            comps = {c.get_name(): c for c in actor.get_components_by_class(unreal.SkeletalMeshComponent)}
            body_comp = comps.get("Body")
            if body_comp is None:
                result["error"] = f"actor '{actor_label}' has no Body component"
                outcome = result
            else:
                orig_mat = body_comp.get_material(0)
                if orig_mat is None:
                    result["error"] = f"Body component of '{actor_label}' has no assigned material"
                    outcome = result
                else:
                    clean_folder = destination_folder or f"/Game/DBV/MetaHumanOutfits/{actor_label}"
                    clean_mi_path = f"{clean_folder}/MI_Body_PielLimpia"

                    # 1. Reutilizar si ya existe
                    if unreal.EditorAssetLibrary.does_asset_exist(clean_mi_path):
                        existing_mi = unreal.load_asset(clean_mi_path)
                        body_comp.modify()
                        body_comp.set_material(0, existing_mi)
                        result["success"] = True
                        result["reused"] = True
                        result["cleaned"] = True
                        result["material_path"] = clean_mi_path
                        outcome = result
                    else:
                        tex_params = find_body_texture_parameters(orig_mat)
                        base_tex = tex_params.get("base_color")
                        norm_tex = tex_params.get("normal")
                        spec_tex = tex_params.get("specular")

                        # Si tiene UnderwearMask shaderizado (Pablo/David), podemos desactivarlo directamente
                        has_shader_underwear = "underwear_mask" in tex_params or hasattr(orig_mat, "scalar_parameter_values") and any(
                            sp.parameter_info.name == "UnderwearNormalIntensity" for sp in orig_mat.scalar_parameter_values
                        )

                        if base_tex is None or norm_tex is None or spec_tex is None:
                            # Caso sin las 3 texturas estándar
                            result["success"] = True
                            result["cleaned"] = False
                            result["notice"] = "unrecognized_underwear_layout_skipped"
                            outcome = result
                        else:
                            temp_dir = os.path.join(unreal.Paths.project_saved_dir(), "DBV_Skin_Temp")
                            os.makedirs(temp_dir, exist_ok=True)
                            base_png = os.path.join(temp_dir, "base.png")
                            norm_png = os.path.join(temp_dir, "norm.png")
                            spec_png = os.path.join(temp_dir, "spec.png")

                            exp_base = export_texture_to_png(base_tex, base_png)
                            exp_norm = export_texture_to_png(norm_tex, norm_png)
                            exp_spec = export_texture_to_png(spec_tex, spec_png)

                            if not (exp_base and exp_norm and exp_spec):
                                result["error"] = "failed to export body textures for analysis"
                                outcome = result
                            else:
                                arr_b = np.array(Image.open(base_png).convert("RGB"))
                                arr_n = np.array(Image.open(norm_png).convert("RGB"))
                                arr_s = np.array(Image.open(spec_png).convert("RGB"))

                                clean_b, clean_n, clean_s, clean_info = clean_texture_maps(arr_b, arr_n, arr_s)

                                if not clean_info.get("cleaned"):
                                    # Sin disposición de ropa interior reconocida
                                    result["success"] = True
                                    result["cleaned"] = False
                                    result["notice"] = "unrecognized_underwear_layout_skipped"
                                    result["coverage_pct"] = clean_info.get("coverage_pct", 0.0)
                                    outcome = result
                                else:
                                    out_b_path = os.path.join(temp_dir, "T_Body_BaseColor_Limpio.png")
                                    out_n_path = os.path.join(temp_dir, "T_Body_Normal_Limpio.png")
                                    out_s_path = os.path.join(temp_dir, "T_Body_Specular_Limpio.png")

                                    Image.fromarray(clean_b).save(out_b_path)
                                    Image.fromarray(clean_n).save(out_n_path)
                                    Image.fromarray(clean_s).save(out_s_path)

                                    tex_folder = f"{clean_folder}/Textures"
                                    u_base = import_png_to_texture(out_b_path, tex_folder, "T_Body_BaseColor_Limpio")
                                    u_norm = import_png_to_texture(out_n_path, tex_folder, "T_Body_Normal_Limpio")
                                    u_spec = import_png_to_texture(out_s_path, tex_folder, "T_Body_Specular_Limpio")

                                    # Crear MaterialInstanceConstant hijo
                                    factory = unreal.MaterialInstanceConstantFactoryNew()
                                    new_mi = unreal.AssetToolsHelpers.get_asset_tools().create_asset(
                                        "MI_Body_PielLimpia", clean_folder, unreal.MaterialInstanceConstant, factory
                                    )
                                    if new_mi is None:
                                        result["error"] = f"failed to create MaterialInstance at {clean_mi_path}"
                                        outcome = result
                                    else:
                                        unreal.MaterialEditingLibrary.set_material_instance_parent(new_mi, orig_mat)
                                        if u_base and tex_params.get("base_color_param"):
                                            unreal.MaterialEditingLibrary.set_material_instance_texture_parameter_value(
                                                new_mi, tex_params["base_color_param"], u_base
                                            )
                                        if u_norm and tex_params.get("normal_param"):
                                            unreal.MaterialEditingLibrary.set_material_instance_texture_parameter_value(
                                                new_mi, tex_params["normal_param"], u_norm
                                            )
                                        if u_spec and tex_params.get("specular_param"):
                                            unreal.MaterialEditingLibrary.set_material_instance_texture_parameter_value(
                                                new_mi, tex_params["specular_param"], u_spec
                                            )
                                        if has_shader_underwear:
                                            unreal.MaterialEditingLibrary.set_material_instance_scalar_parameter_value(
                                                new_mi, "UnderwearNormalIntensity", 0.0
                                            )
                                        unreal.EditorAssetLibrary.save_loaded_asset(new_mi, False)

                                        body_comp.modify()
                                        body_comp.set_material(0, new_mi)

                                        result["success"] = True
                                        result["cleaned"] = True
                                        result["material_path"] = clean_mi_path
                                        result["coverage_pct"] = clean_info.get("coverage_pct", 0.0)
                                        outcome = result

    return outcome


def trim_body_mesh_under_garment(
    actor_label: str,
    garment_kind: str,
    waist_z: float | None = None,
    hem_z: float | None = None,
    destination_folder: str | None = None,
) -> dict[str, Any]:
    """En faldas y vestidos, crea una copia del cuerpo sin los triángulos tapados por la prenda.

    @spec REQ-86
    """
    result: dict[str, Any] = {
        "success": False,
        "actor": actor_label,
        "trimmed": False,
        "mesh_path": None,
    }

    if garment_kind not in ("skirt", "dress"):
        result["success"] = True
        result["trimmed"] = False
        result["notice"] = f"trimming not required for garment kind '{garment_kind}'"
        outcome = result
    elif not HAS_UNREAL:
        result["error"] = "Unreal Engine environment not available"
        outcome = result
    else:
        actors = unreal.get_editor_subsystem(unreal.EditorActorSubsystem).get_all_level_actors()
        matched = [a for a in actors if a.get_actor_label() == actor_label]
        if not matched:
            result["error"] = f"actor not found: {actor_label}"
            outcome = result
        else:
            actor = matched[0]
            comps = {c.get_name(): c for c in actor.get_components_by_class(unreal.SkeletalMeshComponent)}
            body_comp = comps.get("Body")
            body_skm = body_comp.get_skeletal_mesh_asset() if body_comp else None
            if body_comp is None or body_skm is None:
                result["error"] = f"actor '{actor_label}' has no valid Body SkeletalMesh"
                outcome = result
            else:
                asset_utils = unreal.GeometryScript_AssetUtils
                lod_opts = unreal.GeometryScriptMeshReadLOD()
                dyn = unreal.DynamicMesh()
                dyn, _ = asset_utils.copy_mesh_from_skeletal_mesh(body_skm, dyn, unreal.GeometryScriptCopyMeshFromAssetOptions(), lod_opts)

                mq = unreal.GeometryScript_MeshQueries
                bbox = mq.get_mesh_bounding_box(dyn)
                b_h = float(bbox.max.z - bbox.min.z)

                # Definir zona de recorte según kind
                # Para falda: desde cintura hasta dobladillo (cadera y muslos superiores)
                # Para vestido: desde pecho hasta dobladillo
                z_top = waist_z if waist_z is not None else float(bbox.min.z + b_h * 0.58)
                z_bot = hem_z if hem_z is not None else float(bbox.min.z + b_h * 0.40)
                if garment_kind == "dress":
                    z_top = float(bbox.min.z + b_h * 0.72)

                # Seleccionar triángulos dentro de la caja de recorte centrada en el cuerpo
                # Evitar brazos: limitar en X a ±22 cm
                trim_box = unreal.Box(
                    unreal.Vector(-22.0, -20.0, min(z_bot, z_top)),
                    unreal.Vector(22.0, 20.0, max(z_bot, z_top)),
                )

                sel = unreal.GeometryScript_MeshSelection.select_mesh_elements_in_box(
                    dyn, trim_box, unreal.GeometryScriptMeshSelection(), unreal.GeometryScriptElementContainment.CONTAINED
                )
                dyn, del_count = unreal.GeometryScript_MeshEdits.delete_selected_triangles_from_mesh(dyn, sel)

                clean_folder = destination_folder or f"/Game/DBV/MetaHumanOutfits/{actor_label}"
                dest_skm_path = f"{clean_folder}/SK_Body_{actor_label}_Trimmed"

                skm_opts = unreal.GeometryScriptCreateNewSkeletalMeshAssetOptions()
                skm_opts.enable_recompute_normals = False
                skm_opts.enable_recompute_tangents = True
                skeleton = body_skm.get_editor_property("skeleton") if hasattr(body_skm, "get_editor_property") else None

                new_skm, _ = unreal.GeometryScript_NewAssetUtils.create_new_skeletal_mesh_asset_from_mesh(
                    dyn, skeleton, dest_skm_path, skm_opts
                )
                if new_skm is None:
                    result["error"] = f"failed to create trimmed SkeletalMesh at {dest_skm_path}"
                    outcome = result
                else:
                    unreal.EditorAssetLibrary.save_loaded_asset(new_skm, False)
                    body_comp.modify()
                    body_comp.set_skeletal_mesh_asset(new_skm)

                    result["success"] = True
                    result["trimmed"] = True
                    result["deleted_triangles"] = int(del_count) if isinstance(del_count, (int, float)) else 0
                    result["mesh_path"] = dest_skm_path
                    outcome = result

    return outcome


def restore_full_body_mesh(actor_label: str, original_body_mesh_path: str | None = None) -> dict[str, Any]:
    """Restaura la malla completa original del cuerpo en el componente ``Body``.

    @spec REQ-86, REQ-85
    """
    result: dict[str, Any] = {
        "success": False,
        "actor": actor_label,
        "restored": False,
    }

    if not HAS_UNREAL:
        result["error"] = "Unreal Engine environment not available"
        outcome = result
    else:
        actors = unreal.get_editor_subsystem(unreal.EditorActorSubsystem).get_all_level_actors()
        matched = [a for a in actors if a.get_actor_label() == actor_label]
        if not matched:
            result["error"] = f"actor not found: {actor_label}"
            outcome = result
        else:
            actor = matched[0]
            comps = {c.get_name(): c for c in actor.get_components_by_class(unreal.SkeletalMeshComponent)}
            body_comp = comps.get("Body")
            if body_comp is None:
                result["error"] = f"actor '{actor_label}' has no Body component"
                outcome = result
            else:
                target_mesh = None
                if original_body_mesh_path:
                    target_mesh = unreal.load_asset(original_body_mesh_path)
                if target_mesh is None:
                    # Cargar malla completa por defecto de MetaHumans
                    target_mesh = unreal.load_asset("/Game/MetaHumans/Common/Common/m_tal_nrw_body_preview")
                if target_mesh is not None:
                    body_comp.modify()
                    body_comp.set_skeletal_mesh_asset(target_mesh)
                    result["success"] = True
                    result["restored"] = True
                    result["mesh_path"] = target_mesh.get_path_name()
                else:
                    result["error"] = "could not resolve full body mesh asset to restore"
                outcome = result

    return outcome

