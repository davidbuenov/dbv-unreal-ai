# Arquitectura de Gaussian Splatting Nativo en DBV Unreal AI
## Native 3D Gaussian Splatting Architecture for Unreal Engine 5.8

<!--
Copyright (c) 2026 David Bueno Vallejo & David Bueno Carmona. All rights reserved.
Licensed under the MIT License. See LICENSE in the project root for license information.
Developed using dbv-specs-ops methodology.
-->

## 1. Motivación y Decisión de Diseño (ADR) / Motivation & Design Decision

### Contexto
Durante las pruebas de visualización fotorrealista de mundos de World Labs (`.spz`, ~1.92M gaussianas) en Unreal Engine 5.8, se comprobó que el plugin de terceros XGRIDS LCC mostraba diálogos modales de licencia comercial exigiendo una clave de pago ("Pro" / `AppKey`).

### Decisión
1. **Rechazo total de dependencias comerciales privativas con muros de pago:** No se integrará ni distribuirá software con restricciones de pago o binarios cerrados.
2. **Desarrollo de un sistema nativo 100% de código abierto (MIT):** Implementar dentro del módulo `DBVUnrealAICpp` de `Plugins/DBVUnrealAI` nuestro propio componente de renderizado y carga de Gaussian Splatting: `UDbvGaussianSplatComponent` y `ADbvGaussianSplatActor`.
3. **Aprovechamiento de las lecciones de YaGS (Apache 2.0):** Estudiar y adoptar los principios de arquitectura de renderizado descubiertos por proyectos abiertos punteros (especialmente YaGS de Yandex) sin copiar código, implementando oclusión mutua por búfer de profundidad, ordenación eficiente e instanciación ligera.

---

## 2. Lecciones Arquitectónicas Clave de YaGS / Key Lessons from YaGS

### Lección 1: Escritura de Profundidad Real (`AlphaDepthClip`)
* **Problema tradicional:** La mayoría de renderizadores de 3DGS dibujan splats como partículas o billboards puramente translúcidos en una pasada posterior o como overlay de pantalla. Esto causa que mallas poligonales estándar (personajes, objetos dinámicos) se dibujen detrás o delante erróneamente sin oclusión geométrica real.
* **Solución YaGS:** Escritura directa en el búfer de profundidad de la escena (`SceneDepthPass`) gobernada por un umbral de corte alfa configurable (`AlphaDepthThreshold`, típicamente ~0.1f). El núcleo denso de cada gaussiana escribe profundidad geométrica real, permitiendo:
  - Que los personajes y objetos del juego queden correctamente ocultos detrás de colinas, árboles o paredes del splat.
  - Que los splats queden ocluidos por la geometría opaca del nivel.
  - Compatibilidad nativa con efectos de postprocesado dependientes de profundidad de Unreal Engine (TAA / TSR, Motion Blur, Depth of Field).

### Lección 2: Pipeline Dual (Mesh Shader + Vertex Shader Fallback)
* **Mesh Shader Path:** En GPUs modernas (DirectX 12 / Vulkan), un *Amplification Shader* realiza frustum culling y cluster culling a nivel de grupo de gaussianas. El *Mesh Shader* genera los quads directamente en la GPU sin estrangular el Input Assembler (IA) de la CPU.
* **Vertex Shader Path:** Para GPUs sin soporte de Mesh Shaders, se utiliza instanciación de quads (`DrawIndexedPrimitiveIndirect`) con búferes de instancias dinámicos.

### Lección 3: Instanciación Ligera y `argsort`
* Los datos geométricos brutos de las gaussianas (posiciones, covarianzas 3D, armónicos esféricos, opacidades base) se mantienen en búferes GPU compartidos una sola vez en VRAM.
* Cada instancia o actor mantiene únicamente su transformación de coordenadas (Local-to-World) y su propio arreglo de índices ordenados por profundidad (`argsort`), evitando duplicar gigabytes de VRAM al instanciar mundos o fragmentos.

### Lección 4: Ordenación por Profundidad en GPU (Radix Sort)
* El Gaussian Splatting requiere blending de adelante hacia atrás o de atrás hacia adelante (algoritmo del pintor).
* Calcular las claves de profundidad proyectadas en espacio de cámara y ordenarlas mediante un Radix Sort paralelo en GPU garantiza 60+ FPS incluso con 2–4 millones de gaussianas activas.

