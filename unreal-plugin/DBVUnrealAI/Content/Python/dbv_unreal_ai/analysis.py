# =============================================================================
# DBV Unreal AI — Análisis y ejecución de Python para el toolset de Unreal
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
"""Lógica pura del toolset: no importa ``unreal`` y se prueba fuera del editor.

Recibe el módulo ``unreal`` (o un sustituto en los tests) como parámetro.

- ``validate_code`` (@spec REQ-26): analiza el código como DATO —nunca lo ejecuta— y
  comprueba cada referencia ``unreal.Clase.miembro`` contra el motor en marcha.
- ``describe_name`` (@spec REQ-25): referencia de la API sacada del propio motor.
- ``run_code`` (@spec REQ-06): ejecuta el código capturando stdout, stderr y avisos.
"""

from __future__ import annotations

import ast
import contextlib
import difflib
import io
import re
import traceback
import warnings
from dataclasses import dataclass, field
from typing import Any, Callable, Optional

#: Tope de lo que se devuelve de stdout/stderr (caracteres).
OUTPUT_LIMIT = 20000
#: Tope de miembros listados al describir una clase.
MEMBER_LIMIT = 200


@dataclass(slots=True)
class Finding:
    """Un problema encontrado en el código analizado."""

    kind: str
    line: int
    name: str
    message: str
    suggestions: list[str] = field(default_factory=list)


def _clip(text: str, limit: int = OUTPUT_LIMIT) -> str:
    """Recorta ``text`` a ``limit`` caracteres avisando del recorte."""
    result = text
    if len(text) > limit:
        result = f"{text[:limit]}\n… (truncated: {len(text)} characters)"
    return result


def is_deprecated(obj: Any) -> bool:
    """¿La documentación del objeto lo marca como obsoleto? (Unreal lo indica en el docstring)."""
    doc = getattr(obj, "__doc__", None) or ""
    return "deprecated" in doc.lower()


def _deprecation_note(obj: Any) -> str:
    """La frase del docstring que explica la obsolescencia (suele traer la alternativa)."""
    doc = getattr(obj, "__doc__", None) or ""
    lines = [line.strip() for line in doc.splitlines() if "deprecated" in line.lower()]
    result = lines[0] if lines else "deprecated"
    return result[:300]


def subsystem_alternatives(name: str, module: Any, fuzzy: bool = False) -> list[str]:
    """Llamadas equivalentes en los subsistemas del editor para un miembro obsoleto.

    Unreal 5.8 marca obsoleto ``EditorLevelLibrary`` («Use the function in Unreal Editor Subsystem»)
    sin decir cuál; casi siempre es un subsistema con un miembro del MISMO nombre
    (``get_editor_world`` → ``UnrealEditorSubsystem``, ``get_all_level_actors`` → ``EditorActorSubsystem``,
    comprobado en vivo). Sin esto, un modelo de 8B no encontraba la alternativa (eval del 2026-10-24).
    """
    exact: list[str] = []
    approximate: list[str] = []
    for candidate in sorted(n for n in dir(module) if n.endswith("Subsystem") and "Editor" in n):
        owner = getattr(module, candidate, None)
        if not isinstance(owner, type):
            continue
        if hasattr(owner, name):
            exact.append(f"unreal.get_editor_subsystem(unreal.{candidate}).{name}()")
        elif fuzzy:
            close = difflib.get_close_matches(name, [m for m in dir(owner) if not m.startswith("_")], n=1, cutoff=0.6)
            approximate.extend(f"unreal.get_editor_subsystem(unreal.{candidate}).{m}()" for m in close)
    return [*exact, *approximate][:3]


def _suggestions(name: str, candidates: list[str], module: Any, in_class: bool = True) -> list[str]:
    """Qué probar en lugar de ``name``: dónde se MOVIÓ (subsistemas del editor) y luego nombres parecidos.

    Los modelos usan nombres de la API antigua (``EditorLevelLibrary.get_current_level``) que en 5.8 viven en un
    subsistema (``LevelEditorSubsystem``, comprobado en vivo). Ofrecer solo nombres parecidos de la clase obsoleta
    llevó a un modelo de 8B hasta ``save_current_level``, que además modifica el proyecto (eval del 2026-10-24).
    """
    # Un miembro de clase que falta puede haberse movido a un subsistema (aunque con otro nombre: `get_all_actors` →
    # `get_all_level_actors`); un nombre de primer nivel (`EditorAsetLibrary`) no tiene sentido buscarlo allí.
    moved = subsystem_alternatives(name, module, fuzzy=True) if in_class else []
    similar = [n for n in difflib.get_close_matches(name, candidates, n=3) if n not in moved]
    return [*moved, *similar]


