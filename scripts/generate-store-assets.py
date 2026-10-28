# =============================================================================
# DBV Unreal AI — Generador de Assets para Microsoft Store
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================

from pathlib import Path
from PIL import Image

def generate_assets():
    root = Path(__file__).resolve().parent.parent
    src_icon = root / "src-tauri" / "icons" / "icon.png"
    assets_dir = root / "src-tauri" / "gen" / "windows" / "Assets"
    assets_dir.mkdir(parents=True, exist_ok=True)

    base = Image.open(src_icon).convert("RGBA")

    # 1. Square150x150Logo.png
    sq150 = base.resize((150, 150), Image.Resampling.LANCZOS)
    sq150_path = assets_dir / "Square150x150Logo.png"
    sq150.save(sq150_path, "PNG")

    # 2. Square44x44Logo.png
    sq44 = base.resize((44, 44), Image.Resampling.LANCZOS)
    sq44_path = assets_dir / "Square44x44Logo.png"
    sq44.save(sq44_path, "PNG")

    # 3. StoreLogo.png (50x50)
    store = base.resize((50, 50), Image.Resampling.LANCZOS)
    store_path = assets_dir / "StoreLogo.png"
    store.save(store_path, "PNG")

    # 4. Wide310x150Logo.png (310x150 transparente con icono de 150x150 centrado)
    wide = Image.new("RGBA", (310, 150), (0, 0, 0, 0))
    offset_x = (310 - 150) // 2
    offset_y = 0
    wide.paste(sq150, (offset_x, offset_y), sq150)
    wide_path = assets_dir / "Wide310x150Logo.png"
    wide.save(wide_path, "PNG")

    # Verificación heurística de anti-placeholder (MARKETPLACE_PUBLISHING.md §3)
    files = [sq150_path, sq44_path, store_path, wide_path]
    print("Verificando assets generados...")
    for f in files:
        im = Image.open(f)
        colors = len(set(im.getdata()))
        print(f"  [OK] {f.name}: {im.size}, {im.mode}, {colors} colores unicos")
        if colors <= 1:
            raise RuntimeError(f"ALERTA: {f.name} parece ser un placeholder solido (solo {colors} colores).")

    print("\nTodos los assets para Microsoft Store generados y verificados con exito.")

if __name__ == "__main__":
    generate_assets()
