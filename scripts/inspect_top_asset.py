import unreal

top_asset = unreal.load_asset("/Game/DBV/MetaHumanOutfits/model/StaticMeshes/Top_Pablo_Deportista")
if not top_asset:
    print("Top_Pablo_Deportista NOT FOUND!")
else:
    print(f"Top_Pablo_Deportista: class={top_asset.get_class().get_name()}")
    num_lods = top_asset.get_num_lods() if hasattr(top_asset, "get_num_lods") else "N/A"
    print(f"LODs: {num_lods}")
    
    asset_utils = unreal.GeometryScript_AssetUtils
    lod = unreal.GeometryScriptMeshReadLOD()
    dyn = unreal.DynamicMesh()
    dyn, _ = asset_utils.copy_mesh_from_static_mesh_v2(top_asset, dyn, unreal.GeometryScriptCopyMeshFromAssetOptions(), lod, True)
    
    mq = unreal.GeometryScript_MeshQueries
    vc = dyn.get_vertex_count()
    tc = dyn.get_triangle_count()
    bbox = mq.get_mesh_bounding_box(dyn)
    print(f"DynamicMesh: verts={vc}, tris={tc}")
    print(f"Bounding box: min=({bbox.min.x:.2f}, {bbox.min.y:.2f}, {bbox.min.z:.2f}), max=({bbox.max.x:.2f}, {bbox.max.y:.2f}, {bbox.max.z:.2f})")