def _aliases(tree: ast.AST) -> tuple[set[str], dict[str, str]]:
    """Nombres con los que el código se refiere a ``unreal`` y a sus miembros importados.

    Devuelve (alias del módulo, {nombre local: nombre en unreal}).
    """
    module_aliases: set[str] = set()
    imported: dict[str, str] = {}
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            for alias in node.names:
                if alias.name == "unreal":
                    module_aliases.add(alias.asname or "unreal")
        elif isinstance(node, ast.ImportFrom) and node.module == "unreal":
            for alias in node.names:
                if alias.name != "*":
                    imported[alias.asname or alias.name] = alias.name
    return module_aliases, imported


def _chain(node: ast.Attribute) -> Optional[tuple[str, list[str]]]:
    """``a.b.c`` → ("a", ["b", "c"]); ``None`` si la cadena no empieza en un nombre."""
    parts: list[str] = []
    current: ast.AST = node
    while isinstance(current, ast.Attribute):
        parts.append(current.attr)
        current = current.value
    result = (current.id, list(reversed(parts))) if isinstance(current, ast.Name) else None
    return result


def _resolve(module: Any, start: str, parts: list[str], line: int) -> Optional[Finding]:
    """Recorre ``unreal.<start>.<parts…>`` y devuelve el primer problema, si lo hay."""
    finding: Optional[Finding] = None
    current = module
    path = ["unreal"]
    for name in [start, *parts]:
        if current is None:
            break
        nxt = getattr(current, name, None)
        dotted = ".".join([*path, name])
        if nxt is None and not hasattr(current, name):
            candidates = [n for n in dir(current) if not n.startswith("_")]
            finding = Finding("unknown", line, dotted, f"{dotted} does not exist in this engine version", _suggestions(name, candidates, module, current is not module))
            break
        if is_deprecated(nxt):
            finding = Finding("deprecated", line, dotted, f"{dotted}: {_deprecation_note(nxt)}", subsystem_alternatives(name, module))
            break
        # Más allá de una clase (instancias, valores devueltos) ya no se puede resolver sin ejecutar.
        if not isinstance(nxt, type) and getattr(nxt, "__module__", None) not in (None, "unreal", "_unreal_core"):
            current = None
        else:
            current = nxt
        path.append(name)
    return finding


#: Subsistemas del editor que más se necesitan, para sugerir cuando no se sabe qué método se quería.
_COMMON_SUBSYSTEMS = ("EditorActorSubsystem", "LevelEditorSubsystem", "UnrealEditorSubsystem", "EditorAssetSubsystem")


def _is_get_editor_subsystem(func: ast.AST, module_aliases: set[str], imported: dict[str, str]) -> bool:
    """¿``func`` es ``unreal.get_editor_subsystem`` (o su ``from unreal import``)?"""
    chain = _chain(func) if isinstance(func, ast.Attribute) else None
    named = isinstance(func, ast.Name) and imported.get(func.id) == "get_editor_subsystem"
    return named or bool(chain and chain[0] in module_aliases and chain[1] == ["get_editor_subsystem"])


def _is_editor_subsystem(name: str, module: Any) -> bool:
    """¿``unreal.<name>`` es una clase de subsistema del editor?"""
    target = getattr(module, name, None)
    base = getattr(module, "EditorSubsystem", None)
    result = isinstance(target, type) and (issubclass(target, base) if isinstance(base, type) else name.endswith("Subsystem"))
    return result


def _subsystem_misuse(tree: ast.AST, module: Any, module_aliases: set[str], imported: dict[str, str]) -> list[Finding]:
    """``get_editor_subsystem(unreal.X)`` con una ``X`` que NO es un subsistema.

    Pasa la validación de nombres (los dos existen) y falla al ejecutar con un ``TypeError``. Es el error más repetido
    de los modelos con esta API: una eval con ``qwen3:8b`` lo repitió tres veces seguidas ignorando el traceback.
    """
    parents = {id(child): parent for parent in ast.walk(tree) for child in ast.iter_child_nodes(parent)}
    findings: list[Finding] = []
    for node in ast.walk(tree):
        if not (isinstance(node, ast.Call) and node.args and _is_get_editor_subsystem(node.func, module_aliases, imported)):
            continue
        chain = _chain(node.args[0]) if isinstance(node.args[0], ast.Attribute) else None
        if not chain or chain[0] not in module_aliases or len(chain[1]) != 1:
            continue
        name = chain[1][0]
        if not hasattr(module, name) or _is_editor_subsystem(name, module):
            continue
        parent = parents.get(id(node))
        method = parent.attr if isinstance(parent, ast.Attribute) else ""
        options = subsystem_alternatives(method, module) if method else []
        options = options or [f"unreal.{n}" for n in _COMMON_SUBSYSTEMS if hasattr(module, n)]
        message = f"unreal.{name} is not an editor subsystem: get_editor_subsystem() needs a subsystem class such as unreal.EditorActorSubsystem, unreal.LevelEditorSubsystem or unreal.UnrealEditorSubsystem"
        findings.append(Finding("misuse", getattr(node, "lineno", 0), f"unreal.get_editor_subsystem(unreal.{name})", message, options))
    return findings


