<!-- ============================================================================= -->
<!-- DBVMCPFirst — Bug report: unreal_dress_metahuman pierde la ruta del Blueprint copia -->
<!-- Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona -->
<!-- Generated with DBV Unreal AI · https://github.com/davidbuenov/dbv-unreal-ai -->
<!-- Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops -->
<!-- ============================================================================= -->

# Bug: `unreal_dress_metahuman` no encuentra el Blueprint en el paso `fit_and_equip` (vestido / Chaos Cloth)

- **Fecha:** 2026-11-11
- **Componente:** plugin `DBVUnrealAI`, `Content/Python/dbv_unreal_ai/outfit/` (`pipeline.py`, `character.py`, `cloth.py`)
- **Relacionado con:** `docs/bugs/2026-11-11-dress-metahuman-dest-folder.md`. Ese fallo (`dest_folder`) está corregido y ahora el proceso avanza más, hasta este nuevo error.
- **Gravedad:** alta. Ninguna falda ni vestido (método `cloth`) se puede equipar. En prendas por huesos (top, pantalón, zapatos) el paso 2 tampoco hace nada.

## Cómo reproducirlo

1. Editor de Unreal recién reiniciado, con el plugin actualizado.
2. Nivel con el MetaHuman `Victoria_VestidoAzul` (build `f_med_nrw`).
3. Llamada:

   ```json
   unreal_dress_metahuman {
     "actor_label": "Victoria_VestidoAzul",
     "source": "/Game/DBV/MetaHumanOutfits/model/StaticMeshes/Vestido_azul_marino",
     "garment_kind": "dress",
     "garment_cut": "female"
   }
   ```

4. Resultado (`job_id` `outfit_1794406195112_1`):

   ```json
   {"success": false, "error": "Blueprint no encontrado: Victoria_VestidoAzul",
    "step_index": 3, "total_steps": 5, "step_name": "fit_and_equip"}
   ```

El mismo error sale antes de reiniciar (`outfit_1794405997397_4`), así que no depende de la caché de módulos de Python.

## Causa

### 1. Las claves de la copia del personaje no coinciden (causa principal)

`character.prepare_character_copy()` devuelve la ruta con estas claves (`character.py:176-177` y `character.py:256-257`):

```python
"copy_bp_path": ...,
"original_bp_path": ...,
```

`pipeline.py:365-366` las lee con otros nombres:

```python
ctx["working_bp_path"] = prep_res.get("copy_blueprint_path")      # siempre None
ctx["original_bp_path"] = prep_res.get("original_blueprint_path")  # siempre None
```

Por eso `ctx["working_bp_path"]` queda siempre en `None` y:

- **Paso 1 `apply_slot_rules` (`pipeline.py:377-378`):** con `bp_path` en `None` el paso se salta y devuelve `{"success": True}` sin avisar. No se quitan la camiseta ni los vaqueros, aunque el informe del paso diga "Slot rules applied".
- **Paso 3 `fit_and_equip` (`pipeline.py:408`):** `bp_path = ctx.get("working_bp_path") or actor_label`. Usa la **etiqueta del actor** (`Victoria_VestidoAzul`) como si fuera una ruta de asset. `cloth.py:267` hace `does_asset_exist("Victoria_VestidoAzul")` y falla con "Blueprint no encontrado".
- **Resultado final (`pipeline.py:484`):** `"blueprint"` también sale en `None`.

### 2. `apply_slot_rules` recibe el tipo de dato equivocado (aparecerá al corregir el punto 1)

`character.apply_slot_rules(actor, garment_kind, original_slots=None)` (`character.py:268`) espera un **actor**. `pipeline.py:378` le pasa la **ruta del Blueprint** (`bp_path`). Al corregir el punto 1, el paso 1 dejará de saltarse y probablemente fallará o no hará nada. Debería recibir el actor de nivel (`_find_level_actor(actor_label)`, como hace el paso 2) y, si hace falta, `original_slots` del resultado de `prepare_character_copy`.

### 3. Los fallos de importación se ocultan (`pipeline.py:410-418`)

Cuando `source` no empieza por `/Game/` (por ejemplo, un id de la biblioteca), se llama a `editor.import_model(...)`. Si la importación falla o lanza una excepción, el `except Exception:` deja `render_sm = source` sin registrar el motivo. Después `cloth` falla con un error confuso:

```
Asset de render no existe: C:\Users\bueno\DBV Unreal AI\Library\vestido_azul_marino-1794400441413\model.glb
```

El fichero **sí existe** en el disco (6,9 MB). El fallo real está en la importación y no llega al usuario. Además, la ruta de la biblioteca llega como ruta de fichero y no como id. Para continuar, importé primero el vestido con `library_send_to_unreal` y pasé la ruta `/Game/...`.

## Corrección propuesta

