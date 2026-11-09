# Especificación Técnica: Generación e Integración de Ropa para MetaHuman vía IA (`generate_metahuman_outfit`)

## 1. Objetivo y Alcance
Implementar una nueva herramienta (Tool) en el servidor MCP de `dbv-unreal-ai` escrita en **Rust** que permita generar prendas 3D mediante la API de **Tripo 3D**, post-procesar su topología, importarlas automáticamente en **Unreal Engine 5 (5.4 / 5.6+)**, alinear su geometría con el cuerpo base del MetaHuman, transferir los pesos esqueléticos (*Skin Weights*) mediante `GeometryScript` y equipar la prenda en el componente correspondiente del Actor MetaHuman.

---

## 2. Arquitectura del Pipeline (End-to-End)

1. **Entrada MCP (Rust):** El cliente MCP invoca `generate_metahuman_outfit` con el prompt de la prenda, el slot de destino (`Torso`, `Legs`, `Feet`), el cuerpo base de referencia y (opcionalmente) el Actor en escena.
2. **Generación + Retopología (Tripo 3D API v2):**
   - Creación de tarea `text_to_model` (con *prompt engineering* forzado a A-Pose de 45° y malla hueca) o `image_to_model` (usando plantilla de referencia A-Pose de MetaHuman).
   - Ejecución encadenada de tarea `convert_model` con `quad: true` y `face_limit: 8000` para obtener una malla de *quads* limpia apta para deformación esquelética.
   - Descarga del archivo `.glb` resultante en el directorio de *staging* local.
3. **Procesado en Unreal Engine (Python Bridge / GeometryScript):**
   - Importación desatendida del `.glb` como `UStaticMesh` con sus materiales PBR.
   - Volcado del `SkeletalMesh` del cuerpo base del MetaHuman (`Source`) y del `StaticMesh` de la prenda (`Target`) a objetos `UDynamicMesh` en memoria.
   - Auto-escalado y traslación de la *Bounding Box* de la prenda según las cotas anatómicas del slot (`Torso`, `Legs`, `Feet`).
   - Transferencia automática de pesos de huesos mediante `TransferBoneWeightsFromMesh` usando el algoritmo **`InpaintWeights`**.
   - Creación del nuevo `USkeletalMesh` asignado al esqueleto `metahuman_base_skel`.
   - *(Opcional UE 5.6+)* Creación de `UChaosOutfitAsset` para soporte de cuerpos paramétricos.
4. **Inyección en el Actor (Hot-Reload en Viewport):**
   - Asignación del nuevo `SkeletalMesh` al componente correspondiente (`Torso`, `Legs`, `Feet`) del Blueprint del MetaHuman y sincronización con `SetLeaderPoseComponent(Body)`.

---

## 3. Especificación de la Tool MCP (Servidor Rust)

### 3.1. Definición del Esquema (`GenerateMetaHumanOutfitArgs`)

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GenerateMetaHumanOutfitArgs {
    /// Descripción detallada de la prenda (ej. "tactical vest with pouches, black")
    pub prompt: String,
    /// Nombre identificador del asset final en Unreal (ej. "TacticalVest_01")
    pub asset_name: String,
    /// Slot anatómico del MetaHuman: "Torso" | "Legs" | "Feet"
    pub target_slot: String,
    /// Ruta en Content Browser del SkeletalMesh base del cuerpo del MetaHuman
    /// Por defecto: "/Game/MetaHumans/Common/Male/Medium/NormalWeight/Body/m_med_nrw_body"
    pub base_body_mesh_path: Option<String>,
    /// Ruta de destino en el Content Browser donde se guardará la prenda
    /// Por defecto: "/Game/MetaHumans/GeneratedClothing"
    pub destination_content_path: Option<String>,
    /// Etiqueta (ActorLabel) del MetaHuman en el nivel actual para equipar la prenda en vivo
    pub target_actor_label: Option<String>,
    /// Límite de polígonos para la retopología en quads de Tripo 3D (por defecto: 8000)
    pub quad_face_limit: Option<u32>,
}
```

### 3.2. Reglas de Prompt Engineering e Invocación a Tripo 3D

Al construir el payload para `POST https://api.tripo3d.ai/v2/openapi/task`:

1. **Modificador de Prompt por Slot:**
   - Si `target_slot == "Torso"`:
     `"{prompt}, 3D clothing garment only, strict A-pose with sleeves angled at 45 degrees downwards, symmetrical, hollow interior at neck, waist and cuffs, no mannequin, no human body, game-ready PBR"`
   - Si `target_slot == "Legs"`:
     `"{prompt}, 3D pants/trousers garment only, strict A-pose legs slightly apart, symmetrical, hollow waist and ankle openings, no mannequin, no human body, game-ready PBR"`
   - Si `target_slot == "Feet"`:
     `"{prompt}, pair of shoes/boots aligned symmetrically for rigging, hollow ankle opening, game-ready PBR"`

