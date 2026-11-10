import unreal
import json
import math
import time

def dress_pablo():
    sub_actor = unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
    actors = sub_actor.get_all_level_actors()
    pablo = [a for a in actors if a.get_actor_label() == "BP_Pablo"][0]
    
    body = [c for c in pablo.get_components_by_class(unreal.SkeletalMeshComponent) if c.get_name() == "Body"][0]
    torso = [c for c in pablo.get_components_by_class(unreal.SkeletalMeshComponent) if c.get_name() == "Torso"][0]
    legs = [c for c in pablo.get_components_by_class(unreal.SkeletalMeshComponent) if c.get_name() == "Legs"][0]
    feet = [c for c in pablo.get_components_by_class(unreal.SkeletalMeshComponent) if c.get_name() == "Feet"][0]
    
    # Unhook existing meshes from components first so assets are not locked in rendering
    torso.set_skeletal_mesh_asset(None)
    legs.set_skeletal_mesh_asset(None)
    
    body_skm = body.get_skinned_asset()
    skeleton = body_skm.get_editor_property("skeleton")
    
    eal = unreal.EditorAssetLibrary
    mq = unreal.GeometryScript_MeshQueries
    me = unreal.GeometryScript_MeshEdits
    xform = unreal.GeometryScript_MeshTransforms
    asset_utils = unreal.GeometryScript_AssetUtils
    bw = unreal.GeometryScript_BoneWeights
    new_assets = unreal.GeometryScript_NewAssetUtils
    repair = getattr(unreal, "GeometryScript_MeshRepair", None)
    
    folder = "/Game/DBV/MetaHumanOutfits"
    mat = eal.load_asset(f"{folder}/model/Materials/tripo_material_80a26f15-2ff2-4fd6-9b54-57fc598949ed")
    
    body_dyn = unreal.DynamicMesh()
    body_dyn, _ = asset_utils.copy_mesh_from_skeletal_mesh(
        body_skm, body_dyn, unreal.GeometryScriptCopyMeshFromAssetOptions(), unreal.GeometryScriptMeshReadLOD()
    )
    
    transfer_opts = unreal.GeometryScriptTransferBoneWeightsOptions()
    transfer_opts.transfer_method = unreal.TransferBoneWeightsMethod.INPAINT_WEIGHTS
    transfer_opts.num_smoothing_iterations = 5
    transfer_opts.smoothing_strength = 0.5
    
    skm_opts = unreal.GeometryScriptCreateNewSkeletalMeshAssetOptions()
    skm_opts.enable_recompute_normals = False
    skm_opts.enable_recompute_tangents = True
    
    # -------------------------------------------------------------------------
    # 1. JACKET: FULL TORSO COVERAGE + SLEEVES CONFORMATION
    # -------------------------------------------------------------------------
    j_sm = eal.load_asset(f"{folder}/SM_Chaqueta_Pablo_v2")
    dyn_j = unreal.DynamicMesh()
    dyn_j, _ = asset_utils.copy_mesh_from_static_mesh_v2(
        j_sm, dyn_j, unreal.GeometryScriptCopyMeshFromAssetOptions(), unreal.GeometryScriptMeshReadLOD(), True
    )
    
    # Scale jacket:
    # Width X=1.38, Depth Y=2.25 (to cover chest Y=+28 and back Y=-13), Height Z=1.52 (from collar 148 to hem 92)
    bb_j0 = mq.get_mesh_bounding_box(dyn_j)
    c_j0 = (bb_j0.max + bb_j0.min) * 0.5
    dyn_j = xform.scale_mesh(dyn_j, unreal.Vector(1.38, 2.25, 1.52), c_j0)
    
    # Rotate arms down (53.6 deg) and slightly forward (10 deg)
    theta_down = math.radians(53.6)
    theta_fwd = math.radians(10.0)
    shoulder_x = 16.5
    blend_x = 21.5
    pivot_z = (38.0 - float(c_j0.z)) * 1.52 + float(c_j0.z)
    pivot_y = 0.0
    
    vc_j = int(dyn_j.get_vertex_count())
    for i in range(vc_j):
        v, ok = mq.get_vertex_position(dyn_j, i)
        if not ok:
            continue
        x, y, z = float(v.x), float(v.y), float(v.z)
        ax = abs(x)
        if ax > shoulder_x:
            t = min(1.0, max(0.0, (ax - shoulder_x) / (blend_x - shoulder_x)))
            w = t * t * (3.0 - 2.0 * t)
            
            th_d = theta_down * w
            cd, sd = math.cos(th_d), math.sin(th_d)
            
            th_f = theta_fwd * w
            cf, sf = math.cos(th_f), math.sin(th_f)
            
            sleeve_scale = 1.0 + 0.12 * w
            
            if x > 0: # Left arm
                dx = (x - shoulder_x) * sleeve_scale
                dz = (z - pivot_z) * sleeve_scale
                dy = (y - pivot_y) * sleeve_scale
                nx1 = shoulder_x + dx * cd + dz * sd
                nz1 = pivot_z - dx * sd + dz * cd
                nx = shoulder_x + (nx1 - shoulder_x) * cf - dy * sf
                ny = pivot_y + (nx1 - shoulder_x) * sf + dy * cf
                nz = nz1
            else: # Right arm
                dx = (x - (-shoulder_x)) * sleeve_scale
                dz = (z - pivot_z) * sleeve_scale
                dy = (y - pivot_y) * sleeve_scale
                nx1 = -shoulder_x + dx * cd - dz * sd
                nz1 = pivot_z + dx * sd + dz * cd
                nx = -shoulder_x + (nx1 - (-shoulder_x)) * cf + dy * sf
                ny = pivot_y - (nx1 - (-shoulder_x)) * sf + dy * cf
                nz = nz1
            me.set_vertex_position(dyn_j, i, unreal.Vector(nx, ny, nz), i < vc_j - 1)
            
    # Position jacket: collar top at Z=148.0, center X=0, center Y=+7.6 (Pablo body center Y)
    bb_j2 = mq.get_mesh_bounding_box(dyn_j)
    dz_j = 148.0 - float(bb_j2.max.z)
    dx_j = 0.0 - float((bb_j2.max.x + bb_j2.min.x) * 0.5)
    dy_j = 7.6 - float((bb_j2.max.y + bb_j2.min.y) * 0.5)
    dyn_j = xform.translate_mesh(dyn_j, unreal.Vector(dx_j, dy_j, dz_j))
    
    # Weld top edges (REQ-81)
    if repair and hasattr(repair, "weld_mesh_edges"):
        wopts = unreal.GeometryScriptWeldEdgesOptions()
        wopts.tolerance = 0.05
        dyn_j = repair.weld_mesh_edges(dyn_j, wopts)
        
    dyn_j = bw.transfer_bone_weights_from_mesh(body_dyn, dyn_j, transfer_opts)
    
    sk_jacket_path = f"{folder}/SK_Chaqueta_Pablo_S103"
    if eal.does_asset_exist(sk_jacket_path):
        eal.delete_asset(sk_jacket_path)
    sk_jacket, _ = new_assets.create_new_skeletal_mesh_asset_from_mesh(dyn_j, skeleton, sk_jacket_path, skm_opts)
    if sk_jacket:
        if mat:
            sk_jacket.set_material(0, mat)
        sub_skm = unreal.get_editor_subsystem(unreal.SkeletalMeshEditorSubsystem)
        if sub_skm:
            sub_skm.regenerate_lod(sk_jacket, new_lod_count=4)
        eal.save_loaded_asset(sk_jacket, False)
        
    # -------------------------------------------------------------------------
    # 2. PANTS: FULL COVERAGE (GLUTES & QUADS) + Y=7.6 ALIGNMENT
    # -------------------------------------------------------------------------
    p_sm = eal.load_asset(f"{folder}/SM_Pantalones_Pablo_v2")
    dyn_p = unreal.DynamicMesh()
    dyn_p, _ = asset_utils.copy_mesh_from_static_mesh_v2(
        p_sm, dyn_p, unreal.GeometryScriptCopyMeshFromAssetOptions(), unreal.GeometryScriptMeshReadLOD(), True
    )
    
    # Scale pants: Width X=1.40, Depth Y=2.10 (to cover quads Y=+25 and glutes Y=-13), Height Z=1.45 (104 down to 10)
    bb_p0 = mq.get_mesh_bounding_box(dyn_p)
    c_p0 = (bb_p0.max + bb_p0.min) * 0.5
    dyn_p = xform.scale_mesh(dyn_p, unreal.Vector(1.40, 2.10, 1.45), c_p0)
    
    # Close crotch seam gap
    vc_p = int(dyn_p.get_vertex_count())
    for i in range(vc_p):
        v, ok = mq.get_vertex_position(dyn_p, i)
        if not ok:
            continue
        x, y, z = float(v.x), float(v.y), float(v.z)
        ax = abs(x)
        if ax < 9.0 and -10.0 <= z <= 8.0:
            factor = (1.0 - ax / 9.0) * 1.8
            nx = (x - factor) if x > 0 else (x + factor)
            me.set_vertex_position(dyn_p, i, unreal.Vector(nx, y, z), i < vc_p - 1)
            
    # Position pants: Waistband at Z=104.0, Center X=0, Center Y=+6.5
    bb_p2 = mq.get_mesh_bounding_box(dyn_p)
    dz_p = 104.0 - float(bb_p2.max.z)
    dx_p = 0.0 - float((bb_p2.max.x + bb_p2.min.x) * 0.5)
    dy_p = 6.5 - float((bb_p2.max.y + bb_p2.min.y) * 0.5)
    dyn_p = xform.translate_mesh(dyn_p, unreal.Vector(dx_p, dy_p, dz_p))
    
    if repair and hasattr(repair, "weld_mesh_edges"):
        wopts = unreal.GeometryScriptWeldEdgesOptions()
        wopts.tolerance = 0.05
        dyn_p = repair.weld_mesh_edges(dyn_p, wopts)
        
    dyn_p = bw.transfer_bone_weights_from_mesh(body_dyn, dyn_p, transfer_opts)
    
    sk_pants_path = f"{folder}/SK_Pantalones_Pablo_S103"
    if eal.does_asset_exist(sk_pants_path):
        eal.delete_asset(sk_pants_path)
    sk_pants, _ = new_assets.create_new_skeletal_mesh_asset_from_mesh(dyn_p, skeleton, sk_pants_path, skm_opts)
    if sk_pants:
        if mat:
            sk_pants.set_material(0, mat)
        sub_skm = unreal.get_editor_subsystem(unreal.SkeletalMeshEditorSubsystem)
        if sub_skm:
            sub_skm.regenerate_lod(sk_pants, new_lod_count=4)
        eal.save_loaded_asset(sk_pants, False)
        
    # -------------------------------------------------------------------------
    # 3. EQUIP ON BP_PABLO
    # -------------------------------------------------------------------------
    torso.set_skeletal_mesh_asset(sk_jacket)
    if mat:
        torso.set_material(0, mat)
    torso.set_leader_pose_component(body, force_update=True)
    
    legs.set_skeletal_mesh_asset(sk_pants)
    if mat:
        legs.set_material(0, mat)
    legs.set_leader_pose_component(body, force_update=True)
    
    feet.set_skeletal_mesh_asset(None)
    
    from dbv_unreal_ai.outfit import fit
    mats_mod = fit.apply_two_sided_materials(pablo)
    
    sub_skm = unreal.get_editor_subsystem(unreal.SkeletalMeshEditorSubsystem)
    return {
        "success": True,
        "jacket_asset": sk_jacket_path,
        "pants_asset": sk_pants_path,
        "jacket_lods": sub_skm.get_lod_count(sk_jacket) if sub_skm and sk_jacket else 0,
        "pants_lods": sub_skm.get_lod_count(sk_pants) if sub_skm and sk_pants else 0,
        "mats_modified": mats_mod
    }

result = json.dumps(dress_pablo(), indent=2)