def _parse(code: str) -> tuple[Optional[ast.AST], Optional[dict[str, Any]]]:
    """Árbol del código, o ``None`` y el error de sintaxis."""
    result: tuple[Optional[ast.AST], Optional[dict[str, Any]]]
    try:
        result = (ast.parse(code), None)
    except SyntaxError as error:
        result = (None, {"line": error.lineno or 0, "message": error.msg})
    return result


def validate_code(code: str, module: Any) -> dict[str, Any]:
    """Analiza ``code`` sin ejecutarlo. @spec REQ-26

    Returns:
        ``{"ok": bool, "syntax_error": {...}|None, "findings": [...], "checked": n}``. ``ok`` es falso con un
        nombre inexistente (``unknown``) o un uso que fallaría al ejecutar (``misuse``); lo obsoleto solo avisa.
    """
    if not isinstance(code, str) or not code.strip():
        return {"ok": False, "syntax_error": {"line": 0, "message": "empty code"}, "findings": [], "checked": 0}
    tree, syntax_error = _parse(code)
    if tree is None:
        return {"ok": False, "syntax_error": syntax_error, "findings": [], "checked": 0}
    module_aliases, imported = _aliases(tree)
    # Solo la cadena más larga (`unreal.A.b`): sus tramos interiores (`unreal.A`) ya se recorren al resolverla.
    inner = {id(node.value) for node in ast.walk(tree) if isinstance(node, ast.Attribute) and isinstance(node.value, ast.Attribute)}
    findings: list[Finding] = []
    seen: set[tuple[str, int]] = set()
    checked = 0
    for node in ast.walk(tree):
        target: Optional[tuple[str, list[str]]] = None
        if isinstance(node, ast.Attribute) and id(node) not in inner and not isinstance(getattr(node, "ctx", None), ast.Store):
            chain = _chain(node)
            if chain and chain[0] in module_aliases and chain[1]:
                target = (chain[1][0], chain[1][1:])
            elif chain and chain[0] in imported:
                target = (imported[chain[0]], chain[1])
        elif isinstance(node, ast.ImportFrom) and node.module == "unreal":
            for alias in node.names:
                if alias.name != "*" and (alias.name, node.lineno) not in seen:
                    seen.add((alias.name, node.lineno))
                    checked += 1
                    problem = _resolve(module, alias.name, [], node.lineno)
                    if problem:
                        findings.append(problem)
        if target is None:
            continue
        key = (".".join([target[0], *target[1]]), getattr(node, "lineno", 0))
        if key in seen:
            continue
        seen.add(key)
        checked += 1
        problem = _resolve(module, target[0], target[1], getattr(node, "lineno", 0))
        if problem and not any(f.name == problem.name and f.line == problem.line for f in findings):
            findings.append(problem)
    findings.extend(_subsystem_misuse(tree, module, module_aliases, imported))
    findings.sort(key=lambda f: (f.line, f.name))
    result = {
        "ok": not any(f.kind in ("unknown", "misuse") for f in findings),
        "syntax_error": None,
        "findings": [{"kind": f.kind, "line": f.line, "name": f.name, "message": f.message, "suggestions": f.suggestions} for f in findings],
        "checked": checked,
    }
    return result


def describe_name(name: str, module: Any) -> dict[str, Any]:
    """Referencia de ``unreal.<name>`` sacada del motor en marcha. @spec REQ-25"""
    clean = (name or "").strip().split("(")[0].strip().rstrip(".").removeprefix("unreal.")
    if not clean:
        return {"exists": False, "name": name, "message": "empty name"}
    current: Any = module
    walked: list[str] = []
    missing: Optional[str] = None
    for part in clean.split("."):
        if not hasattr(current, part):
            missing = part
            break
        current = getattr(current, part)
        walked.append(part)
    if missing is not None:
        candidates = [n for n in dir(current) if not n.startswith("_")]
        result = {
            "exists": False,
            "name": ".".join(["unreal", *walked, missing]),
            "message": "not found in this engine version",
            "suggestions": _suggestions(missing, candidates, module, bool(walked))[:6],
        }
    else:
        doc = getattr(current, "__doc__", None) or ""
        kind = "class" if isinstance(current, type) else ("callable" if callable(current) else type(current).__name__)
        members = sorted(n for n in dir(current) if not n.startswith("_"))[:MEMBER_LIMIT] if isinstance(current, type) else []
        deprecated = is_deprecated(current)
        result = {
            "exists": True,
            "name": "unreal." + ".".join(walked),
            "kind": kind,
            "deprecated": deprecated,
            "alternatives": subsystem_alternatives(walked[-1], module) if deprecated else [],
            "doc": _clip(doc, 3000),
            "members": members,
        }
    return result


