"""Spike en el editor: ajusta prendas de Tripo conservando su malla y las equipa."""
import importlib
import sys
import time

import numpy as np
import unreal

SP = __import__("pathlib").Path(__file__).resolve().parent.as_posix()
if SP not in sys.path:
    sys.path.insert(0, SP)
import fitlab  # noqa: E402
import fit2  # noqa: E402

importlib.reload(fitlab)
importlib.reload(fit2)

Q = unreal.GeometryScript_MeshQueries
BW = unreal.GeometryScript_BoneWeights
AU = unreal.GeometryScript_AssetUtils
FOLDER = "/Game/DBV/MetaHumanOutfits/Fable"


def _dyn_arrays(dyn):
    n, nt = dyn.get_vertex_count(), dyn.get_triangle_count()
    pos = np.zeros((n, 3))
    for i in range(n):
        p = Q.get_vertex_position(dyn, i)[0]
        pos[i] = (p.x, p.y, p.z)
    tris = np.zeros((nt, 3), np.int64)
    for t in range(nt):
        idx = Q.get_triangle_indices(dyn, t)[0]
        tris[t] = (idx.x, idx.y, idx.z)
    return pos, tris


def body_model(skm):
    dyn = unreal.DynamicMesh()
    dyn = AU.copy_mesh_from_skeletal_mesh(skm, dyn, unreal.GeometryScriptCopyMeshFromAssetOptions(), unreal.GeometryScriptMeshReadLOD())[0]
    pos, tris = _dyn_arrays(dyn)
    info = BW.get_all_bones_info(dyn)[1]
    names = [str(b.name) for b in info]
    bpos = np.array([(b.world_transform.translation.x, b.world_transform.translation.y, b.world_transform.translation.z) for b in info])
    K = 12
    wi = np.zeros((len(pos), K), np.int64)
    ww = np.zeros((len(pos), K))
    for i in range(len(pos)):
        for k, w in enumerate(list(BW.get_vertex_bone_weights(dyn, i)[1])[:K]):
            wi[i, k], ww[i, k] = w.bone_index, w.weight
    B = fitlab.Body({"pos": pos, "tris": tris, "names": np.array(names), "wi": wi, "ww": ww, "bpos": bpos})
    return B, dyn


def garment_model(sm):
    dyn = unreal.DynamicMesh()
    dyn = AU.copy_mesh_from_static_mesh_v2(sm, dyn, unreal.GeometryScriptCopyMeshFromAssetOptions(), unreal.GeometryScriptMeshReadLOD(), True)[0]
    pos, tris = _dyn_arrays(dyn)
    nl = unreal.GeometryScript_Normals.get_mesh_per_vertex_normals(dyn)[1]
    nrm = np.array([(v.x, v.y, v.z) for v in unreal.GeometryScript_List.convert_vector_list_to_array(nl)])
    return dyn, pos, tris, nrm


def skin_weights(B, pos, tris, sleeveless=True):
    """Pesos por vértice de la prenda: los de la piel más cercana, suavizados sobre la malla soldada."""
    mask = (B.arm_w < 0.5) & ~(np.abs(B.pos[:, 0]) > abs(B.bone("upperarm_l")[0]) + 3.0) if sleeveless else np.ones(len(B.pos), bool)
    pts, _, owner = B.surface(mask)
    lo, hi = pos.min(0) - 10, pos.max(0) + 10
    m = ((pts > lo) & (pts < hi)).all(1)
    idx, _ = fitlab.nearest(pos, pts[m])
    src = owner[m][idx]
    nb = len(B.names)
    inv = fitlab.weld_map(pos)
    ng = inv.max() + 1
    dense = np.zeros((ng, nb))
    rows = np.repeat(inv, B.wi.shape[1])
    np.add.at(dense, (rows, B.wi[src].ravel()), B.ww[src].ravel())
    edges = fitlab.adjacency(tris, inv)
    dense = fitlab.laplacian_smooth(dense, edges, ng, 8, keep=0.5)
    out = dense[inv]
    top = np.argsort(-out, axis=1)[:, :8]
    w = np.take_along_axis(out, top, axis=1)
    w /= w.sum(1, keepdims=True) + 1e-12
    return top, w


