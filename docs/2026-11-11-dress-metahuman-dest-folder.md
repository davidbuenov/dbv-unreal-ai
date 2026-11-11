<!-- ============================================================================= -->
<!-- DBVMCPFirst — Bug report: unreal_dress_metahuman falla en fit_and_equip (cloth) -->
<!-- Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona -->
<!-- Generated with DBV Unreal AI · https://github.com/davidbuenov/dbv-unreal-ai -->
<!-- Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops -->
<!-- ============================================================================= -->

# Bug: `unreal_dress_metahuman` falla con vestidos/faldas (Chaos Cloth)

- **Fecha:** 2026-11-11
- **Proyecto:** DBVMCPFirst (Unreal Engine 5.8)
- **Componente:** plugin `DBVUnrealAI` → `Content/Python/dbv_unreal_ai/outfit/`
- **Specs afectadas:** REQ-88 (pipeline determinista), REQ-87/85/84 (Chaos Cloth)
- **Severidad:** alta — ninguna prenda `skirt` o `dress` se puede equipar.

## Reproducción

1. `generate_metahuman_outfit` con `kind="dress"`, `cut="female"` →
   asset de librería `vestido_azul_marino-1794400441413` (OK).
2. `unreal_dress_metahuman`:
   ```json
   {
     "actor_label": "Victoria_VestidoAzul",
     "source": "vestido_azul_marino-1794400441413",
     "garment_kind": "dress",
     "garment_cut": "female"
   }
   ```
3. Resultado:
   ```json
   {
     "success": false,
     "job_id": "outfit_1794400498566_1",
     "error": "Exception in step 'fit_and_equip': TypeError: setup_cloth_for_character() got an unexpected keyword argument 'dest_folder'",
     "step_index": 3, "total_steps": 5, "step_name": "fit_and_equip"
   }
   ```

## Causa

Desajuste de firma entre el llamador y la función.

`pipeline.py:409` (paso `fit_and_equip`, método `cloth`):

```python
cloth_res = cloth.setup_cloth_for_character(
    actor_or_bp_path=bp_path,
    garment_kind=kind,
    garment_name=asset_name,
    render_sm_path=source,
    dest_folder=dest_folder,      # <- no existe en la firma
    target_triangles=1500,
)
```

`cloth.py:357`:

```python
def setup_cloth_for_character(
    actor_or_bp_path, garment_kind, garment_name, render_sm_path,
    skeletal_mesh_path=None, physics_asset_path=None, material_path=None,
    sim_sm_path=None, target_triangles=1500, slot_name="SkirtCloth",
)
```

La función calcula la carpeta internamente (`cloth.py` ~l.392:
`dest_folder = f"/Game/DBV/MetaHumanOutfits/{garment_name}"`), así que el
`dest_folder` del job (`job_data["destination_folder"]`) se ignora.

## Corrección propuesta

Opción A (preferida): añadir `dest_folder: str | None = None` a
`setup_cloth_for_character` y usarlo si viene, con el valor actual como
respaldo:

```python
dest_folder = dest_folder or f"/Game/DBV/MetaHumanOutfits/{garment_name}"
```

Opción B: quitar `dest_folder=dest_folder` de la llamada en `pipeline.py`.

Añadir un test que llame al paso `fit_and_equip` con `kind` `skirt` y
`dress` (o al menos que compruebe la firma con `inspect.signature`).

## Revisar también

- `render_sm_path=source`: `source` puede ser un **id de librería** (como en
  este caso), no una ruta `/Game/...` de StaticMesh. Verificar que en el paso
  `fit_and_equip` se pasa la ruta del StaticMesh ya importado y no el id;
  si no, el siguiente fallo será en `generate_cloth_simulation_mesh`.
- Estado parcial: el job se queda en el paso 3 de 5 con los pasos
  `prepare_character`, `apply_slot_rules` y `prepare_skin` ya aplicados
  (copia `_DBV` creada, ranuras Torso/Legs vaciadas, piel limpiada) pero sin
  prenda. Valorar revertir o dejar el job reanudable tras un fallo.

## Estado

Corregido el 2026-11-11:
1. `dest_folder: str | None = None` añadido a `setup_cloth_for_character` en `cloth.py`.
2. `pipeline.py` importa automáticamente el asset si `source` es un archivo en disco o id de biblioteca antes de pasar `render_sm_path`.
3. Rollback automático con `character.revert_to_original(actor_label)` integrado si cualquier paso falla.
4. Sincronizado a `D:\Programacion\Unreal\DBVMCPFirst\Plugins\DBVUnrealAI`. Listo para probar el paso 2 con el mismo asset.

## Revisión posterior (2026-11-11)

La corrección anterior quitaba el `TypeError`, pero el vestido seguía sin poder equiparse. Comprobado
en el editor abierto (solo lectura) y en el código:

- **La causa de fondo era otra.** `pipeline.py` leía `copy_blueprint_path`, y `prepare_character_copy`
  devuelve `copy_bp_path`. El Blueprint de la copia quedaba vacío: el paso `apply_slot_rules` se saltaba
  (por eso `Victoria_VestidoAzul` conserva `Torso` y `Legs`) y la tela recibía la etiqueta del actor
  como ruta de Blueprint (`Blueprint no encontrado: Victoria_VestidoAzul`). Los tests no lo veían porque
  simulaban la respuesta con la clave equivocada. Corregido, con un test que llama al orquestador real.
- **El cuerpo por defecto de la tela no existe.** `/Game/MetaHumans/Victoria/f_med_nrw_body` no está en
  el proyecto; el cuerpo real es `/Game/MetaHumans/Common/Female/Medium/NormalWeight/Body/f_med_nrw_body`.
  Ahora se usan el cuerpo y el Physics Asset del propio personaje, y se comprueba que existen.
- **El rollback del punto 3 era peligroso.** Revertía cualquier copia al Blueprint original, también la
  de un personaje ya vestido (`BP_David3_MH` lleva los tirantes y los shorts en el actor del nivel): un
  fallo en una prenda nueva habría borrado las anteriores. Ahora solo se revierte la copia creada por el
  trabajo que falla; en las demás se devuelven a las ranuras sus mallas previas.
- **La importación del punto 2 escondía errores** (si fallaba, seguía con el id o la ruta del fichero).
  Ahora el paso falla con el motivo y exige que la prenda sea un `StaticMesh`.

Pendiente: sincronizar el plugin con `DBVMCPFirst`, reiniciar el editor y repetir el paso 2 de la
reproducción. La corrección está cubierta por tests, no probada en el editor.
