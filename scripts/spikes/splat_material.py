# =============================================================================
# DBV Unreal AI — Spike: material de splats (billboard + caída gaussiana con dither) creado por Python
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
#
# Spike P-10 (REQ-41), 2026-10-26: se ejecuta DENTRO del editor (run_python del toolset). No es código del producto
# todavía: es la referencia para el adaptador de splats propio. Ver memory.md (lecciones del spike).

import unreal

FOLDER = "/Game/DBV/Splats/Spike"
NAME = "M_DBVSplat"
MEL = unreal.MaterialEditingLibrary
path = f"{FOLDER}/{NAME}"
# Nunca borrar un material en uso: abre un diálogo modal que bloquea el editor (y el MCP). Se vacía y se rehace.
if unreal.EditorAssetLibrary.does_asset_exist(path):
    mat = unreal.load_asset(path)
    MEL.delete_all_material_expressions(mat)
else:
    mat = unreal.AssetToolsHelpers.get_asset_tools().create_asset(NAME, FOLDER, unreal.Material, unreal.MaterialFactoryNew())
mat.set_editor_property("blend_mode", unreal.BlendMode.BLEND_MASKED)
mat.set_editor_property("shading_model", unreal.MaterialShadingModel.MSM_UNLIT)
mat.set_editor_property("two_sided", True)
mat.set_editor_property("opacity_mask_clip_value", 0.5)


def custom_input(name):
    item = unreal.CustomInput()
    item.set_editor_property("input_name", name)
    return item


def node(cls, x, y):
    return MEL.create_material_expression(mat, cls, x, y)


corner = node(unreal.MaterialExpressionTextureCoordinate, -900, -200)
corner.set_editor_property("coordinate_index", 0)
radius = node(unreal.MaterialExpressionTextureCoordinate, -900, 0)
radius.set_editor_property("coordinate_index", 1)
# Color en UV (Interchange no trae COLOR_0 por defecto): UV2 = (R, G), UV3 = (B, opacidad), en lineal.
red_green = node(unreal.MaterialExpressionTextureCoordinate, -900, -600)
red_green.set_editor_property("coordinate_index", 2)
blue_alpha = node(unreal.MaterialExpressionTextureCoordinate, -900, -450)
blue_alpha.set_editor_property("coordinate_index", 3)
color = node(unreal.MaterialExpressionCustom, -600, -550)
color.set_editor_property("output_type", unreal.CustomMaterialOutputType.CMOT_FLOAT3)
color.set_editor_property("inputs", [custom_input(n) for n in ("RG", "BA")])
color.set_editor_property("code", "return float3(RG.x, RG.y, BA.x);")
MEL.connect_material_expressions(red_green, "", color, "RG")
MEL.connect_material_expressions(blue_alpha, "", color, "BA")

campos = node(unreal.MaterialExpressionCameraPositionWS, -1300, 200)
worldpos = node(unreal.MaterialExpressionWorldPosition, -1300, 300)
worldpos.set_editor_property("world_position_shader_offset", unreal.WorldPositionIncludedOffsets.WPT_EXCLUDE_ALL_SHADER_OFFSETS)
tocam = node(unreal.MaterialExpressionSubtract, -1000, 250)
MEL.connect_material_expressions(campos, "", tocam, "A")
MEL.connect_material_expressions(worldpos, "", tocam, "B")


def local_axis(vec, y):
    const = node(unreal.MaterialExpressionConstant3Vector, -1300, y)
    const.set_editor_property("constant", unreal.LinearColor(vec[0], vec[1], vec[2], 0))
    tr = node(unreal.MaterialExpressionTransform, -1000, y)
    tr.set_editor_property("transform_source_type", unreal.MaterialVectorCoordTransformSource.TRANSFORMSOURCE_LOCAL)
    tr.set_editor_property("transform_type", unreal.MaterialVectorCoordTransform.TRANSFORM_WORLD)
    MEL.connect_material_expressions(const, "", tr, "")
    return tr


# glTF X → Unreal local X; glTF Y → Unreal local Z (medido con los límites de la malla importada).
axis_a = local_axis((1, 0, 0), 450)
axis_b = local_axis((0, 0, 1), 600)

wpo = node(unreal.MaterialExpressionCustom, -500, 200)
wpo.set_editor_property("output_type", unreal.CustomMaterialOutputType.CMOT_FLOAT3)
wpo.set_editor_property("inputs", [custom_input(n) for n in ("Corner", "Radius", "ToCam", "Aw", "Bw")])
wpo.set_editor_property("code", "\n".join([
    "float3 t = normalize(ToCam);",
    "float3 r = normalize(cross(float3(0, 0, 1), t));",
    "float3 u = cross(t, r);",
    "return Radius.x * (Corner.x * (r * length(Aw) - Aw) + Corner.y * (u * length(Bw) - Bw));",
]))
for source, pin in ((corner, "Corner"), (radius, "Radius"), (tocam, "ToCam"), (axis_a, "Aw"), (axis_b, "Bw")):
    MEL.connect_material_expressions(source, "", wpo, pin)
MEL.connect_material_property(wpo, "", unreal.MaterialProperty.MP_WORLD_POSITION_OFFSET)

falloff = node(unreal.MaterialExpressionCustom, -500, -300)
falloff.set_editor_property("output_type", unreal.CustomMaterialOutputType.CMOT_FLOAT1)
falloff.set_editor_property("inputs", [custom_input(n) for n in ("Corner", "Opacity")])
falloff.set_editor_property("code", "\n".join([
    "float a = Opacity.y * exp(-0.5 * dot(Corner, Corner) * 6.25);",
    "float2 p = Parameters.SvPosition.xy + View.StateFrameIndexMod8 * 5.588238;",
    "float n = frac(52.9829189 * frac(dot(p, float2(0.06711056, 0.00583715))));",
    "return a - n + 0.5;",
]))
MEL.connect_material_expressions(corner, "", falloff, "Corner")
MEL.connect_material_expressions(blue_alpha, "", falloff, "Opacity")
MEL.connect_material_property(falloff, "", unreal.MaterialProperty.MP_OPACITY_MASK)
MEL.connect_material_property(color, "", unreal.MaterialProperty.MP_EMISSIVE_COLOR)

MEL.recompile_material(mat)
unreal.EditorAssetLibrary.save_asset(path)

result = path
