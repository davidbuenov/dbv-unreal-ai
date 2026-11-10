import unreal

actor = None
for a in unreal.EditorLevelLibrary.get_all_level_actors():
    if a.get_actor_label() == "Pablo_Deportista":
        actor = a
        break

comps = {c.get_name(): c for c in actor.get_components_by_class(unreal.SkeletalMeshComponent)}
for cname, c in comps.items():
    print(f"Component {cname}:")
    num_mats = c.get_num_materials()
    for i in range(num_mats):
        mat = c.get_material(i)
        print(f"  Slot {i}: {mat.get_path_name() if mat else 'None'}")