_ATTRIBUTE_ERROR = re.compile(r"'(\w+)' object has no attribute '(\w+)'")


def runtime_hint(error: BaseException, module: Any) -> Optional[str]:
    """Qué probar tras un ``AttributeError`` de ejecución, o ``None``.

    La validación estática no ve el tipo de lo que DEVUELVE una llamada (``world.get_all_actors()``), así que ese error solo
    aparece al ejecutar. El mensaje trae la clase y el miembro que faltan: se busca el miembro (o uno parecido) en los
    subsistemas del editor y en la propia clase. Una eval con ``qwen3:8b`` repitió ``world.get_all_actors()`` hasta agotar
    los pasos: el método existe como ``EditorActorSubsystem.get_all_level_actors``.
    """
    match = _ATTRIBUTE_ERROR.search(str(error)) if isinstance(error, AttributeError) else None
    options: list[str] = []
    if match:
        cls, attr = match.groups()
        owner = getattr(module, cls, None)
        similar = difflib.get_close_matches(attr, [n for n in dir(owner) if not n.startswith("_")], n=3) if isinstance(owner, type) else []
        options = [*subsystem_alternatives(attr, module, fuzzy=True), *[f"{cls}.{n}" for n in similar]]
    return f"unreal.{match.group(1)} has no '{match.group(2)}'. Try: " + "; ".join(options) if match and options else None


def _compile_with_tail(code: str) -> tuple[Any, Any]:
    """Compila ``code`` separando la última expresión suelta (``static_mesh_actor_count``), si la hay.

    ``exec`` no devuelve el valor de una expresión final, como sí hace una consola. Casi todos los modelos escriben así
    el resultado, y el código se ejecutaba bien pero llegaba ``result: null``: el modelo creía que había fallado (eval
    con ``qwen3:8b``, 2026-10-24). Devuelve (cuerpo, expresión final o ``None``).
    """
    tree = ast.parse(code, "<dbv-unreal-ai>")
    tail = ast.Expression(tree.body.pop().value) if tree.body and isinstance(tree.body[-1], ast.Expr) else None
    return compile(tree, "<dbv-unreal-ai>", "exec"), (compile(tail, "<dbv-unreal-ai>", "eval") if tail else None)


def run_code(code: str, module: Any, transaction: Optional[Callable[[], Any]] = None) -> dict[str, Any]:
    """Ejecuta ``code`` capturando su salida. @spec REQ-06

    El código se ejecuta en un espacio de nombres nuevo con ``unreal`` ya importado. Se devuelve en ``result`` la variable
    ``result`` si el código la define, o el valor de su última expresión (como en una consola). ``transaction`` crea el
    contexto de deshacer del editor (``unreal.ScopedEditorTransaction``).
    """
    if not isinstance(code, str) or not code.strip():
        return {"success": False, "stdout": "", "stderr": "", "warnings": [], "error": "empty code", "result": None}
    stdout, stderr = io.StringIO(), io.StringIO()
    namespace: dict[str, Any] = {"__name__": "__dbv_unreal_ai__", "unreal": module}
    error: Optional[str] = None
    hint: Optional[str] = None
    caught: list[str] = []
    scope = transaction() if transaction else contextlib.nullcontext()
    with warnings.catch_warnings(record=True) as recorded:
        warnings.simplefilter("always")
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            try:
                body, tail = _compile_with_tail(code)
                with scope:
                    exec(body, namespace)  # noqa: S102 — ejecutar es la herramienta (REQ-06, aprobada en REQ-12)
                    if tail is not None:
                        last = eval(tail, namespace)  # noqa: S307 — la última expresión del mismo código
                        namespace["result"] = namespace.get("result") if namespace.get("result") is not None else last
            except Exception as failure:  # noqa: BLE001 — frontera: cualquier fallo del código del usuario se devuelve como dato
                error = traceback.format_exc(limit=8)
                hint = runtime_hint(failure, module)
        caught = [f"{w.category.__name__}: {w.message} (line {w.lineno})" for w in recorded]
    value = namespace.get("result")
    if value is not None and not isinstance(value, (str, int, float, bool, list, dict)):
        value = repr(value)
    result = {
        "success": error is None,
        "stdout": _clip(stdout.getvalue()),
        "stderr": _clip(stderr.getvalue()),
        "warnings": caught[:50],
        "error": error,
        "hint": hint,
        "result": value,
    }
    return result
