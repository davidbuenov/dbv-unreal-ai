import unreal

actors = unreal.EditorLevelLibrary.get_all_level_actors()

for actor in actors:
    name = actor.get_actor_label()
    if "Pablo" in name:
        loc = actor.get_actor_location()
        print(f"Actor {name} at location: X={loc.x}, Y={loc.y}, Z={loc.z}")

