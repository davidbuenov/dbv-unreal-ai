import unreal

skm = unreal.load_asset("/Game/MetaHumans/Common/Male/Tall/NormalWeight/Tops/Shirt/m_tal_nrw_top_shirt_nrm")
print(f"skm: {skm.get_path_name()}")
print(f"materials: {len(skm.materials)}")
for i, m in enumerate(skm.materials):
    mat_interface = m.material_interface
    print(f"  mat {i}: {mat_interface.get_path_name() if mat_interface else 'None'}")
