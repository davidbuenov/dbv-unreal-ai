// =============================================================================
// DBV Unreal AI — Visor 3D ligero de los assets de la biblioteca
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-31
//
// Ver el modelo antes de llevarlo a Unreal: órbita, zoom, giro automático y, si el GLB trae animaciones (un
// rig animado de Tripo), su lista para reproducirlas. Three.js 0.180 va copiado en `src/vendor/three/` (sin
// red, RNF-01) y se carga solo al abrir un visor (D12). Un WebView tiene pocos contextos WebGL: quien crea un
// visor debe llamar a `dispose()` al cerrarlo.

/** Distancia de cámara para encuadrar una esfera de radio `radius` con un campo vertical `fovDegrees`. */
export function frameDistance(radius, fovDegrees) {
  const half = (Math.max(1, Math.min(170, fovDegrees)) * Math.PI) / 360;
  return Math.max(0.01, radius) / Math.sin(half) * 1.15;
}

/** Nombre legible de un clip de Tripo (`preset:biped:dance_01` → `dance 01`). */
export const clipLabel = (name) => String(name ?? '').split(':').pop().replace(/_/g, ' ') || 'clip';

let threeModules = null;

/** Three.js y sus complementos, una sola vez. */
function loadThree() {
  threeModules ??= Promise.all([import('three'), import('../vendor/three/examples/jsm/loaders/GLTFLoader.js'), import('../vendor/three/examples/jsm/controls/OrbitControls.js')]).then(([THREE, gltf, orbit]) => ({ THREE, GLTFLoader: gltf.GLTFLoader, OrbitControls: orbit.OrbitControls }));
  return threeModules;
}

/**
 * Monta un visor en `container` con el modelo de `url`.
 * @param {HTMLElement} container
 * @param {string} url Enlace `asset:` del GLB/glTF (convertFileSrc).
 * @param {{autoRotate?: boolean, background?: string}} [options]
 * @returns {Promise<{dispose(): void, clips: string[], play(name: string): void, snapshot(size?: number): Promise<Uint8Array|null>}>}
 */
export async function createModelViewer(container, url, options = {}) {
  const { THREE, GLTFLoader, OrbitControls } = await loadThree();
  const width = Math.max(1, container.clientWidth || 320);
  const height = Math.max(1, container.clientHeight || 240);
  // `preserveDrawingBuffer`: sin él la captura de la miniatura sale en negro.
  const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true, preserveDrawingBuffer: true });
  renderer.setPixelRatio(Math.min(2, window.devicePixelRatio || 1));
  renderer.setSize(width, height);
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  container.replaceChildren(renderer.domElement);

  const scene = new THREE.Scene();
  scene.add(new THREE.HemisphereLight(0xffffff, 0x444455, 2.2));
  const sun = new THREE.DirectionalLight(0xffffff, 2.4);
  sun.position.set(3, 5, 4);
  scene.add(sun);
  const camera = new THREE.PerspectiveCamera(40, width / height, 0.01, 1000);
  const controls = new OrbitControls(camera, renderer.domElement);
  controls.enableDamping = true;
  controls.autoRotate = options.autoRotate !== false;
  controls.autoRotateSpeed = 1.5;

  const gltf = await new GLTFLoader().loadAsync(url);
  const model = gltf.scene;
  scene.add(model);
  // Encuadre: el modelo centrado y la cámara a la distancia que lo abarca, un poco por encima.
  const box = new THREE.Box3().setFromObject(model);
  const sphere = box.getBoundingSphere(new THREE.Sphere());
  model.position.sub(sphere.center);
  const distance = frameDistance(sphere.radius, camera.fov);
  camera.position.set(distance * 0.6, distance * 0.35, distance * 0.75);
  camera.near = distance / 100;
  camera.far = distance * 100;
  camera.updateProjectionMatrix();
  controls.target.set(0, 0, 0);
  controls.update();

  const mixer = gltf.animations.length ? new THREE.AnimationMixer(model) : null;
  const clips = gltf.animations.map((clip) => clip.name);
  let action = null;
  function play(name) {
    const clip = gltf.animations.find((c) => c.name === name);
    if (!mixer || !clip) return;
    action?.stop();
    action = mixer.clipAction(clip);
    action.reset().play();
  }
  if (clips.length) play(clips[0]);

  const clock = new THREE.Clock();
  let frame = 0;
  let alive = true;
  function loop() {
    if (!alive) return;
    frame = requestAnimationFrame(loop);
    mixer?.update(clock.getDelta());
    controls.update();
    renderer.render(scene, camera);
  }
  loop();

  const resize = new ResizeObserver(() => {
    const w = Math.max(1, container.clientWidth);
    const h = Math.max(1, container.clientHeight);
    renderer.setSize(w, h);
    camera.aspect = w / h;
    camera.updateProjectionMatrix();
  });
  resize.observe(container);

  return {
    clips,
    play,
    /** PNG cuadrado del encuadre actual (para la miniatura de un asset que no la traía). */
    async snapshot(size = 256) {
      renderer.render(scene, camera);
      const source = renderer.domElement;
      const canvas = document.createElement('canvas');
      canvas.width = size;
      canvas.height = size;
      const side = Math.min(source.width, source.height);
      canvas.getContext('2d').drawImage(source, (source.width - side) / 2, (source.height - side) / 2, side, side, 0, 0, size, size);
      const blob = await new Promise((resolve) => canvas.toBlob(resolve, 'image/png'));
      return blob ? new Uint8Array(await blob.arrayBuffer()) : null;
    },
    dispose() {
      alive = false;
      cancelAnimationFrame(frame);
      resize.disconnect();
      controls.dispose();
      mixer?.stopAllAction();
      scene.traverse((node) => {
        node.geometry?.dispose?.();
        for (const material of [node.material].flat().filter(Boolean)) {
          for (const value of Object.values(material)) value?.isTexture && value.dispose();
          material.dispose?.();
        }
      });
      renderer.dispose();
      renderer.forceContextLoss?.();
      renderer.domElement.remove();
    },
  };
}