def welded_normals(pos, tris, ref):
    """Normales suaves a través de las costuras de las islas de UV, con el sentido de las originales."""
    inv = fitlab.weld_map(pos)
    a, b, c = (pos[tris[:, k]] for k in range(3))
    fn = np.cross(b - a, c - a)
    g = np.zeros((inv.max() + 1, 3))
    for k in range(3):
        np.add.at(g, inv[tris[:, k]], fn)
    n = g[inv]
    n /= np.linalg.norm(n, axis=1, keepdims=True) + 1e-12
    flip = (n * ref).sum(1) < 0
    # el sentido global lo decide la mayoría (malla orientada de forma coherente)
    if flip.mean() > 0.5:
        n = -n
    return n


def build(B, body_dyn, full_skm, sm_path, kind, name, collider):
    t0 = time.time()
    sm = unreal.load_asset(sm_path)
    dyn, pos, tris, nrm0 = garment_model(sm)
    if kind == "pants":
        new_pos, kept, info = fit2.fit_pants(pos, tris, B, collider)
    else:
        new_pos, kept, info = fit2.fit_top(pos, tris, B, collider)
    cap = info.pop("cap_mask")
    info["caps_removed"] = int(cap.sum())
    # el giro de normalise() también gira las normales de referencia
    ref = nrm0.copy()
    if info["rot90"]:
        ref = np.stack([-ref[:, 1], ref[:, 0], ref[:, 2]], 1)
    if info["flip180"]:
        ref = ref * np.array([-1.0, -1.0, 1.0])
    normals = welded_normals(new_pos, kept, ref)
    wi, ww = skin_weights(B, new_pos, kept)
    vec = unreal.GeometryScript_List.convert_array_to_vector_list([unreal.Vector(float(p[0]), float(p[1]), float(p[2])) for p in new_pos])
    dyn = unreal.GeometryScript_MeshEdits.set_all_mesh_vertex_positions(dyn, vec)
    nvec = unreal.GeometryScript_List.convert_array_to_vector_list([unreal.Vector(float(p[0]), float(p[1]), float(p[2])) for p in normals])
    dyn = unreal.GeometryScript_Normals.set_per_vertex_normals(dyn)
    dyn = unreal.GeometryScript_Normals.set_mesh_per_vertex_normals(dyn, nvec)
    if cap.any():
        ids = unreal.GeometryScript_List.convert_array_to_index_list([int(i) for i in np.nonzero(cap)[0]], unreal.GeometryScriptIndexType.TRIANGLE)
        r = unreal.GeometryScript_MeshEdits.delete_triangles_from_mesh(dyn, ids)
        dyn = r[0] if isinstance(r, tuple) else r
    dyn = BW.copy_bones_from_mesh(body_dyn, dyn, unreal.GeometryScriptCopyBonesFromMeshOptions())
    r = BW.mesh_create_bone_weights(dyn, True)
    dyn = r[0] if isinstance(r, tuple) else r
    for i in range(len(new_pos)):
        arr = [unreal.GeometryScriptBoneWeight(int(b), float(w)) for b, w in zip(wi[i], ww[i]) if w > 1e-4]
        BW.set_vertex_bone_weights(dyn, i, arr)
    # malla esquelética: copia del cuerpo con la prenda como LOD0 (mismo orden de huesos)
    eal = unreal.EditorAssetLibrary
    if not eal.does_directory_exist(FOLDER):
        eal.make_directory(FOLDER)
    dest = f"{FOLDER}/{name}"
    skm = unreal.load_asset(dest) if eal.does_asset_exist(dest) else unreal.AssetToolsHelpers.get_asset_tools().duplicate_asset(name, FOLDER, full_skm)
    material = sm.static_materials[0].material_interface
    opt = unreal.GeometryScriptCopyMeshToAssetOptions()
    opt.replace_materials = True
    opt.new_materials = [material]
    opt.enable_recompute_normals = False
    opt.enable_recompute_tangents = True
    AU.copy_mesh_to_skeletal_mesh(dyn, skm, opt, unreal.GeometryScriptMeshWriteLOD())
    eal.save_loaded_asset(skm, False)
    info["asset"] = dest
    info["seconds"] = round(time.time() - t0, 1)
    return skm, new_pos, kept, info, material