```python
# pipeline.py:365-366
ctx["working_bp_path"] = prep_res.get("copy_bp_path")
ctx["original_bp_path"] = prep_res.get("original_bp_path")
ctx["original_slots"] = prep_res.get("original_slots")
```

```python
# pipeline.py:376-378
actor = _find_level_actor(actor_label)
if actor is None:
    step_err = f"MetaHuman '{actor_label}' not found"
else:
    slot_res = character.apply_slot_rules(actor, kind, ctx.get("original_slots"))
```

```python
# pipeline.py:408 — no usar la etiqueta como ruta
bp_path = ctx.get("working_bp_path")
if not bp_path:
    step_err = "No working Blueprint path from prepare_character"
```

```python
# pipeline.py:417 — no ocultar el error de importación
except Exception as exc:
    step_err = f"Import of garment failed: {exc}"
```

Además: si `imp_data["success"]` es falso, devolver su `error` en lugar de seguir con `render_sm = source`.

## Tests sugeridos

- `prepare_character_copy` → `pipeline`: comprobar que `ctx["working_bp_path"]` no es `None`, tanto con un actor ya copiado (`reused=True`) como con uno nuevo (`reused=False`).
- `apply_slot_rules` con `kind="dress"`: tras el paso 1, las ranuras Torso y Legs deben quedar vacías.
- `fit_and_equip` con `method="cloth"` y `source` como id de biblioteca: debe importar el modelo o devolver el error real de la importación.
- Un test de contrato que compare las claves que devuelve `prepare_character_copy` con las que lee `pipeline.py`, para que no vuelvan a divergir.

## Estado en el que queda el proyecto tras el fallo

Comprobado en el editor tras el último intento:

- **Actor `Victoria_VestidoAzul`:** clase `/Game/DBV/Runway/Cloth/BP_Victoria_Vestido_C` (la original, sin tags de copia DBV). Sigue con `SK_Top_Conform` y los vaqueros. **No lleva el vestido.**
  - Antes de reiniciar, la lista de MetaHumans mostraba la clase `BP_Victoria_Vestido_DBV_C`. Tras reiniciar vuelve a la original. Puede deberse a que el nivel no se guardó o a una reversión del propio proceso; no lo he verificado.
- **Assets que han quedado a medias** (el proceso llegó a crearlos antes de fallar). Están sueltos en la raíz `/Game/DBV/MetaHumanOutfits/` y no en una subcarpeta de la prenda:
  - `/Game/DBV/MetaHumanOutfits/SM_Dress_Victoria_VestidoAzul_Sim`
  - `/Game/DBV/MetaHumanOutfits/DF_Dress_Victoria_VestidoAzul`
  - `/Game/DBV/MetaHumanOutfits/CA_Dress_Victoria_VestidoAzul`
- **Copia del Blueprint existente:** `/Game/DBV/MetaHumanOutfits/Victoria_Vestido/BP_Victoria_Vestido_DBV`.
- **Vestido importado** (lo hice yo para evitar el punto 3): `/Game/DBV/MetaHumanOutfits/model/StaticMeshes/Vestido_azul_marino`. La carpeta `model/` sale del nombre del fichero `model.glb`; convendría usar el nombre de la prenda.

## Estado

Corregido el 2026-11-11 (los puntos 1 a 3 en el commit `bc16bfd`; el resto en el siguiente):

1. **Claves:** `pipeline.py` lee `copy_bp_path` y `original_bp_path`. Test de contrato con la función
   real, copia nueva y reutilizada (`test_contrato_prepare_character_copy_con_el_pipeline`).
2. **`apply_slot_rules`** recibe el actor del nivel y las mallas previas de las ranuras.
3. **Importación:** si falla, el paso devuelve el motivo; la prenda de tela debe ser un `StaticMesh`.
   Sin Blueprint de copia el paso falla en vez de usar la etiqueta del actor.
4. **Restos:** cada prenda de tela va a su carpeta (`<destino>/<nombre>`), no a la raíz, y si el
   Cloth Asset no llega a crearse se borran la simulación, el Dataflow y el Cloth Asset a medias.
   Un reintento sustituye los que existan.
5. Los pesos de la tela salen del cuerpo y el Physics Asset del propio personaje.

Sin hacer: los tres assets sueltos de este intento (`SM_Dress_…_Sim`, `DF_Dress_…`, `CA_Dress_…` en
la raíz) siguen en el proyecto y se pueden borrar a mano; la carpeta `model/` la crea
`library_send_to_unreal` a partir del nombre del fichero, no este pipeline.

Cubierto por tests; falta repetir la reproducción en el editor con el plugin sincronizado.

Sugerencia original: si `fit_and_equip` falla, borrar o reutilizar los assets `SM_*_Sim`, `DF_*` y `CA_*` creados. Así los reintentos no acumulan restos ni chocan con nombres que ya existen.
