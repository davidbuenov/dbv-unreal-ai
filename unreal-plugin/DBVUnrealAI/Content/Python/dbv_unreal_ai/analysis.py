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


def subsystem_alternatives(name: str, module: Any) -> list[str]:
    """Llamadas equivalentes en los subsistemas del editor para un miembro obsoleto.

    Unreal 5.8 marca obsoleto ``EditorLevelLibrary`` («Use the function in Unreal Editor Subsystem»)
    sin decir cuál; casi siempre es un subsistema con un miembro del MISMO nombre
    (``get_editor_world`` → ``UnrealEditorSubsystem``, ``get_all_level_actors`` → ``EditorActorSubsystem``,
    comprobado en vivo). Sin esto, un modelo de 8B no encontraba la alternativa (eval del 2026-10-24).
    """
    found: list[str] = []
    for candidate in sorted(n for n in dir(module) if n.endswith("Subsystem") and "Editor" in n):
        owner = getattr(module, candidate, None)
        if isinstance(owner, type) and hasattr(owner, name):
            found.append(f"unreal.get_editor_subsystem(unreal.{candidate}).{name}()")
    return found[:3]


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
            finding = Finding("unknown", line, dotted, f"{dotted} does not exist in this engine version", difflib.get_close_matches(name, candidates, n=3))
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
        ``{"ok": bool, "syntax_error": {...}|None, "findings": [...], "checked": n}``.
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
    findings.sort(key=lambda f: (f.line, f.name))
    result = {
        "ok": not any(f.kind == "unknown" for f in findings),
        "syntax_error": None,
        "findings": [{"kind": f.kind, "line": f.line, "name": f.name, "message": f.message, "suggestions": f.suggestions} for f in findings],
        "checked": checked,
    }
    return result


def describe_name(name: str, module: Any) -> dict[str, Any]:
    """Referencia de ``unreal.<name>`` sacada del motor en marcha. @spec REQ-25"""
    clean = (name or "").strip().removeprefix("unreal.")
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
            "suggestions": difflib.get_close_matches(missing, candidates, n=5),
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


def run_code(code: str, module: Any, transaction: Optional[Callable[[], Any]] = None) -> dict[str, Any]:
    """Ejecuta ``code`` capturando su salida. @spec REQ-06

    El código se ejecuta en un espacio de nombres nuevo con ``unreal`` ya importado. Si
    define una variable ``result`` serializable, se devuelve en ``result``. ``transaction``
    crea el contexto de deshacer del editor (``unreal.ScopedEditorTransaction``).
    """
    if not isinstance(code, str) or not code.strip():
        return {"success": False, "stdout": "", "stderr": "", "warnings": [], "error": "empty code", "result": None}
    stdout, stderr = io.StringIO(), io.StringIO()
    namespace: dict[str, Any] = {"__name__": "__dbv_unreal_ai__", "unreal": module}
    error: Optional[str] = None
    caught: list[str] = []
    scope = transaction() if transaction else contextlib.nullcontext()
    with warnings.catch_warnings(record=True) as recorded:
        warnings.simplefilter("always")
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            try:
                with scope:
                    exec(compile(code, "<dbv-unreal-ai>", "exec"), namespace)  # noqa: S102 — ejecutar es la herramienta (REQ-06, aprobada en REQ-12)
            except Exception:  # noqa: BLE001 — frontera: cualquier fallo del código del usuario se devuelve como dato
                error = traceback.format_exc(limit=8)
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
        "result": value,
    }
    return result