def equip(actor_label, slot, skm, full_skm, material):
    actor = next(a for a in unreal.get_editor_subsystem(unreal.EditorActorSubsystem).get_all_level_actors() if a.get_actor_label() == actor_label)
    comps = {c.get_name(): c for c in actor.get_components_by_class(unreal.SkeletalMeshComponent)}
    body, comp = comps["Body"], comps[slot]
    with unreal.ScopedEditorTransaction("DBV: equip fitted garment"):
        actor.modify()
        if body.get_skeletal_mesh_asset().get_path_name() != full_skm.get_path_name():
            skin = body.get_material(0)
            body.modify()
            body.set_skeletal_mesh_asset(full_skm)
            if skin is not None:
                body.set_material(0, skin)
        comp.modify()
        comp.set_skeletal_mesh_asset(skm)
        comp.set_editor_property("override_materials", [])
        comp.set_editor_property("visible", True)
        comp.set_material(0, material)
        comp.set_leader_pose_component(body, False)


def dress(actor_label, shorts_sm, top_sm):
    full_skm = unreal.load_asset("/Game/MetaHumans/Common/Common/m_tal_nrw_body_preview")
    B, body_dyn = body_model(full_skm)
    col = fit2.body_collider(B)
    report = {}
    skm_s, ps, ts, report["pants"], mat_s = build(B, body_dyn, full_skm, shorts_sm, "pants", "SK_Shorts_Fable", col)
    gp, gn = fit2.garment_collider(ps, ts)
    col2 = fit2.Collider(np.concatenate([col.pts, gp]), np.concatenate([col.nrm, gn]))
    skm_t, pt, tt, report["top"], mat_t = build(B, body_dyn, full_skm, top_sm, "top", "SK_Tirantes_Fable", col2)
    equip(actor_label, "Legs", skm_s, full_skm, mat_s)
    equip(actor_label, "Torso", skm_t, full_skm, mat_t)
    return report


def capture(actor_label, out_dir, views=(("front", 0.0), ("side", 90.0), ("back", 180.0), ("q", 35.0)), height=105.0, distance=260.0, fov=30.0, size=900):
    """Capturas con una luz frontal temporal por vista, para juzgar la prenda y no la hora del día."""
    import math
    import os
    os.makedirs(out_dir, exist_ok=True)
    actors = unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
    actor = next(a for a in actors.get_all_level_actors() if a.get_actor_label() == actor_label)
    world = unreal.get_editor_subsystem(unreal.UnrealEditorSubsystem).get_editor_world()
    target = actor.get_actor_location()
    look = unreal.Vector(target.x, target.y, target.z + height)
    rt = unreal.RenderingLibrary.create_render_target2d(world, size, size, unreal.TextureRenderTargetFormat.RTF_RGBA8)
    cap = actors.spawn_actor_from_class(unreal.SceneCapture2D, target)
    light = actors.spawn_actor_from_class(unreal.DirectionalLight, target)
    files = []
    try:
        lc = light.get_component_by_class(unreal.DirectionalLightComponent)
        lc.set_mobility(unreal.ComponentMobility.MOVABLE)
        lc.set_intensity(4.0)
        lc.set_cast_shadows(False)
        comp = cap.capture_component2d
        comp.texture_target = rt
        comp.capture_source = unreal.SceneCaptureSource.SCS_FINAL_COLOR_LDR
        comp.capture_every_frame = False
        comp.fov_angle = fov
        comp.primitive_render_mode = unreal.SceneCapturePrimitiveRenderMode.PRM_USE_SHOW_ONLY_LIST
        comp.show_only_actor_components(actor)
        base = actor.get_actor_rotation().yaw + 90.0
        for name, off in views:
            ang = math.radians(base + off)
            cam = unreal.Vector(target.x + distance * math.cos(ang), target.y + distance * math.sin(ang), look.z)
            rot = unreal.MathLibrary.find_look_at_rotation(cam, look)
            cap.set_actor_location_and_rotation(cam, rot, False, False)
            light.set_actor_rotation(unreal.Rotator(rot.roll, rot.pitch - 15.0, rot.yaw + 20.0), False)
            comp.capture_scene()
            unreal.RenderingLibrary.export_render_target(world, rt, out_dir, name + ".png")
            files.append(out_dir + "/" + name + ".png")
    finally:
        actors.destroy_actor(cap)
        actors.destroy_actor(light)
    return files