---

## 3. Especificación del Formato Binario SPZ / SPZ Format Specification

El formato `.spz` (creado por Scaniverse / Niantic y adoptado por World Labs) es un formato ultra-compacto comprimido con zlib/deflate:

### Estructura de Cabecera (Header)
```
Magic: 0x50535053 ("SPSP") o stream zlib
Version: uint32 (1 o 2)
NumPoints: uint32 (número de gaussianas)
SHDegree: uint8 (0 a 3)
FractionalBits: uint8 (precisión de cuantización de posición)
Flags: uint8
Reserved: uint8
```

### Datos Cuantizados
1. **Posiciones (24-bit fixed-point):** Cada coordenada $(x, y, z)$ se almacena como entero de 24 bits escalado por $2^{\text{fractionalBits}}$.
   $$\text{Posición} = \frac{\text{valor\_entero}}{2^{\text{fractionalBits}}}$$
2. **Escalas (8-bit log-space):** Las 3 componentes de escala se almacenan cuantizadas en espacio logarítmico.
   $$\text{Scale} = \exp\left(\frac{\text{raw} \cdot \text{scale\_range}}{255} + \text{scale\_min}\right)$$
3. **Rotaciones (8-bit normalized quaternions):** Cuaternión $(q_w, q_x, q_y, q_z)$ empaquetado en 4 bytes normalizados ($[-1.0, 1.0]$).
4. **Opacidad (8-bit sigmoid / logit):** Valor $\alpha \in [0, 255]$ transformado mediante función logística sigmoide:
   $$\alpha = \frac{1}{1 + \exp(-\text{logit})}$$
5. **Armónicos Esféricos (Spherical Harmonics):**
   - Grado 0 (DC): 3 bytes (R, G, B) normalizados con el factor base $C_0 \approx 0.28209479$.
   - Grados 1–3: coeficientes de color dependientes de la dirección de la cámara cuantizados en 8 bits.

---

## 4. Transformación de Coordenadas (OpenCV → Unreal Engine)

Los archivos `.spz` generados por World Labs utilizan el sistema de coordenadas de OpenCV / Computer Vision:
* **OpenCV:** $+X$ = Derecha, $+Y$ = Abajo, $+Z$ = Hacia adelante.
* **Unreal Engine:** $+X$ = Hacia adelante, $+Y$ = Derecha, $+Z$ = Hacia arriba.

### Regla de Transformación Verificada en UE 5.8:
* **Rotación del Actor:** `Roll = 90.0°`, `Pitch = 0.0°`, `Yaw = 0.0°` (En Unreal: `FRotator(0.0f, 0.0f, 90.0f)`).
* **Escala del Actor:** `100.0, 100.0, 100.0` (1 unidad OpenCV = 1 metro = 100 cm Unreal).
* **Posición:** `(0, 0, 0)`.

---

## 5. Arquitectura de Clases C++ en `DBVUnrealAICpp`

```
Plugins/DBVUnrealAI/Source/DBVUnrealAICpp/
├── Public/
│   ├── DbvGaussianSplatTypes.h         // Estructuras de datos, cabeceras y constantes
│   ├── DbvSpzReader.h                  // Parser rápido de archivos .spz (zlib decompression)
│   ├── DbvPlyReader.h                  // Parser de archivos .ply binarios estándar
│   ├── DbvGaussianSplatAsset.h         // UObject con datos de splat y buffers de GPU
│   ├── DbvGaussianSplatComponent.h     // UMeshComponent / UPrimitiveComponent
│   ├── DbvGaussianSplatActor.h         // AActor para colocación en escena y llamadas MCP
│   └── DbvGaussianSplatSceneProxy.h    // FPrimitiveSceneProxy para renderizado en Render Thread
└── Private/
    ├── DbvSpzReader.cpp
    ├── DbvPlyReader.cpp
    ├── DbvGaussianSplatAsset.cpp
    ├── DbvGaussianSplatComponent.cpp
    ├── DbvGaussianSplatActor.cpp
    └── DbvGaussianSplatSceneProxy.cpp
```
