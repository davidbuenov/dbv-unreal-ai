import unreal

def capture_actor(actor_name, out_filename):
    actor = next((a for a in unreal.EditorLevelLibrary.get_all_level_actors() if a.get_actor_label() == actor_name), None)
    if not actor:
        return
    loc = actor.get_actor_location()
    cam_pos = unreal.Vector(loc.x + 180.0, loc.y - 120.0, loc.z + 120.0)
    sub = unreal.get_editor_subsystem(unreal.UnrealEditorSubsystem)
    unreal.AutomationLibrary.take_high_res_screenshot(1280, 720, out_filename)

print("Helper ready")
