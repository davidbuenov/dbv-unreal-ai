# 🏬 Publicación en Microsoft Store: DBV Unreal AI

> **Estado:** 📦 Paquete de la `v0.3.1` generado y listo para subir a Partner Center (§7).  
> **Versión base:** `v0.3.1`  
> **Última revisión:** 2026-11-03  

Documento operativo: checklist accionable para empaquetar y publicar **DBV Unreal AI** en la **Microsoft Store** mediante paquetes MSIX auto-firmados por la tienda, siguiendo las lecciones y estándares de `MARKETPLACE_PUBLISHING.md`.

---

## 1. Vía elegida: MSIX subido directamente a Partner Center

Tauri v2 no genera MSIX nativamente. Se utiliza la herramienta auditada `@choochmeque/tauri-windows-bundle` integrada en el proyecto.

- **Firma de código:** La Microsoft Store firma el paquete automáticamente tras pasar la certificación. No se requiere comprar certificados comerciales Authenticode.
- **Empaquetado:** `npm run tauri:windows:build` compila la app y empaqueta el `.msixbundle` / `.msix` en `src-tauri/target/msix/`.
- **Coexistencia:** El canal MSIX convive perfectamente con el instalador NSIS estándar de GitHub Releases.

---

## 2. Configuración e Identidad del Paquete

Archivos versionados en el repositorio:
- `src-tauri/gen/windows/bundle.config.json`
- `src-tauri/gen/windows/AppxManifest.xml.template`
- `src-tauri/gen/windows/Assets/` (Square150x150, Square44x44, StoreLogo, Wide310x150)

Identidad en `bundle.config.json`:
```json
{
  "identifier": "davidbuenov.DBVUnrealAI",
  "displayName": "dbv-unreal-ai",
  "publisher": "CN=13EE2A5D-F49E-48C9-8873-941069B15D63",
  "publisherDisplayName": "davidbuenov",
  "capabilities": {
    "general": ["internetClient"]
  }
}
```
> **Nota técnica crítica:** `displayName` se mantiene como `"dbv-unreal-ai"` para que coincida exactamente con el nombre de ejecutable producido por Cargo (`dbv-unreal-ai.exe`), evitando el error de empaquetado `Executable not found`.

---

## 3. Checklist de Envío en Partner Center

1. **Reservar nombre:** En Partner Center, crear nuevo envío como producto MSIX (nombre recomendado: *DBV Unreal AI*).
2. **Identidad del producto:** Confirmar que `Package/Identity/Name` (`davidbuenov.DBVUnrealAI`) y `Publisher` coinciden con `bundle.config.json`.
3. **Generación del paquete:**
   ```bash
   npm run tauri:windows:build
   ```
4. **Verificación de Assets (Regla Anti-Placeholder):**
   - Ejecutar `python scripts/generate-store-assets.py` para asegurar que ningún asset en `src-tauri/gen/windows/Assets/` sea un rectángulo sólido.
5. **Política de Privacidad:** Enlazar a `https://davidbuenov.github.io/dbv-unreal-ai/privacidad.html`.
6. **Ficha de Store:**
   - Copiar textos de `descripcionStore_es.md` (Español), `descripcionStore_en.md` (Inglés) y `descripcionStore_zh.md` (Chino simplificado, desde la v0.3.0). Cada idioma de la ficha necesita al menos una captura: valen las mismas de `docs/screenshots/`.
7. **Notas para la certificación:** Indicar que la conexión a Unreal Engine usa el servidor MCP local en `localhost:8000` y que la IA externa es completamente opcional y desactivada por defecto.
8. **Subir paquete y enviar a certificación.**

---

## 4. Envío de la v0.2.3 (actualización, 2026-10-29)

**Paquete generado** con `npm run tauri:windows:build`:

| Fichero | Uso |
| --- | --- |
| `src-tauri/target/msix/dbv-unreal-ai_0.2.3.0.msixbundle` | **El que se sube** a Partner Center (Paquetes) |
| `src-tauri/target/msix/dbv-unreal-ai_0.2.3.0_x64.msix` | El paquete x64 que va dentro del bundle |

**Comprobado antes de subir:**

- `Identity`: `Name="davidbuenov.DBVUnrealAI"`, `Publisher="CN=13EE2A5D-F49E-48C9-8873-941069B15D63"`, `Version="0.2.3.0"`, `x64` (coinciden con `bundle.config.json` y con el producto de Partner Center).
- Capacidades: `internetClient` y `runFullTrust` (necesaria para lanzar los agentes de IA y el `--mcp` como procesos).
- Assets de `src-tauri/gen/windows/Assets/` sin rectángulos de relleno (entre 471 y 998 colores cada uno).
- Tests: Vitest 294, Rust 177, Python y banco e2e en verde.

---

