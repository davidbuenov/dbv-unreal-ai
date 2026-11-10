import unreal

opts = unreal.GeometryScriptTransferBoneWeightsOptions()
print("transfer_method:", opts.transfer_method)
if hasattr(opts, "output_target_mesh_bones"):
    print("output_target_mesh_bones:", opts.output_target_mesh_bones)
for prop in dir(opts):
    if not prop.startswith("_"):
        val = getattr(opts, prop)
        print(f"  {prop} = {val}")

