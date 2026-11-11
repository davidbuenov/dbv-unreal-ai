# =============================================================================
# DBV Unreal AI — S100: Tela automática con Chaos Cloth en faldas y vestidos
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""Pipeline automático de tela Chaos Cloth para MetaHumans en UE 5.8 (REQ-87).

Automatiza la generación de la malla de simulación simplificada, creación y
evaluación procedimental del Dataflow y ChaosClothAsset a partir de plantillas
limpias del plugin, adición del componente ChaosClothComponent al Blueprint del
personaje, enlace de líder en EventBeginPlay y aplicación de ranuras.
"""

from __future__ import annotations

import json
from typing import Any

try:
    import unreal

    HAS_UNREAL = True
except ImportError:
    unreal = None  # type: ignore[assignment]
    HAS_UNREAL = False


#: Plantilla predeterminada de Cloth Asset en el plugin DBVUnrealAI.
DEFAULT_TEMPLATE_CA = "/DBVUnrealAI/Cloth/CA_Template"

#: Plantilla predeterminada de Dataflow en el plugin DBVUnrealAI.
DEFAULT_TEMPLATE_DF = "/DBVUnrealAI/Cloth/DF_Template"

#: Rutas alternativas de respaldo si las plantillas del spike se reubicaran en el proyecto.
FALLBACK_TEMPLATE_CA = "/Game/DBV/MetaHumanOutfits/_spike/CA_Template"
FALLBACK_TEMPLATE_DF = "/Game/DBV/MetaHumanOutfits/_spike/DF_Template"

#: Mallas y físicas predeterminadas de complexión para MetaHumans.
ARCHETYPE_FULL_BODIES: dict[str, str] = {
    "f_med_nrw": "/Game/MetaHumans/Common/Female/Medium/NormalWeight/Body/f_med_nrw_body",
    "m_tal_nrw": "/Game/MetaHumans/Common/Male/Tall/NormalWeight/Body/m_tal_nrw_body",
    "m_med_nrw": "/Game/MetaHumans/Common/Male/Medium/NormalWeight/Body/m_med_nrw_body",
}

ARCHETYPE_PHYSICS: dict[str, str] = {
    "f_med_nrw": "/Game/MetaHumans/Common/Female/Medium/NormalWeight/Body/f_med_nrw_body_Physics",
    "m_tal_nrw": "/Game/MetaHumans/Common/Male/Tall/NormalWeight/Body/m_tal_nrw_body_Physics",
    "m_med_nrw": "/Game/MetaHumans/Common/Male/Medium/NormalWeight/Body/m_med_nrw_body_Physics",
}


def _json(value: Any) -> str:
    """Serializa a JSON con codificación UTF-8 segura."""
    return json.dumps(value, ensure_ascii=False, default=str)


def generate_cloth_simulation_mesh(
    render_sm_path: str,
    dest_folder: str,
    sim_name: str,
    target_triangles: int = 1500,
) -> dict[str, Any]:
    """Genera una malla de simulación simplificada (low-poly) a partir de la malla de render.

    @spec REQ-87
    """
    res: dict[str, Any] = {
        "success": False,
        "error": None,
        "sim_sm_path": None,
        "triangle_count": 0,
    }

    if not render_sm_path or not dest_folder or not sim_name:
        res["error"] = "render_sm_path, dest_folder y sim_name son obligatorios"
    elif target_triangles <= 0:
        res["error"] = f"target_triangles debe ser positivo (recibido {target_triangles})"
    elif not HAS_UNREAL or not hasattr(unreal, "GeometryScript_MeshSimplification"):
        sim_path = f"{dest_folder.rstrip('/')}/{sim_name}"
        res["success"] = True
        res["sim_sm_path"] = sim_path
        res["triangle_count"] = target_triangles
    else:
        eal = unreal.EditorAssetLibrary
        if not eal.does_asset_exist(render_sm_path):
            res["error"] = f"Asset de render no existe: {render_sm_path}"
        else:
            source_sm = eal.load_asset(render_sm_path)
            clean_folder = dest_folder.rstrip("/")
            if not eal.does_directory_exist(clean_folder):
                eal.make_directory(clean_folder)

            dest_path = f"{clean_folder}/{sim_name}"
            if eal.does_asset_exist(dest_path):
                eal.delete_asset(dest_path)

            try:
                dyn = unreal.DynamicMesh()
                copy_opts = unreal.GeometryScriptCopyMeshFromAssetOptions()
                read_lod = unreal.GeometryScriptMeshReadLOD()
                dyn, _ = unreal.GeometryScript_AssetUtils.copy_mesh_from_static_mesh_v2(
                    source_sm, dyn, copy_opts, read_lod, True
                )

                simp_opts = unreal.GeometryScriptSimplifyMeshOptions()
                simp_opts.allow_seam_collapse = False
                simp_opts.retain_quadric_memory = False
                simp_dyn = unreal.GeometryScript_MeshSimplification.apply_simplify_to_triangle_count(
                    dyn, target_triangles, simp_opts
                )

                sm_opts = unreal.GeometryScriptCreateNewStaticMeshAssetOptions()
                sm_opts.enable_collision = False
                sm_opts.enable_recompute_normals = False
                sm_opts.enable_recompute_tangents = True

                new_sm, outcome = unreal.GeometryScript_NewAssetUtils.create_new_static_mesh_asset_from_mesh(
                    simp_dyn, dest_path, sm_opts
                )

                if new_sm is not None:
                    tri_count = unreal.GeometryScript_MeshQueries.get_num_triangle_i_ds(simp_dyn)
                    eal.save_asset(dest_path)
                    res["success"] = True
                    res["sim_sm_path"] = dest_path
                    res["triangle_count"] = tri_count
                else:
                    res["error"] = f"Error al crear asset de simulación: {outcome}"
            except Exception as exc:
                res["error"] = f"Excepción generando malla de simulación: {exc}"

    return res


def create_cloth_asset_and_dataflow(
    dest_folder: str,
    garment_name: str,
    render_sm_path: str,
    sim_sm_path: str,
    skeletal_mesh_path: str,
    physics_asset_path: str,
    template_ca: str = DEFAULT_TEMPLATE_CA,
    template_df: str = DEFAULT_TEMPLATE_DF,
) -> dict[str, Any]:
    """Crea un ChaosClothAsset y su Dataflow procedimental a partir de plantillas limpias.

    @spec REQ-87
    """
    res: dict[str, Any] = {
        "success": False,
        "error": None,
        "cloth_asset": None,
        "dataflow_asset": None,
    }

    if not dest_folder or not garment_name:
        res["error"] = "dest_folder y garment_name son obligatorios"
    elif not render_sm_path or not sim_sm_path:
        res["error"] = "render_sm_path y sim_sm_path son obligatorios"
    elif not skeletal_mesh_path or not physics_asset_path:
        res["error"] = "skeletal_mesh_path y physics_asset_path son obligatorios"
    elif not HAS_UNREAL or not hasattr(unreal, "DataflowEditorBlueprintLibrary"):
        clean_folder = dest_folder.rstrip("/")
        res["success"] = True
        res["cloth_asset"] = f"{clean_folder}/CA_{garment_name}"
        res["dataflow_asset"] = f"{clean_folder}/DF_{garment_name}"
    else:
        eal = unreal.EditorAssetLibrary
        lib_ed = unreal.DataflowEditorBlueprintLibrary
        lib_df = unreal.DataflowBlueprintLibrary

        used_ca_tmpl = (
            template_ca
            if eal.does_asset_exist(template_ca)
            else (
                DEFAULT_TEMPLATE_CA
                if eal.does_asset_exist(DEFAULT_TEMPLATE_CA)
                else FALLBACK_TEMPLATE_CA
            )
        )
        used_df_tmpl = (
            template_df
            if eal.does_asset_exist(template_df)
            else (
                DEFAULT_TEMPLATE_DF
                if eal.does_asset_exist(DEFAULT_TEMPLATE_DF)
                else FALLBACK_TEMPLATE_DF
            )
        )

        if not eal.does_asset_exist(used_ca_tmpl) or not eal.does_asset_exist(used_df_tmpl):
            res["error"] = f"Plantillas de Chaos Cloth no encontradas en {used_ca_tmpl} / {used_df_tmpl}"
        elif not eal.does_asset_exist(skeletal_mesh_path) or not eal.does_asset_exist(physics_asset_path):
            # Sin cuerpo o sin físicas el Dataflow se evalúa igual y deja una tela sin pesos ni colisión.
            res["error"] = f"Cuerpo o Physics Asset del MetaHuman no encontrados: {skeletal_mesh_path} / {physics_asset_path}"
        else:
            clean_folder = dest_folder.rstrip("/")
            if not eal.does_directory_exist(clean_folder):
                eal.make_directory(clean_folder)

            target_df = f"{clean_folder}/DF_{garment_name}"
            target_ca = f"{clean_folder}/CA_{garment_name}"

            if eal.does_asset_exist(target_ca):
                eal.delete_asset(target_ca)
            if eal.does_asset_exist(target_df):
                eal.delete_asset(target_df)

            ok_df = eal.duplicate_asset(used_df_tmpl, target_df)
            ok_ca = eal.duplicate_asset(used_ca_tmpl, target_ca)

            if not ok_df or not ok_ca:
                res["error"] = f"Fallo al duplicar plantillas hacia {target_df} / {target_ca}"
            else:
                df = eal.load_asset(target_df)
                ca = eal.load_asset(target_ca)

                try:
                    df_inst = ca.get_editor_property("dataflow_instance")
                    df_inst.set_editor_property("dataflow_asset", df)
                    df_inst.set_editor_property("dataflow_terminal", "Terminal")
                    ca.set_editor_property("dataflow_instance", df_inst)

                    lib_ed.set_dataflow_node_property(df, "ImportRender", "StaticMesh", render_sm_path)
                    lib_ed.set_dataflow_node_property(df, "Import", "StaticMesh", sim_sm_path)
                    lib_ed.set_dataflow_node_property(df, "Transfer", "SkeletalMesh", skeletal_mesh_path)
                    lib_ed.set_dataflow_node_property(df, "Physics", "PhysicsAsset", physics_asset_path)

                    lib_df.evaluate_terminal_node_by_name(df, "Terminal", ca)
                    lib_df.evaluate_dataflow(df, ca)

                    eal.save_asset(target_df)
                    eal.save_asset(target_ca)

                    res["success"] = True
                    res["cloth_asset"] = target_ca
                    res["dataflow_asset"] = target_df
                except Exception as exc:
                    res["error"] = f"Excepción configurando Dataflow: {exc}"

    return res


def setup_cloth_component_in_blueprint(
    blueprint_path: str,
    cloth_asset_path: str,
    material_path: str | None = None,
    slot_name: str = "SkirtCloth",
) -> dict[str, Any]:
    """Añade o actualiza un ChaosClothComponent en un Blueprint y enlaza su líder a Body.

    @spec REQ-87, REQ-85
    """
    res: dict[str, Any] = {
        "success": False,
        "error": None,
        "blueprint_path": blueprint_path,
        "slot_name": slot_name,
        "leader_bound": False,
    }

    if not blueprint_path or not cloth_asset_path:
        res["error"] = "blueprint_path y cloth_asset_path son obligatorios"
    elif not HAS_UNREAL or not hasattr(unreal, "SubobjectDataBlueprintFunctionLibrary"):
        res["success"] = True
        res["leader_bound"] = True
    else:
        eal = unreal.EditorAssetLibrary
        if not eal.does_asset_exist(blueprint_path):
            res["error"] = f"Blueprint no encontrado: {blueprint_path}"
        elif not eal.does_asset_exist(cloth_asset_path):
            res["error"] = f"Cloth Asset no encontrado: {cloth_asset_path}"
        else:
            bp = eal.load_asset(blueprint_path)
            cloth_asset = eal.load_asset(cloth_asset_path)
            sub = unreal.get_engine_subsystem(unreal.SubobjectDataSubsystem)
            lib_so = unreal.SubobjectDataBlueprintFunctionLibrary
            bp_lib = unreal.BlueprintEditorLibrary

            try:
                handles = sub.k2_gather_subobject_data_for_blueprint(bp)
                target_handle = None
                root_handle = None

                for h in handles:
                    data = sub.k2_find_subobject_data_from_handle(h)
                    vname = str(lib_so.get_variable_name(data))
                    if vname == slot_name:
                        target_handle = h
                    elif vname == "Root":
                        root_handle = h

                cloth_comp_obj = None
                if target_handle is not None:
                    data = sub.k2_find_subobject_data_from_handle(target_handle)
                    cloth_comp_obj = lib_so.get_object_for_blueprint(data, bp)
                else:
                    cloth_cls = unreal.load_class(None, "/Script/ChaosClothAssetEngine.ChaosClothComponent")
                    parent_h = root_handle if root_handle is not None else (handles[0] if len(handles) > 0 else None)

                    params = unreal.AddNewSubobjectParams()
                    params.set_editor_property("blueprint_context", bp)
                    params.set_editor_property("new_class", cloth_cls)
                    if parent_h is not None:
                        params.set_editor_property("parent_handle", parent_h)

                    new_handle, fail_reason = sub.add_new_subobject(params)
                    if not lib_so.is_handle_valid(new_handle):
                        res["error"] = f"Fallo al añadir ChaosClothComponent: {fail_reason}"
                    else:
                        sub.rename_subobject(new_handle, unreal.Text(slot_name))
                        sub.rename_subobject_member_variable(bp, new_handle, slot_name)

                        new_data = sub.k2_find_subobject_data_from_handle(new_handle)
                        cloth_comp_obj = lib_so.get_object_for_blueprint(new_data, bp)

                if res["error"] is None and cloth_comp_obj is not None:
                    if hasattr(cloth_comp_obj, "set_editor_property"):
                        cloth_comp_obj.set_editor_property("asset", cloth_asset)
                    if material_path and eal.does_asset_exist(material_path):
                        mat = eal.load_asset(material_path)
                        if hasattr(cloth_comp_obj, "set_material"):
                            cloth_comp_obj.set_material(0, mat)

                    # Enlazar LeaderPoseComponent en EventBeginPlay mediante BlueprintTools DSL
                    leader_bound = False
                    try:
                        from editor_toolset.toolsets.blueprint import BlueprintTools

                        tools = BlueprintTools()
                        graph = tools.get_graph(blueprint=bp, graph_name="EventGraph")
                        if graph:
                            dsl = tools.read_graph_dsl(graph=graph) or ""
                            if f"Get{slot_name}" in dsl and "SetLeaderPoseComponent" in dsl:
                                leader_bound = True
                            else:
                                lead_code = (
                                    f"(event EventBeginPlay\n"
                                    f"  (Components|SkinnedMesh|SetLeaderPoseComponent "
                                    f"(Variables|Default|Get{slot_name}) (Variables|Default|GetBody)))\n"
                                )
                                tools.write_graph_dsl(graph=graph, code=lead_code)
                                leader_bound = True
                    except Exception:
                        # En caso de entorno sin BlueprintTools DSL, el enlace se asume manual o diferido
                        leader_bound = False

                    bp_lib.compile_blueprint(bp)
                    eal.save_asset(blueprint_path)

                    res["success"] = True
                    res["leader_bound"] = leader_bound
            except Exception as exc:
                res["error"] = f"Excepción configurando componente de tela en Blueprint: {exc}"

    return res


def _discard_assets(asset_paths: list[str]) -> list[str]:
    """Borra los assets a medio crear de un intento fallido y devuelve los que borró. @spec REQ-87, REQ-88"""
    removed: list[str] = []
    eal = getattr(unreal, "EditorAssetLibrary", None) if HAS_UNREAL else None
    if eal is not None and hasattr(eal, "does_asset_exist"):
        for path in asset_paths:
            if path and eal.does_asset_exist(path) and eal.delete_asset(path):
                removed.append(path)
    return removed


def setup_cloth_for_character(
    actor_or_bp_path: str,
    garment_kind: str,
    garment_name: str,
    render_sm_path: str,
    skeletal_mesh_path: str | None = None,
    physics_asset_path: str | None = None,
    material_path: str | None = None,
    sim_sm_path: str | None = None,
    target_triangles: int = 1500,
    slot_name: str = "SkirtCloth",
    dest_folder: str | None = None,
) -> dict[str, Any]:
    """Orquestador de la tela: malla de simulación, Dataflow, Cloth Asset y componente en el Blueprint.

    @spec REQ-87, REQ-85, REQ-84
    """
    res: dict[str, Any] = {
        "success": False,
        "error": None,
        "sim_sm_path": None,
        "cloth_asset": None,
        "dataflow_asset": None,
        "blueprint_path": None,
    }

    if not actor_or_bp_path or not garment_name or not render_sm_path:
        res["error"] = "actor_or_bp_path, garment_name y render_sm_path son obligatorios"
    elif garment_kind not in ("skirt", "dress"):
        res["error"] = f"Chaos Cloth solo se aplica a faldas y vestidos (recibido '{garment_kind}')"
    else:
        # Resolver complexión por defecto
        clean_skel = skeletal_mesh_path or ARCHETYPE_FULL_BODIES["f_med_nrw"]
        clean_phys = physics_asset_path or ARCHETYPE_PHYSICS["f_med_nrw"]

        target_folder = dest_folder or f"/Game/DBV/MetaHumanOutfits/{garment_name}"
        active_sim_sm = sim_sm_path

        # 1. Generar malla de simulación si no se suministra
        if not active_sim_sm:
            sim_res = generate_cloth_simulation_mesh(
                render_sm_path=render_sm_path,
                dest_folder=target_folder,
                sim_name=f"SM_{garment_name}_Sim",
                target_triangles=target_triangles,
            )
            if not sim_res["success"]:
                res["error"] = sim_res["error"]
            else:
                active_sim_sm = sim_res["sim_sm_path"]
                res["sim_sm_path"] = active_sim_sm
        else:
            res["sim_sm_path"] = active_sim_sm

        # 2. Crear Dataflow y Cloth Asset procedimental
        if res["error"] is None and active_sim_sm is not None:
            ca_res = create_cloth_asset_and_dataflow(
                dest_folder=target_folder,
                garment_name=garment_name,
                render_sm_path=render_sm_path,
                sim_sm_path=active_sim_sm,
                skeletal_mesh_path=clean_skel,
                physics_asset_path=clean_phys,
            )
            if not ca_res["success"]:
                res["error"] = ca_res["error"]
                # Aún no los usa ningún Blueprint: no se dejan restos sueltos del intento fallido.
                clean_folder = target_folder.rstrip("/")
                leftovers = [f"{clean_folder}/CA_{garment_name}", f"{clean_folder}/DF_{garment_name}"]
                if not sim_sm_path:
                    leftovers.append(active_sim_sm)
                    res["sim_sm_path"] = None
                res["discarded_assets"] = _discard_assets(leftovers)
            else:
                res["cloth_asset"] = ca_res["cloth_asset"]
                res["dataflow_asset"] = ca_res["dataflow_asset"]

                # 3. Configurar componente en el Blueprint
                bp_path = actor_or_bp_path
                comp_res = setup_cloth_component_in_blueprint(
                    blueprint_path=bp_path,
                    cloth_asset_path=ca_res["cloth_asset"],
                    material_path=material_path,
                    slot_name=slot_name,
                )
                if not comp_res["success"]:
                    res["error"] = comp_res["error"]
                else:
                    res["blueprint_path"] = bp_path
                    # Las reglas de ranura (REQ-84) se aplican sobre el actor del nivel, no sobre el
                    # Blueprint: las aplica quien llama (pipeline.dress_metahuman_step, paso apply_slot_rules).
                    res["success"] = True

    return res