## 5. Envío de la v0.2.4 (mantenimiento y fix MCP, 2026-10-29)

**Paquete generado** con `npm run tauri:windows:build`:

| Fichero | Uso |
| --- | --- |
| `src-tauri/target/msix/dbv-unreal-ai_0.2.4.0.msixbundle` | **El que se sube** a Partner Center (Paquetes) |
| `src-tauri/target/msix/dbv-unreal-ai_0.2.4.0_x64.msix` | El paquete x64 que va dentro del bundle |

**Comprobado antes de subir:**

- `Identity`: `Name="davidbuenov.DBVUnrealAI"`, `Publisher="CN=13EE2A5D-F49E-48C9-8873-941069B15D63"`, `Version="0.2.4.0"`, `x64`.
- **Extensiones:** añadido `uap3:AppExecutionAlias` (`alias="dbv-unreal-ai"`) en `bundle.config.json` para que los agentes externos puedan arrancar el subproceso MCP vía `%LOCALAPPDATA%\Microsoft\WindowsApps\dbv-unreal-ai.exe` sin error de acceso denegado en `WindowsApps`.
- Tests: Vitest 294, Rust 179 pasando al 100%.

**Pasos en Partner Center**:
1. **Paquetes:** subir `dbv-unreal-ai_0.2.4.0.msixbundle` y retirar del envío el anterior.
2. **Novedades de esta versión:** «Corrección en la inicialización del servidor MCP propio al conectar agentes externos de IA».
3. **Enviar a certificación.**

---

## 6. Envío de la v0.3.0 (2026-11-01)

**Paquete generado** con `npm run tauri:windows:build`:

| Fichero | Uso |
| --- | --- |
| `src-tauri/target/msix/dbv-unreal-ai_0.3.0.0.msixbundle` | **El que se sube** a Partner Center (Paquetes) |
| `src-tauri/target/msix/dbv-unreal-ai_0.3.0.0_x64.msix` | El paquete x64 que va dentro del bundle |

**Comprobado antes de subir** (leyendo el `AppxManifest.xml` del `.msix`):

- `Identity`: `Name="davidbuenov.DBVUnrealAI"`, `Publisher="CN=13EE2A5D-F49E-48C9-8873-941069B15D63"`, `Version="0.3.0.0"`, `x64`.
- Capacidades `runFullTrust` e `internetClient`; alias `dbv-unreal-ai.exe` para que los agentes lancen el `--mcp`. La v0.3.0 no necesita capacidades nuevas: compilar (Live Coding, UBT) y abrir el editor son procesos que ya cubre `runFullTrust`.
- Assets sin rectángulos de relleno (`python scripts/generate-store-assets.py`: entre 471 y 998 colores).
- Tests: Rust 233, Vitest 366, Python 47, `dbv-lint`, clippy y banco e2e en verde.

**Pasos en Partner Center:**
1. **Paquetes:** subir `dbv-unreal-ai_0.3.0.0.msixbundle` y retirar del envío el de la 0.2.4.
2. **Fichas de Store:** actualizar español e inglés (descripción, *Novedades de esta versión* y características) y **añadir el idioma Chino (simplificado)** con `descripcionStore_zh.md` y las capturas.
3. **Notas para la certificación:** las de `descripcionStore_en.md`, que ahora explican la compilación con las herramientas de Unreal y la biblioteca en Documentos.
4. **Enviar a certificación.**

---

## 7. Envío de la v0.3.1 (2026-11-03)

**Paquete generado** con `npm run tauri:windows:build`:

| Fichero | Uso |
| --- | --- |
| `src-tauri/target/msix/dbv-unreal-ai_0.3.1.0.msixbundle` | **El que se sube** a Partner Center (Paquetes) |
| `src-tauri/target/msix/dbv-unreal-ai_0.3.1.0_x64.msix` | El paquete x64 que va dentro del bundle |

**Comprobado antes de subir** (ver el resultado real en el resumen de `/ship`):

- `Identity`: `Name="davidbuenov.DBVUnrealAI"`, `Publisher="CN=13EE2A5D-F49E-48C9-8873-941069B15D63"`, `Version="0.3.1.0"`, `x64`.
- Capacidades `runFullTrust` e `internetClient` y alias `dbv-unreal-ai.exe`, como en la 0.3.0. La 0.3.1 no declara capacidades nuevas: las imágenes pegadas se escriben en la carpeta temporal del usuario (`%TEMP%\dbv-unreal-ai-clipboard`, hasta 20 ficheros).
- Assets sin rectángulos de relleno (`python scripts/generate-store-assets.py`: entre 471 y 998 colores).
- Tests: Rust 235, Vitest 371, Python y banco e2e en verde, clippy limpio.
- **Sin probar en la ventana real del MSIX:** (1) arrastrar y soltar ficheros sobre la conversación (la 0.3.1 desactiva `dragDropEnabled` de Tauri para que llegue el `drop` HTML5); (2) pegar una imagen con `Ctrl+V` y comprobar que se guarda en la carpeta temporal y que Tripo puede usarla (el MSIX virtualiza algunas escrituras en `AppData`; ver REJECTED-06). Conviene probar las dos con el MSIX instalado antes de enviar.