/**
 * Renderiza una miniatura de un modelo 3D (GLB/glTF) fuera de pantalla y devuelve el PNG en bytes.
 * @param {string} url Enlace del modelo (convertFileSrc)
 * @param {number} [size=256] Tamaño del thumbnail en píxeles.
 * @returns {Promise<Uint8Array|null>}
 */
export async function renderModelThumbnail(url, size = 256) {
  let result = null;
  try {
    const { THREE, GLTFLoader } = await loadThree();
    const canvas = document.createElement('canvas');
    canvas.width = size;
    canvas.height = size;
    const renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: true, preserveDrawingBuffer: true });
    renderer.setSize(size, size);
    renderer.outputColorSpace = THREE.SRGBColorSpace;

    const scene = new THREE.Scene();
    scene.add(new THREE.HemisphereLight(0xffffff, 0x444455, 2.2));
    const sun = new THREE.DirectionalLight(0xffffff, 2.4);
    sun.position.set(3, 5, 4);
    scene.add(sun);

    const camera = new THREE.PerspectiveCamera(40, 1, 0.01, 1000);
    const gltf = await new GLTFLoader().loadAsync(url);
    const model = gltf.scene;
    scene.add(model);

    const box = new THREE.Box3().setFromObject(model);
    const sphere = box.getBoundingSphere(new THREE.Sphere());
    model.position.sub(sphere.center);
    const distance = frameDistance(sphere.radius, camera.fov);
    camera.position.set(distance * 0.6, distance * 0.35, distance * 0.75);
    camera.near = distance / 100;
    camera.far = distance * 100;
    camera.lookAt(0, 0, 0);
    camera.updateProjectionMatrix();

    renderer.render(scene, camera);
    const blob = await new Promise((resolve) => canvas.toBlob(resolve, 'image/png'));
    if (blob) {
      result = new Uint8Array(await blob.arrayBuffer());
    }

    renderer.dispose();
    renderer.forceContextLoss?.();
    scene.traverse((node) => {
      node.geometry?.dispose?.();
      for (const material of [node.material].flat().filter(Boolean)) {
        for (const value of Object.values(material)) {
          if (value?.isTexture) value.dispose();
        }
        material.dispose?.();
      }
    });
  } catch {
    result = null;
  }
  return result;
}

