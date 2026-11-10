import unreal

for name in ["BP_Pablo", "Pablo_Kilt", "Pablo_Deportista"]:
    actor = None
    for a in unreal.EditorLevelLibrary.get_all_level_actors():
        if a.get_actor_label() == name:
            actor = a
            break
    if not actor:
        continue
    print(f"\n--- {name} ---")
    comps = {c.get_name(): c for c in actor.get_components_by_class(unreal.SkeletalMeshComponent)}
    for cname, c in comps.items():
        skm = c.get_skeletal_mesh_asset()
        vis = c.get_editor_property("visible")
        leader = c.get_editor_property("leader_pose_component")
        leader_name = leader.get_name() if leader else "None"
        loc = c.get_editor_property("relative_location")
        print(f"  {cname}: skm={skm.get_path_name() if skm else 'None'}, vis={vis}, leader={leader_name}, loc=({loc.x:.1f},{loc.y:.1f},{loc.z:.1f})")
