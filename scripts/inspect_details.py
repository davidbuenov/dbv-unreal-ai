import unreal

for name in ["BP_Pablo", "Pablo_Kilt", "Pablo_Deportista"]:
    actor = None
    for a in unreal.EditorLevelLibrary.get_all_level_actors():
        if a.get_actor_label() == name:
            actor = a
            break
    if not actor:
        print(f"Actor {name} not found")
        continue
    print(f"\n================= {name} =================")
    comps = actor.get_components_by_class(unreal.SkeletalMeshComponent)
    for c in comps:
        skm = c.get_editor_property("skeletal_mesh_asset")
        skm_name = skm.get_path_name() if skm else "None"
        vis = c.get_editor_property("visible")
        print(f"  [{c.get_name()}] visible={vis} skm={skm_name}")