**Pasos en Partner Center:**
1. **Paquetes:** subir `dbv-unreal-ai_0.3.1.0.msixbundle` y retirar del envío el de la 0.3.0.
2. **Fichas de Store:** actualizar *Novedades de esta versión* en español (`descripcionStore_es.md`), inglés (`descripcionStore_en.md`) y chino simplificado (`descripcionStore_zh.md`), y la característica 17 y la viñeta de pegar imágenes. Las capturas no cambian.
3. **Notas para la certificación:** las de `descripcionStore_en.md`, que ahora explican también dónde se guardan las imágenes pegadas.
4. **Enviar a certificación.**

---

## 8. Envío de la v0.5.0 (2026-11-10)

> ⚠️ **REQ-90 sigue abierto.** La spec dice que la v0.5.0 no se publica en la Store sin la matriz de 12 casos de ropa ejecutada con la app instalada (MSIX), un juego empaquetado y un vídeo de pasarela. **No se ha ejecutado.** El paquete y la ficha están preparados, pero enviar a certificación es una decisión de la persona responsable sabiendo que ese criterio sigue sin cumplirse (las camisetas y pantalones ceñidos son experimentales y la ficha lo dice).

**Paquete:** `npm run tauri:windows:build` genera `src-tauri/target/msix/dbv-unreal-ai_0.5.0.0.msixbundle` (el que se sube a Partner Center) y el `.msix` x64 que lleva dentro. Antes de subirlo: comprobar `Identity` (`davidbuenov.DBVUnrealAI`, `CN=13EE2A5D-F49E-48C9-8873-941069B15D63`, `Version="0.5.0.0"`, `x64`) y `python scripts/generate-store-assets.py` (assets sin rectángulos de relleno).

**Capacidades:** sin cambios (`runFullTrust` e `internetClient`). La caja negra solo lee el juego del usuario y escribe en `Saved/DBVUnrealAI/Sessions/` de su proyecto de Unreal, sin red.

**Pasos en Partner Center:**
1. **Paquetes:** subir `dbv-unreal-ai_0.5.0.0.msixbundle` y retirar del envío el de la 0.3.1 (o el último enviado).
2. **Fichas de Store:** pegar *Novedades de esta versión*, la descripción, las características y las notas para la certificación de `descripcionStore_es.md`, `descripcionStore_en.md` y `descripcionStore_zh.md` (ya actualizadas a la v0.5.0).
3. **Capturas:** conviene añadir una de la caja negra (botón y panel de grabación) y renovar las que muestren la versión anterior.
4. **Enviar a certificación.**

## 9. Envío de la v0.5.1 (2026-11-12)

> ⚠️ **REQ-90 sigue abierto**, igual que en la v0.5.0: la matriz de 12 casos de ropa con la app instalada no se ha ejecutado. En vivo solo se ha probado un vestido sobre `f_med_nrw` (Victoria). Enviar es decisión de la persona responsable sabiendo que ese criterio sigue sin cumplirse.

**Paquete:** `src-tauri/target/msix/dbv-unreal-ai_0.5.1.0.msixbundle` (generado con `npm run tauri:windows:build`). Identidad comprobada: `davidbuenov.DBVUnrealAI`, `CN=13EE2A5D-F49E-48C9-8873-941069B15D63`, `Version="0.5.1.0"`, `x64`; capacidades sin cambios (`runFullTrust`, `internetClient`).

**Fichas:** `descripcionStore_es.md`, `descripcionStore_en.md` y `descripcionStore_zh.md` tienen ahora cada campo en su propio bloque de código (las 20 características y las 7 palabras clave, una por bloque), con el número de caracteres y el límite del formulario al lado. Abiertas en DBV MD Reader, se copian con el botón del bloque. Cambian *Novedades de esta versión*, la característica de MetaHumans de la descripción y una nota nueva para la certificación (vestir crea assets en el proyecto del usuario y una copia del Blueprint, todo local).

**Pasos en Partner Center:**
1. **Paquetes:** subir `dbv-unreal-ai_0.5.1.0.msixbundle` y retirar del envío el anterior.
2. **Fichas de Store** (español, inglés y chino): pegar *Novedades de esta versión* y la *Descripción*. El resto de campos no cambia, salvo que se quiera repasar.
3. **Notas para la certificación:** pegar las de `descripcionStore_en.md` (un solo campo por envío).
4. **Capturas (opcional):** una del vestido sobre un MetaHuman.
5. **Enviar a certificación.**
