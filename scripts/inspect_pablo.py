import unreal

actors = unreal.EditorLevelLibrary.get_all_level_actors()

for actor in actors:
    name = actor.get_actor_label()
    if "Pablo" in name or "Victoria" in name:
        print(f"=== Actor: {name} (Class: {actor.get_class().get_name()}) ===")
        comps = actor.get_components_by_class(unreal.SkeletalMeshComponent)
        for c in comps:
            skm = c.get_editor_property("skeletal_mesh_asset")
            skm_path = skm.get_path_name() if skm else "None"
            print(f"  Component '{c.get_name()}': SKM = {skm_path}")
