# 🏬 Publicación en Microsoft Store: DBV Unreal AI

> **Estado:** 📦 Paquete de la `v0.2.3` generado y listo para subir a Partner Center (§4).  
> **Versión base:** `v0.2.3`  
> **Última revisión:** 2026-10-29  

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
   - Copiar textos de `descripcionStore_es.md` (Español) y `descripcionStore_en.md` (Inglés).
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
