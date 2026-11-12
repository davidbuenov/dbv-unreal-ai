<!-- ============================================================================= -->
<!-- DBVMCPFirst — Bug report: unreal_dress_metahuman invierte la orientación en vestidos con escote en espalda -->
<!-- Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona -->
<!-- Generated with DBV Unreal AI · https://github.com/davidbuenov/dbv-unreal-ai -->
<!-- Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops -->
<!-- ============================================================================= -->

# Bug: `unreal_dress_metahuman` invierte 180° la orientación en vestidos con cuello alto o espalda baja

- **Fecha:** 2026-11-12
- **Proyecto:** DBVMCPFirst (Unreal Engine 5.8)
- **Componente:** plugin `DBVUnrealAI` → `Content/Python/dbv_unreal_ai/outfit/tailor.py` y `shell.py`
- **Specs afectadas:** REQ-88 (pipeline determinista de vestimenta), REQ-98 (orientación y ajuste al cuerpo)
- **Severidad:** media-alta: prendas con corte asimétrico en cuello/espalda se equipan giradas 180° (el volumen de los glúteos queda delante y el frontal liso detrás).

---

## 1. Reproducción

1. Enviar una prenda a vestir con `unreal_dress_metahuman` cuyo corte frontal sea más alto que el trasero (por ejemplo, vestido con volantes en cuello y escote en espalda o corte abierto posterior):
   ```json
   {
     "actor_label": "Victoria_VestidoAzul",
     "source": "vestido_azul_marino-1794400441413",
     "garment_kind": "dress",
     "garment_cut": "female"
   }
   ```
2. **Resultado observado en el viewport:**
   - La prenda se equipa y pasa la conformación y el Dataflow de Chaos Cloth.
   - Sin embargo, la parte delantera muestra la forma y pliegues de los glúteos sobre la pelvis del MetaHuman.
   - La espalda muestra la caída lisa y recta que correspondía al abdomen/pecho.

---

## 2. Causa raíz

### Heurística frágil en `tailor.py` y `shell.py`

En `tailor.py` (función `normalise`, líneas 320-338) y en `shell.py` (función `orientation`, líneas 90-102):

```python
if kind == "top":
    c = np.abs(p[:, 0]) < 0.06 * w
    front_pts = p[c & (p[:, 1] > 0.0), 2]
    back_pts = p[c & (p[:, 1] < 0.0), 2]
    front_top = float(front_pts.max()) if len(front_pts) else 0.0
    back_top = float(back_pts.max()) if len(back_pts) else 0.0
    flip = front_top > back_top
    info["neck_front_back"] = (round(front_top, 1), round(back_top, 1))
```

La lógica asume como axioma universal:
> *"El lado con el borde superior central más alto es la espalda: el escote es más bajo delante en un top..."* (`shell.py:93-95`).

### Medición real en el asset `vestido_azul_marino-1794400441413`:

| Parámetro | Valor medido | Significado |
|---|---|---|
| `front_top` (`+Y`, frontal original) | **29.7 cm** | Cuello fruncido/volantes delanteros |
| `back_top` (`-Y`, trasero original) | **27.0 cm** | Escote trasero más despegado/bajo |
| `front_top > back_top` | `True` | Se activa `flip180 = True` |

Al ser `29.7 > 27.0`, el algoritmo asume erróneamente que el lado `+Y` es la espalda y aplica:
```python
p = p * np.array([-1.0, -1.0, 1.0])
```
Esto invierte deliberadamente el frente hacia atrás.

### Por qué rotar la malla origen antes de enviarla no lo soluciona

Si se gira la malla fuente 180° antes de llamar a `normalise`:
1. El cuello (29.7 cm) pasa a estar en `-Y` y el escote (27.0 cm) en `+Y`.
2. `normalise` calcula `front_top = 27.0` y `back_top = 29.7`.
3. `front_top > back_top` da `False`, por lo que **no** aplica `flip180`.
4. El resultado final sitúa nuevamente el cuello (29.7 cm) en `-Y` (la espalda).

`normalise` fuerza deterministamente que el lado más alto termine siempre en `-Y`. Para cualquier prenda donde el frontal sea más alto que la espalda, el algoritmo siempre la dejará del revés.

---

## 3. Soluciones propuestas

### Opción A (Recomendada): Heurística combinada con curvatura de silueta
Para prendas tipo `dress` o `top` largos, no depender únicamente del escote central. Incorporar la detección de volumen de pecho frente a espalda (o glúteos/cadera en vestidos), análoga a la que ya implementa `normalise` para pantalones (`tailor.py:329-335`):

```python
# En tailor.py / normalise:
if kind == "dress":
    # 1. Comprobar prominencia de pecho a media altura
    zt = float(p[:, 2].max())
    zb = float(p[:, 2].min())
    h = zt - zb
    chest_band = p[(p[:, 2] > zb + 0.6 * h) & (p[:, 2] < zb + 0.8 * h)]
    seat_band = p[(p[:, 2] > zb + 0.3 * h) & (p[:, 2] < zb + 0.5 * h)]

    # Si hay pronunciación clara de glúteos en seat_band o pecho en chest_band,
    # contrastar con neck_front_back antes de decidir el volteo.
```

### Opción B: Parámetro de anulación de orientación (`flip_override` o `yaw_offset`)
Añadir a `dress_metahuman` y a la herramienta MCP `unreal_dress_metahuman` un parámetro opcional:
```python
def dress_metahuman(
    actor_label: str,
    garment_source: str,
    garment_kind: str = "top",
    garment_cut: str | None = None,
    destination_folder: str = "/Game/DBV/MetaHumanOutfits",
    asset_name: str = "",
    scale_multiplier: float = 1.05,
    z_offset: float = 0.0,
    force: bool = False,
    flip_orientation: bool | None = None,  # None = auto, True = forzar volteo 180°, False = no voltear
) -> str:
```

Si `flip_orientation is not None`, `normalise` y `fit_dress` respetan la directiva del usuario/agente en lugar de forzar la heurística del escote.

---

## 4. Estado (2026-11-12)

**Corregido. El informe acertaba.** Medido en `vestido_azul_marino`: la cintura se hunde 8,4 cm en el lado que el escote tomaba por frente y 1,5 cm en el otro, así que ese lado es la espalda.

- **Opción A, hecha:** `tailor.dress_back_side` decide la espalda por la silueta (hundimiento de la cintura entre la espalda alta y los glúteos, con un margen de 1,5 cm). El escote solo decide si la silueta no distingue. Tests: `test_la_espalda_se_reconoce_por_la_curva_lumbar_y_no_por_el_escote` y `test_flip_orientation_gira_el_vestido_180_grados`.
- **Opción B, hecha:** `flip_orientation` (opcional, `false` por defecto) en `dress_metahuman`, `dress_metahuman_start`, el toolset, el puente C++, la herramienta MCP `unreal_dress_metahuman` y la de la IA de la app. Gira 180° el resultado de la detección.
- Comprobado en el editor: `Victoria_VestidoAzul` vestida con `front_by: silhouette`, validación `ok`, pecho delante y glúteos detrás en la captura de perfil.
- Nota: el mismo día se descartó por error la opción A y se volvió a poner el vestido al revés; ver REJECTED-16 en `memory.md`. `shell.py` ya no existe (se retiró con REJECTED-15), así que solo cambia `tailor.py`.
