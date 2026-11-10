from run_ue import run_in_unreal
import json

code = """
import unreal, json, math, time

actor_sub = unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
pablo = [a for a in actor_sub.get_all_level_actors() if a.get_actor_label() == "BP_Pablo"][0]
comps = {c.get_name(): c for c in pablo.get_components_by_class(unreal.SkeletalMeshComponent)}
body = comps.get("Body")
torso = comps.get("Torso")
legs = comps.get("Legs")
feet = comps.get("Feet")
body_skm = body.get_skeletal_mesh_asset()
skeleton = body_skm.get_editor_property("skeleton")

body_origin_y = float(body_skm.get_bounds().origin.y) # should be ~7.6

eal = unreal.EditorAssetLibrary
mq = unreal.GeometryScript_MeshQueries
me = unreal.GeometryScript_MeshEdits
xform = unreal.GeometryScript_MeshTransforms
asset_utils = unreal.GeometryScript_AssetUtils
bw = unreal.GeometryScript_BoneWeights
repair = getattr(unreal, "GeometryScript_MeshRepair", None)
new_assets = unreal.GeometryScript_NewAssetUtils

folder = "/Game/DBV/MetaHumanOutfits"
mat = eal.load_asset(f"{folder}/model/Materials/tripo_material_80a26f15-2ff2-4fd6-9b54-57fc598949ed")
if mat:
    bpo = mat.get_editor_property("base_property_overrides")
    bpo.set_editor_property("override_two_sided", True)
    bpo.set_editor_property("two_sided", True)
    mat.set_editor_property("base_property_overrides", bpo)
    eal.save_loaded_asset(mat, False)

body_dyn = unreal.DynamicMesh()
body_dyn, _ = asset_utils.copy_mesh_from_skeletal_mesh(
    body_skm, body_dyn, unreal.GeometryScriptCopyMeshFromAssetOptions(), unreal.GeometryScriptMeshReadLOD()
)

transfer_opts = unreal.GeometryScriptTransferBoneWeightsOptions()
transfer_opts.transfer_method = unreal.TransferBoneWeightsMethod.INPAINT_WEIGHTS
transfer_opts.num_smoothing_iterations = 6
transfer_opts.smoothing_strength = 0.5

skm_opts = unreal.GeometryScriptCreateNewSkeletalMeshAssetOptions()
skm_opts.enable_recompute_normals = False
skm_opts.enable_recompute_tangents = True
# Do not set skm_opts.materials directly to avoid TMap::FindChecked assertion crashes in UE

# =========================================================================
# 1. JACKET WITH S103 WELDING & ALIGNED TO BODY Y=7.6
# =========================================================================
j_sm = eal.load_asset(f"{folder}/SM_Chaqueta_Pablo_v2")
dyn_j = unreal.DynamicMesh()
dyn_j, _ = asset_utils.copy_mesh_from_static_mesh_v2(j_sm, dyn_j, unreal.GeometryScriptCopyMeshFromAssetOptions(), unreal.GeometryScriptMeshReadLOD(), True)

# S103: Weld edges (safely without filling anatomical openings)
from dbv_unreal_ai.outfit import fit
dyn_j = fit.weld_top_dynamic_mesh(dyn_j, tolerance=0.05, fill_holes=False)

bb_j0 = mq.get_mesh_bounding_box(dyn_j)
c_j0 = (bb_j0.max + bb_j0.min) * 0.5
# Scale: chest width X=1.38, depth Y=1.45, height Z=1.20
dyn_j = xform.scale_mesh(dyn_j, unreal.Vector(1.38, 1.45, 1.20), c_j0)

theta_down = math.radians(53.6) # Exact MetaHuman A-pose arm slope
theta_fwd = math.radians(10.0)

shoulder_x = 16.5
blend_x = 21.5
pivot_z = (38.0 - float(c_j0.z)) * 1.20 + float(c_j0.z)
pivot_y = 0.0

vc_j = int(dyn_j.get_vertex_count())
for i in range(vc_j):
    v, ok = mq.get_vertex_position(dyn_j, i)
    if not ok:
        continue
    x, y, z = float(v.x), float(v.y), float(v.z)
    ax = abs(x)
    # S103 fix: do NOT cut off lower sleeves; rotate full arm down to cuffs
    if ax > shoulder_x and z > (pivot_z - 45.0):
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

# Position jacket:
# Collar top at Z = 144.0 (snug around neck)
# Center X = 0.0, Center Y = body_origin_y (aligned with Pablo's body in depth!)
bb_j2 = mq.get_mesh_bounding_box(dyn_j)
target_top_z = 144.0
dz_j = target_top_z - float(bb_j2.max.z)
dx_j = 0.0 - float((bb_j2.max.x + bb_j2.min.x) * 0.5)
dy_j = body_origin_y - float((bb_j2.max.y + bb_j2.min.y) * 0.5)
dyn_j = xform.translate_mesh(dyn_j, unreal.Vector(dx_j, dy_j, dz_j))

dyn_j = bw.transfer_bone_weights_from_mesh(body_dyn, dyn_j, transfer_opts)

sk_jacket_path = f"{folder}/SK_Chaqueta_Pablo_v2"
sk_jacket, _ = new_assets.create_new_skeletal_mesh_asset_from_mesh(dyn_j, skeleton, sk_jacket_path, skm_opts)
if sk_jacket:
    if mat:
        sk_jacket.set_material(0, mat)
    # S103: Generate 4 LOD levels
    sub = unreal.get_editor_subsystem(unreal.SkeletalMeshEditorSubsystem)
    if sub:
        sub.regenerate_lod(sk_jacket, new_lod_count=4)
    eal.save_loaded_asset(sk_jacket, False)

# =========================================================================
# 2. PANTS WITH S103 WELDING & ALIGNED TO BODY Y=7.6
# =========================================================================
p_sm = eal.load_asset(f"{folder}/SM_Pantalones_Pablo_v2")
dyn_p = unreal.DynamicMesh()
dyn_p, _ = asset_utils.copy_mesh_from_static_mesh_v2(p_sm, dyn_p, unreal.GeometryScriptCopyMeshFromAssetOptions(), unreal.GeometryScriptMeshReadLOD(), True)

# S103: Weld edges (safely without filling anatomical openings)
dyn_p = fit.weld_top_dynamic_mesh(dyn_p, tolerance=0.05, fill_holes=False)

# Scale: X=1.48 (wider thighs), Y=1.52 (fuller depth to cover glutes and quads), Z=1.51
bb_p0 = mq.get_mesh_bounding_box(dyn_p)
c_p0 = (bb_p0.max + bb_p0.min) * 0.5
dyn_p = xform.scale_mesh(dyn_p, unreal.Vector(1.48, 1.52, 1.51), c_p0)

# Close inner thigh gap in crotch region (Z between -10 and +8 in local mesh)
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

# Position pants:
# Top waistband at Z = 104.5, Center X = 0.0, Center Y = body_origin_y
bb_p2 = mq.get_mesh_bounding_box(dyn_p)
dz_p = 104.5 - float(bb_p2.max.z)
dx_p = 0.0 - float((bb_p2.max.x + bb_p2.min.x) * 0.5)
dy_p = body_origin_y - float((bb_p2.max.y + bb_p2.min.y) * 0.5)
dyn_p = xform.translate_mesh(dyn_p, unreal.Vector(dx_p, dy_p, dz_p))

dyn_p = bw.transfer_bone_weights_from_mesh(body_dyn, dyn_p, transfer_opts)

sk_pants_path = f"{folder}/SK_Pantalones_Pablo_v2"
sk_pants, _ = new_assets.create_new_skeletal_mesh_asset_from_mesh(dyn_p, skeleton, sk_pants_path, skm_opts)
if sk_pants:
    if mat:
        sk_pants.set_material(0, mat)
    sub = unreal.get_editor_subsystem(unreal.SkeletalMeshEditorSubsystem)
    if sub:
        sub.regenerate_lod(sk_pants, new_lod_count=4)
    eal.save_loaded_asset(sk_pants, False)

# =========================================================================
# 3. EQUIP ON BP_PABLO
# =========================================================================
torso.set_skeletal_mesh_asset(sk_jacket)
if mat:
    torso.set_material(0, mat)
torso.set_leader_pose_component(body, force_update=True)

legs.set_skeletal_mesh_asset(sk_pants)
if mat:
    legs.set_material(0, mat)
legs.set_leader_pose_component(body, force_update=True)

feet.set_skeletal_mesh_asset(None)

# S103: Apply two sided materials to actor
from dbv_unreal_ai.outfit import fit
mats_modified = fit.apply_two_sided_materials(pablo)

result = json.dumps({
    "success": True,
    "body_origin_y": body_origin_y,
    "jacket_lods": sub.get_lod_count(sk_jacket) if sub and sk_jacket else None,
    "pants_lods": sub.get_lod_count(sk_pants) if sub and sk_pants else None,
    "mats_modified": mats_modified
}, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

