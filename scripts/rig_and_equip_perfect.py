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

eal = unreal.EditorAssetLibrary
mq = unreal.GeometryScript_MeshQueries
me = unreal.GeometryScript_MeshEdits
xform = unreal.GeometryScript_MeshTransforms
asset_utils = unreal.GeometryScript_AssetUtils
bw = unreal.GeometryScript_BoneWeights
new_assets = unreal.GeometryScript_NewAssetUtils

folder = "/Game/DBV/MetaHumanOutfits"
mat = eal.load_asset(f"{folder}/model/Materials/tripo_material_80a26f15-2ff2-4fd6-9b54-57fc598949ed")

# 1. LOAD BODY DYNAMIC MESH FOR WEIGHT TRANSFER
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
if mat:
    skm_opts.materials = {"tripo_material_80a26f15-2ff2-4fd6-9b54-57fc598949ed": mat}

# 2. PREPARE JACKET
j_sm = eal.load_asset(f"{folder}/SM_Chaqueta_Pablo_v2")
dyn_j = unreal.DynamicMesh()
dyn_j, _ = asset_utils.copy_mesh_from_static_mesh_v2(j_sm, dyn_j, unreal.GeometryScriptCopyMeshFromAssetOptions(), unreal.GeometryScriptMeshReadLOD(), True)

# Rotate arms down by 46 deg and forward by 12 deg
theta_down = math.radians(46.0)
theta_fwd = math.radians(12.0)
c_down, s_down = math.cos(theta_down), math.sin(theta_down)
c_fwd, s_fwd = math.cos(theta_fwd), math.sin(theta_fwd)

shoulder_x = 13.0
blend_x = 17.5
pivot_z = 39.5
pivot_y = 0.0

vc_j = int(dyn_j.get_vertex_count())
for i in range(vc_j):
    v, ok = mq.get_vertex_position(dyn_j, i)
    if not ok:
        continue
    x, y, z = float(v.x), float(v.y), float(v.z)
    ax = abs(x)
    if ax > shoulder_x and z > 22.0:
        t = min(1.0, max(0.0, (ax - shoulder_x) / (blend_x - shoulder_x)))
        w = t * t * (3.0 - 2.0 * t)
        
        th_d = theta_down * w
        cd, sd = math.cos(th_d), math.sin(th_d)
        
        th_f = theta_fwd * w
        cf, sf = math.cos(th_f), math.sin(th_f)
        
        if x > 0: # Left arm
            dx = x - shoulder_x
            dz = z - pivot_z
            dy = y - pivot_y
            # Rotate down around Y
            nx1 = shoulder_x + dx * cd + dz * sd
            nz1 = pivot_z - dx * sd + dz * cd
            # Rotate forward around Z
            nx = shoulder_x + (nx1 - shoulder_x) * cf - dy * sf
            ny = pivot_y + (nx1 - shoulder_x) * sf + dy * cf
            nz = nz1
        else: # Right arm
            dx = x - (-shoulder_x)
            dz = z - pivot_z
            dy = y - pivot_y
            # Rotate down around Y
            nx1 = -shoulder_x + dx * cd - dz * sd
            nz1 = pivot_z + dx * sd + dz * cd
            # Rotate forward around Z
            nx = -shoulder_x + (nx1 - (-shoulder_x)) * cf + dy * sf
            ny = pivot_y - (nx1 - (-shoulder_x)) * sf + dy * cf
            nz = nz1
        me.set_vertex_position(dyn_j, i, unreal.Vector(nx, ny, nz), i < vc_j - 1)

# Now scale and translate jacket to match Pablo's torso
# Pablo's torso: neck at Z=150.0, waist at Z=104.0 (height = 46.0cm)
# Current jacket Z: from 13.5 to 50.01 (height = 36.5cm)
# Scale: X=1.22, Y=1.25, Z=1.26
bb_j = mq.get_mesh_bounding_box(dyn_j)
c_j = (bb_j.max + bb_j.min) * 0.5
dyn_j = xform.scale_mesh(dyn_j, unreal.Vector(1.22, 1.25, 1.26), c_j)
bb_j2 = mq.get_mesh_bounding_box(dyn_j)

