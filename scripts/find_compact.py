import unreal

found = []
for mod_name in dir(unreal):
    if "GeometryScript" in mod_name:
        mod = getattr(unreal, mod_name)
        for fn in dir(mod):
            if "compact" in fn.lower():
                found.append(f"{mod_name}.{fn}")

print("Compact functions found:", found)

