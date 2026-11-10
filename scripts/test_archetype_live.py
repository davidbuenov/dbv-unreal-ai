import unreal
import dbv_unreal_ai.metahuman as mh

actor = None
for a in unreal.EditorLevelLibrary.get_all_level_actors():
    if a.get_actor_label() == "Pablo_Deportista":
        actor = a
        break

if not actor:
    print("Pablo_Deportista not found")
else:
    comps = {c.get_name(): c for c in actor.get_components_by_class(unreal.SkeletalMeshComponent)}
    body_comp = comps.get("Body")
    body_skm = body_comp.get_skeletal_mesh_asset() if body_comp else None
    print(f"body_skm: {body_skm.get_path_name() if body_skm else 'None'}")
    
    arch = mh.resolve_full_body_archetype(body_skm, actor)
    print(f"resolve_full_body_archetype: {arch}")
    
    build = mh.body_build(body_skm, actor)
    print(f"body_build: {build}")
    
    cand_skm, cand_path = mh._find_full_body_asset(build)
    print(f"_find_full_body_asset: {cand_skm} -> {cand_path}")
    
    full_skm, body_dyn, restored, source_path = mh._ensure_full_body(
        body_comp, body_skm, build, "Torso", actor
    )
    print(f"_ensure_full_body: full_skm={full_skm.get_path_name() if full_skm else 'None'}, restored={restored}, source_path={source_path}")
    if body_dyn:
        print(f"body_dyn vertex count: {body_dyn.get_vertex_count()}")