2. **Paso obligatorio de Retopología (Quad Remesh):**
   Una vez que la tarea de generación (`task_id`) alcance `status == "success"`, lanzar una segunda tarea antes de descargar:
   ```json
   {
     "type": "convert_model",
     "original_model_task_id": "<GENERATION_TASK_ID>",
     "format": "GLTF",
     "quad": true,
     "face_limit": 8000,
     "flatten_bottom": false
   }
   ```
   Descargar el `.glb` resultante de esta segunda tarea en `<staging_dir>/<asset_name>.glb`.

---

## 4. Script de Automatización en Unreal Engine (Python Bridge)

Requisitos previos en el `.uproject` de Unreal Engine:
- Plugins habilitados: `PythonScriptPlugin`, `GeometryScripting`, `EditorScriptingUtilities`. El glTF lo importa Interchange, sin plugin aparte (`GLTFImporter` no existe en UE 5.8).
- Solo para prendas con tela (Chaos Cloth): `ChaosClothAssetToolset`. Arrastra `ChaosClothAsset`, `ChaosClothAssetEditorCore` y `ChaosClothAssetDataflowNodes`; `Dataflow` y `ChaosCloth` ya vienen activos por defecto. No hacen falta `ChaosFlesh`, `ChaosFleshGenerator`, `ChaosClothGenerator`, `ChaosMover`, `ChaosRigidAsset`, `ChaosDataflowSolver`, `ChaosClothAssetUsdDataflowNodes`, `InterchangeChaosClothAsset` ni `InterchangeOpenUSDChaosClothAsset`.

El servidor Rust debe enviar y ejecutar el siguiente script en el entorno Python de Unreal Engine a través del puente existente en `dbv-unreal-ai`:

```python
import unreal

def process_and_rig_metahuman_garment(
    glb_disk_path: str,
    asset_name: str,
    dest_content_path: str = "/Game/MetaHumans/GeneratedClothing",
    mh_body_path: str = "/Game/MetaHumans/Common/Male/Medium/NormalWeight/Body/m_med_nrw_body",
    target_slot: str = "Torso",
    target_actor_label: str = ""
) -> str:
    # 1. Asegurar directorio de destino
    if not unreal.EditorAssetLibrary.does_directory_exist(dest_content_path):
        unreal.EditorAssetLibrary.make_directory(dest_content_path)

    # 2. Importar el .glb de Tripo 3D como StaticMesh
    import_task = unreal.AssetImportTask()
    import_task.filename = glb_disk_path
    import_task.destination_path = dest_content_path
    import_task.destination_name = f"SM_{asset_name}"
    import_task.replace_existing = True
    import_task.automated = True
    import_task.save = True

    unreal.AssetToolsHelpers.get_asset_tools().import_asset_tasks([import_task])
    
    sm_path = f"{dest_content_path}/SM_{asset_name}"
    garment_sm = unreal.EditorAssetLibrary.load_asset(sm_path)
    mh_body_skm = unreal.EditorAssetLibrary.load_asset(mh_body_path)

    if not garment_sm or not mh_body_skm:
        raise RuntimeError(f"Error cargando assets: SM={sm_path}, Body={mh_body_path}")

    # 3. Inicializar librerías de GeometryScript
    copy_lib = unreal.GeometryScriptLibrary_SceneUtilityFunctions
    transform_lib = unreal.GeometryScriptLibrary_MeshTransformFunctions
    query_lib = unreal.GeometryScriptLibrary_MeshQueryFunctions
    bone_lib = unreal.GeometryScriptLibrary_MeshBoneWeightFunctions

    body_dyn = unreal.DynamicMesh()
    garment_dyn = unreal.DynamicMesh()

    copy_opts = unreal.GeometryScriptCopyMeshFromAssetOptions()
    lod_opts = unreal.GeometryScriptMeshReadLOD()

    # Copiar cuerpo base del MetaHuman (con huesos y pesos) y malla de la prenda
    copy_lib.copy_mesh_from_skeletal_mesh(mh_body_skm, body_dyn, copy_opts, lod_opts)
    copy_lib.copy_mesh_from_static_mesh(garment_sm, garment_dyn, copy_opts, lod_opts)

    # 4. Auto-Alineación Anatómica por Bounding Box (Cotas en cm para m_med_nrw)
    # Cotas de referencia en eje Z para MetaHuman masculino estatura media:
    # Torso: Z_min ~ 85.0, Z_max ~ 148.0 (Altura ~ 63cm, Centro Z ~ 116.5cm)
    # Legs:  Z_min ~ 8.0,  Z_max ~ 95.0  (Altura ~ 87cm, Centro Z ~ 51.5cm)
    # Feet:  Z_min ~ 0.0,  Z_max ~ 14.0  (Altura ~ 14cm, Centro Z ~ 7.0cm)
    slot_presets = {
        "Torso": {"target_height": 64.0, "target_center_z": 117.0},
        "Legs":  {"target_height": 87.0, "target_center_z": 51.5},
        "Feet":  {"target_height": 14.0, "target_center_z": 7.0},
    }
    preset = slot_presets.get(target_slot, slot_presets["Torso"])

    bbox = query_lib.get_mesh_bounding_box(garment_dyn)
    current_height = bbox.max.z - bbox.min.z

    if current_height > 0.001:
        scale_factor = preset["target_height"] / current_height
        transform_lib.scale_mesh(
            garment_dyn,
            unreal.Vector(scale_factor, scale_factor, scale_factor),
            unreal.Vector(0, 0, 0)
        )
        
        # Recalcular Bounding Box tras escalado y centrar sobre el cuerpo del MetaHuman
        scaled_bbox = query_lib.get_mesh_bounding_box(garment_dyn)
        center = (scaled_bbox.min + scaled_bbox.max) * 0.5
        translation = unreal.Vector(-center.x, -center.y, preset["target_center_z"] - center.z)
        transform_lib.translate_mesh(garment_dyn, translation)

    # 5. Transferencia de Esqueleto y Skin Weights mediante InpaintWeights
    transfer_opts = unreal.GeometryScriptTransferBoneWeightsOptions()
    transfer_opts.transfer_method = unreal.GeometryScriptTransferBoneWeightsMethod.INPAINT_WEIGHTS

    bone_lib.transfer_bone_weights_from_mesh(
        source_mesh=body_dyn,
        target_mesh=garment_dyn,
        options=transfer_opts
    )

    # 6. Crear el SkeletalMesh final emparentado al esqueleto de MetaHuman
    skm_opts = unreal.GeometryScriptCreateNewSkeletalMeshAssetOptions()
    skm_opts.enable_recompute_normals = False
    skm_opts.enable_recompute_tangents = True

    skm_asset_path = f"{dest_content_path}/SK_{asset_name}"
    if unreal.EditorAssetLibrary.does_asset_exist(skm_asset_path):
        unreal.EditorAssetLibrary.delete_asset(skm_asset_path)

    new_skm = copy_lib.create_new_skeletal_mesh_asset_from_mesh(
        from_dynamic_mesh=garment_dyn,
        asset_path_and_name=skm_asset_path,
        skeleton=mh_body_skm.skeleton,
        options=skm_opts
    )

    # Copiar el material PBR importado del StaticMesh al nuevo SkeletalMesh
    if garment_sm.static_materials and len(garment_sm.static_materials) > 0:
        skm_materials = []
        for mat_slot in garment_sm.static_materials:
            skm_mat = unreal.SkeletalMaterial()
            skm_mat.material_interface = mat_slot.material_interface
            skm_mat.material_slot_name = mat_slot.material_slot_name
            skm_materials.append(skm_mat)
        new_skm.materials = skm_materials

    unreal.EditorAssetLibrary.save_loaded_asset(new_skm)

    # 7. Equipar en vivo en el Actor MetaHuman del nivel (si se especificó target_actor_label)
    if target_actor_label:
        actor_sub = unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
        for actor in actor_sub.get_all_level_actors():
            if actor.get_actor_label() == target_actor_label:
                skm_components = actor.get_components_by_class(unreal.SkeletalMeshComponent)
                body_comp = None
                slot_comp = None
                for comp in skm_components:
                    if comp.get_name() == "Body":
                        body_comp = comp
                    elif comp.get_name() == target_slot:
                        slot_comp = comp
                
                if slot_comp:
                    slot_comp.set_skeletal_mesh_asset(new_skm)
                    if body_comp:
                        slot_comp.set_leader_pose_component(body_comp)
                break

    return skm_asset_path
```

---

## 5. Plan de Implementación Paso a Paso en `dbv-unreal-ai`

1. **Módulo Tripo (`src/tripo.rs` o equivalente):**
   - Añadir soporte para la tarea `convert_model` (`quad: true`, `face_limit`) tras finalizar `text_to_model` / `image_to_model`.
2. **Módulo de Tools MCP (`src/mcp/tools.rs` o equivalente):**
   - Registrar la tool `generate_metahuman_outfit` con el esquema JSON de `GenerateMetaHumanOutfitArgs`.
   - Implementar la plantilla de inyección del script Python reemplazando los parámetros (`glb_disk_path`, `asset_name`, `target_slot`, `mh_body_path`, `target_actor_label`).
3. **Verificación en el Proyecto de Unreal Engine:**
   - Comprobar que `GeometryScripting` está activo (directamente en el `.uproject` o como dependencia de otro plugin).
   - Probar el flujo generando una prenda de prueba (`target_slot: "Torso"`) y verificando en el visor de `SkeletalMesh` que las articulaciones (`clavicle_l`, `upperarm_l`, `spine_03`) deforman la malla sin vértices huérfanos.