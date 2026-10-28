# 🏬 Publicación en Microsoft Store: DBV Unreal AI

> **Estado:** 📦 Preparado para empaquetado y certificación en Partner Center.  
> **Versión base:** `v0.2.1`  
> **Última revisión:** 2026-10-28  

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