# Desired top collar: Z = 150.5 (just below Pablo's chin at 155, at neck base 150)
target_top_z = 150.5
dz_j = target_top_z - float(bb_j2.max.z)
dx_j = 0.0 - float((bb_j2.max.x + bb_j2.min.x) * 0.5)
dy_j = 2.28 - float((bb_j2.max.y + bb_j2.min.y) * 0.5)
dyn_j = xform.translate_mesh(dyn_j, unreal.Vector(dx_j, dy_j, dz_j))

# Transfer bone weights
dyn_j = bw.transfer_bone_weights_from_mesh(body_dyn, dyn_j, transfer_opts)

# Save SK_Chaqueta_Pablo_v2
sk_jacket_path = f"{folder}/SK_Chaqueta_Pablo_v2"
if eal.does_asset_exist(sk_jacket_path):
    eal.delete_asset(sk_jacket_path)
sk_jacket, _ = new_assets.create_new_skeletal_mesh_asset_from_mesh(dyn_j, skeleton, sk_jacket_path, skm_opts)
if sk_jacket:
    eal.save_loaded_asset(sk_jacket, False)

# 3. PREPARE PANTS
p_sm = eal.load_asset(f"{folder}/SM_Pantalones_Pablo_v2")
dyn_p = unreal.DynamicMesh()
dyn_p, _ = asset_utils.copy_mesh_from_static_mesh_v2(p_sm, dyn_p, unreal.GeometryScriptCopyMeshFromAssetOptions(), unreal.GeometryScriptMeshReadLOD(), True)

# Pablo's legs: top waistband at Z=106.0, bottom of boots at Z=8.0
# Current pants: Z from -50.0 to 15.0 (height 65cm)
# Scale: X=1.28, Y=1.32, Z=1.51
bb_p = mq.get_mesh_bounding_box(dyn_p)
c_p = (bb_p.max + bb_p.min) * 0.5
dyn_p = xform.scale_mesh(dyn_p, unreal.Vector(1.28, 1.32, 1.51), c_p)
bb_p2 = mq.get_mesh_bounding_box(dyn_p)

# Desired top waistband: Z = 106.5 (overlapping jacket bottom at ~104)
dz_p = 106.5 - float(bb_p2.max.z)
dx_p = 0.0 - float((bb_p2.max.x + bb_p2.min.x) * 0.5)
dy_p = 2.28 - float((bb_p2.max.y + bb_p2.min.y) * 0.5)
dyn_p = xform.translate_mesh(dyn_p, unreal.Vector(dx_p, dy_p, dz_p))

# Transfer bone weights
dyn_p = bw.transfer_bone_weights_from_mesh(body_dyn, dyn_p, transfer_opts)

# Save SK_Pantalones_Pablo_v2
sk_pants_path = f"{folder}/SK_Pantalones_Pablo_v2"
if eal.does_asset_exist(sk_pants_path):
    eal.delete_asset(sk_pants_path)
sk_pants, _ = new_assets.create_new_skeletal_mesh_asset_from_mesh(dyn_p, skeleton, sk_pants_path, skm_opts)
if sk_pants:
    eal.save_loaded_asset(sk_pants, False)

# 4. EQUIP ON BP_PABLO
with unreal.ScopedEditorTransaction("DBV Unreal AI: Equip Perfect Pablo Outfit"):
    pablo.modify()
    
    # Torso
    torso.modify()
    torso.set_skeletal_mesh_asset(sk_jacket)
    torso.empty_override_materials()
    if mat:
        torso.set_material(0, mat)
    torso.set_leader_pose_component(body, force_update=True)
    
    # Legs
    legs.modify()
    legs.set_skeletal_mesh_asset(sk_pants)
    legs.empty_override_materials()
    if mat:
        legs.set_material(0, mat)
    legs.set_leader_pose_component(body, force_update=True)
    
    # Feet (cleared since boots are on pants)
    feet.modify()
    feet.set_skeletal_mesh_asset(None)

result = json.dumps({
    "success": True,
    "jacket": sk_jacket_path,
    "pants": sk_pants_path
}, indent=2)
"""

res = run_in_unreal(code)
print(res.get("result", {}).get("content", [{}])[0].get("text"))

