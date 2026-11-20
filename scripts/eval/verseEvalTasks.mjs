// =============================================================================
// DBV Unreal AI — Eval de Verse: UEFN simulado, 12 tareas y jueces
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-130 (SPECIFICATIONS.md §8, «Verse [v0.7.0]»)
//
// 12 tareas de Verse que cubren las 10 reglas de `docs/UEFN_VERSE_STUDY.md` §2. Sin UEFN, el editor es un SIMULADOR:
// `VerseToolset.WriteFile` guarda en memoria y `BuildAll` devuelve, con la forma real grabada en P-14
// (`{severity, code, message, filePath, span}`), el error típico de la tarea mientras el código no cumpla lo que la
// regla exige, y `[]` cuando lo cumple. No es un compilador: comprueba la regla de la tarea, no todo Verse. Con UEFN
// abierto (`--live`) se compila de verdad.
//
// Una tarea se da por buena SOLO si:
//   1. la última llamada a BuildAll devolvió `[]` dentro del tope de intentos (REQ-127);
//   2. no hay jugabilidad en clases `creative_device`;
//   3. no usa firmas inventadas (las de la lista de cada tarea que no están en el digest simulado);
//   4. el código nuevo no usa `/UnrealEngine.com/Temporary/SpatialMath` salvo para convertir.
// Además, ninguna respuesta puede decir que compila sin haber llamado a BuildAll (métrica que debe ser 0).
//
// Lo usan `verse-eval.mjs` (con un modelo de Ollama) y `tests/app/verseEval.test.js` (con un modelo guionizado).

import { createVerseTools } from '../../src/unreal/uefn.js';

export const MAX_ATTEMPTS = 5;

/** Digest simulado: firmas reales de UEFN 43.12 (recortadas) que `verse_api_search` puede encontrar. */
export const DIGEST = [
  'Characters<public> := module:',
  '    (InAgent:agent).GetFortCharacter<native><public>()<transacts><decides>:fort_character',
  '    fort_character<native><public> := interface<unique><epic_internal>(positional):',
  '        TeleportTo<public>(Position:(/UnrealEngine.com/Temporary/SpatialMath:)vector3, Rotation:(/UnrealEngine.com/Temporary/SpatialMath:)rotation)<transacts><decides>:void',
  '        GetTransform<public>()<reads>:(/UnrealEngine.com/Temporary/SpatialMath:)transform',
  '        EliminatedEvent<public>():listenable(elimination_result)',
  '    elimination_result<native><public> := struct<epic_internal>:',
  '        EliminatedCharacter<native><public>:fort_character',
  '        EliminatingCharacter<native><public>:?fort_character',
  'team_settings_and_inventory_device<public> := class<concrete><final>(creative_device_base):',
  '    TeamMemberSpawnedEvent<public>:listenable(agent) = external {}',
  'Playspaces<public> := module:',
  '    GetPlayspace<public>()<transacts>:fort_playspace',
  '    fort_playspace<native><public> := interface<epic_internal>:',
  '        GetPlayers<public>()<transacts>:[]player',
  'Simulation<public> := module:',
  '    Sleep<native><public>(Seconds:float)<suspends>:void',
  'SpatialMath<public> := module:',
  '    vector3<native><public> := struct<concrete><computes><persistable>:',
  '        Left<native><public>:float = external {}',
  '        Up<native><public>:float = external {}',
  '        Forward<native><public>:float = external {}',
  '    FromVector3<public>(InVector3:(/UnrealEngine.com/Temporary/SpatialMath:)vector3)<reads>:vector3',
  '    FromVector3<public>(InVector3:(/Verse.org/SpatialMath:)vector3)<reads>:(/UnrealEngine.com/Temporary/SpatialMath:)vector3',
  'Floor<native><public>(X:rational)<computes>:int',
  'Round<native><public>(Val:float)<varies><decides>:int',
  'component<native><public> := class<abstract><unique><castable>:',
  '    OnBeginSimulation<native_callable><public>():void',
  '    OnSimulate<native_callable><public>()<suspends>:void',
  'Print<native><public>(Message:string, ?Duration:float = external {}, ?Color:color = external {}):void',
];

/** Temas de la skill simulada (el contenido real es de Epic y no se incluye). */
const REFERENCE = {
  gotchas: 'Default effects are <transacts><no_rollback>. Call <decides> functions with []. int / int is rational: use Floor[]. No shadowing. for returns an array. Use /Verse.org/SpatialMath.',
  'error-codes': '3512: a function with the <decides> effect is called with () or from a context that cannot fail; add <decides><transacts> to the helper and call it with []. 3509: the value has a different type than the variable. 3502: name already defined (shadowing).',
  'scene-graph': 'Gameplay goes in components: my_component := class<final_super>(component). Override OnBeginSimulation and OnSimulate<suspends>.',
  concurrency: 'race { A(); B() } ends when the first finishes; Sleep(Seconds) suspends. loop with Sleep inside to avoid runaway loops.',
};

const diag = (code, message, line = 5) => ({ severity: 'Error', code, message, filePath: '/zz_eval/zz_task.verse', span: { startLine: line - 1, startCharacter: 4, endLine: line - 1, endCharacter: 20 } });

/** Lo común a todas: componentes, no `creative_device`. */
const COMPONENT = /class<final_super>\(component\)/;

/**
 * Las 12 tareas. `rule` dice qué regla del estudio cubre; `errors(code)` es lo que devuelve el BuildAll simulado
 * mientras el código no cumpla (vacío = compila); `invented` son firmas que no existen y que un modelo suele inventar.
 */
export const TASKS = [
  {
    id: 'kill_counter', rule: 1,
    prompt: 'Write a Scene Graph component that counts how many eliminations each player makes and prints "ZZTEST kills <n>" when it changes. Use the real API.',
    errors: (code) => [...(COMPONENT.test(code) ? [] : [diag(3502, 'Gameplay must be in a component (class<final_super>(component)).')]), ...(/EliminatedEvent/.test(code) ? [] : [diag(3506, 'Unknown identifier: the elimination event was not used.')])],
    invented: ['OnPlayerEliminated', 'GetKiller(', 'KillCount('],
  },
  {
    id: 'door_three_seconds', rule: 6,
    prompt: 'Write a Scene Graph component for a door that opens for 3 seconds and then closes again (print "ZZTEST door open" and "ZZTEST door closed"). No polling loops.',
    errors: (code) => [...(COMPONENT.test(code) ? [] : [diag(3502, 'Gameplay must be in a component.')]), ...(/Sleep\(\s*3(\.0)?\s*\)/.test(code) ? [] : [diag(3509, 'Expected Sleep(3.0) with a float.')])],
    invented: ['Wait(', 'Delay(', 'SetTimer('],
  },
  {
    id: 'cancellable_countdown', rule: 6,
    prompt: 'Write a component with a 10-second countdown that can be cancelled by an event, using structured concurrency (no flags checked in a loop). Print "ZZTEST exploded" or "ZZTEST cancelled".',
    errors: (code) => (/race\b/.test(code) && /Sleep\(/.test(code) ? [] : [diag(3512, 'Use race between Sleep and the cancel event.')]),
    invented: ['CancelTimer(', 'StopCoroutine('],
  },
  {
    id: 'average_score', rule: 3,
    prompt: 'Write a function that returns the average score (an int) of an array of int scores, and use it from a component that prints "ZZTEST average <n>".',
    errors: (code) => (/\/\s*\w+/.test(code) && !/Floor\[|Floor\(|Round\[/.test(code) ? [diag(3509, 'This variable expects a value of type int, but the initializer is of type rational.', 6)] : []),
    invented: ['ToInt(', 'IntDivide(', 'Average('],
  },
  {
    id: 'first_leader', rule: 2,
    prompt: 'Write a <decides> helper that finds the first player with more than 10 points in a map, and call it correctly from a component inside an if.',
    errors: (code) => (/<decides>/.test(code) && /<transacts>|<reads>|<computes>/.test(code) && /\w+\[[^\]]*\]/.test(code) ? [] : [diag(3511, "This call uses parentheses to call a function that has the 'decides' effect. Functions that may fail, which is indicated by the 'decides' effect, must be called with square brackets, while functions that always succeed must be called with parentheses.", 7)]),
    invented: ['FindFirst(', 'Where('],
  },
  {
    id: 'damage_multiplier', rule: 3,
    prompt: 'Write a function that multiplies an int base damage by a float multiplier and returns the rounded int result, used from a component that prints "ZZTEST damage <n>".',
    errors: (code) => (/ToFloat\(/.test(code) ? [diag(3506, 'Unknown identifier ToFloat.', 4)] : /Round\[|Floor\[|Ceil\[/.test(code) ? [] : [diag(3509, 'Expected int, found float.', 5)]),
    invented: ['ToFloat(', 'Float('],
  },
  {
    id: 'teleport_player', rule: 8,
    prompt: 'Write a component that teleports a player character 5 metres up from its current position. New code should use /Verse.org/SpatialMath and convert only where the API needs the old vector3.',
    errors: (code) => (/TeleportTo\[/.test(code) ? [] : [diag(3511, "This call uses parentheses to call a function that has the 'decides' effect. Functions that may fail must be called with square brackets.", 9)]),
    invented: ['SetPosition(', 'MoveTo(', 'SetActorLocation('],
  },
  {
    id: 'late_joiners', rule: 9,
    prompt: 'Write a component that keeps track of all players, including players who join a match in progress, using the team settings device spawn event through an @editable field.',
    errors: (code) => (/TeamMemberSpawnedEvent/.test(code) && /@editable/.test(code) ? [] : [diag(3506, 'Unknown identifier: use TeamMemberSpawnedEvent from team_settings_and_inventory_device.')]),
    invented: ['PlayerAddedEvent', 'OnPlayerJoined', 'PlayerJoinedEvent'],
  },
  {
    id: 'no_shadowing', rule: 4,
    prompt: 'In a project with a Content folder named "Score", write a component that keeps a score per player. Avoid every name clash.',
    errors: (code) => (/\bScore\s*:=|\bScore\s*:\s*(int|float)/.test(code) ? [diag(3502, 'The name Score is already defined (the Score folder is a module).', 3)] : []),
    invented: [],
  },
  {
    id: 'for_expression', rule: 5,
    prompt: 'Write a function that returns the names of the players whose score is above 5, using a for expression (no break, no continue).',
    errors: (code) => (/\bbreak\b|\bcontinue\b/.test(code) ? [diag(3500, 'break/continue are not allowed in for.', 6)] : /for\s*\(/.test(code) ? [] : [diag(3509, 'Expected a for expression that returns an array.')]),
    invented: ['Filter(', 'Map('],
  },
  {
    id: 'safe_loop', rule: 7,
    prompt: 'Write a component that prints "ZZTEST tick" every second while the game runs, without blocking the session or hitting the runtime limits.',
    errors: (code) => (/loop\s*:/.test(code) && /Sleep\(\s*1(\.0)?\s*\)/.test(code) ? [] : [diag(3512, 'An infinite loop without Sleep exceeds the runtime limit.')]),
    invented: ['Tick(', 'OnTick('],
  },
  {
    id: 'fix_3512', rule: 10,
    prompt: 'This Verse does not compile. BuildAll says: "zz_task.verse:7 · 3512 · This invocation calls a function that has the <decides> effect, which is not allowed by its context." Fix it by its code and compile again.\n\nIsHighScore(S:int):logic = if (S > 100) then true else false\nzz_check := class<final_super>(component):\n    Check(S:int):void =\n        if (IsHighScore(S)):\n            Print("ZZTEST high")',
    errors: (code) => (/IsHighScore\s*\([^)]*\)\s*<decides>|<decides>\s*<transacts>|<transacts>\s*<decides>/.test(code) && /IsHighScore\[/.test(code) ? [] : [diag(3512, 'This invocation calls a function that has the <decides> effect, which is not allowed by its context.', 7)]),
    invented: [],
  },
];

// ─── UEFN simulado ───────────────────────────────────────────────────

/** El editor simulado: ficheros en memoria, BuildAll de la tarea y registro de llamadas. */
export function createSimulatedUefn(task) {
  const files = new Map();
  const calls = [];
  const builds = [];
  const code = () => [...files.values()].join('\n');
  function call(toolset, tool, args) {
    calls.push({ toolset, tool, args });
    if (!/VerseToolset/.test(toolset)) return { ok: true, value: `error: in this evaluation only VerseToolset is available (got ${toolset}).` };
    if (tool === 'WriteFile') {
      files.set(String(args?.path ?? ''), String(args?.content ?? ''));
      return { ok: true, value: '{"returnValue":null}' };
    }
    if (tool === 'ReadFile') return { ok: true, value: JSON.stringify({ returnValue: files.get(String(args?.path ?? '')) ?? '' }) };
    if (tool === 'ListFiles') return { ok: true, value: JSON.stringify({ returnValue: [{ name: '/zz_eval', type: 'Directory' }, ...[...files.keys()].map((name) => ({ name, type: 'File' }))] }) };
    if (tool === 'BuildAll') {
      const errors = files.size ? task.errors(code()) : [diag(3000, 'No Verse file was written: write it with WriteFile first.')];
      builds.push(errors);
      return { ok: true, value: JSON.stringify({ returnValue: errors }) };
    }
    return { ok: true, value: `error: unknown VerseToolset tool ${tool}` };
  }
  return { files, calls, builds, code, call };
}

/** `verse_api_search` contra el digest simulado. */
export function searchDigest(query) {
  const wanted = String(query ?? '').toLowerCase().trim();
  const lines = wanted ? DIGEST.filter((line) => line.toLowerCase().includes(wanted)) : [];
  return lines.length ? `${lines.length} match(es) for "${query}" (digest from build ++Fortnite+Release-43.12-CL-60598086):\n${lines.map((l) => l.trim()).join('\n')}` : `No match for "${query}" in the Verse API of this project. Do not invent the signature.`;
}

/** Herramientas de la eval: las de Verse de la app (con datos simulados) y `unreal_call_tool` sobre el simulador. */
export function evalTools(task) {
  const uefn = createSimulatedUefn(task);
  const searches = [];
  const verse = createVerseTools({
    specs: [
      { name: 'verse_reference', description: "Read Epic's Verse reference. No topic = list of topics.", inputSchema: { type: 'object', properties: { topic: { type: 'string' } }, required: [] } },
      { name: 'verse_api_search', description: "Search the real Verse API of the user's UEFN version. Use it before using any API.", inputSchema: { type: 'object', properties: { query: { type: 'string' } }, required: ['query'] } },
    ],
    run: async (name, args) => {
      if (name === 'verse_api_search') searches.push(String(args.query ?? ''));
      const value = name === 'verse_reference' ? (REFERENCE[String(args.topic ?? '').toLowerCase()] ?? `Topics: ${Object.keys(REFERENCE).join(', ')}`) : searchDigest(args.query);
      return { ok: true, value };
    },
  });
  const callTool = {
    name: 'unreal_call_tool',
    description: 'Call a tool of a UEFN editor toolset. Here: VerseToolset.VerseToolset with WriteFile {path, content, bCreateIfMissing}, ReadFile {path}, ListFiles {path, bRecursive} and BuildAll {}. Verse paths start with /zz_eval/.',
    parameters: { type: 'object', properties: { toolset: { type: 'string' }, tool: { type: 'string' }, arguments: { type: 'object' } }, required: ['toolset', 'tool'], additionalProperties: false },
    label: (args) => `UEFN: ${args.tool}`,
    run: async ({ toolset = '', tool = '', arguments: args = {} }) => uefn.call(toolset, tool, args).value,
  };
  return { tools: [...verse, callTool], uefn, searches };
}

// ─── Jueces ──────────────────────────────────────────────────────────

const CREATIVE_DEVICE_GAMEPLAY = /:=\s*class(?:<[^>]*>)*\(\s*creative_device\s*\)/;
const OLD_SPATIAL = /using\s*\{\s*\/UnrealEngine\.com\/Temporary\/SpatialMath\s*\}/;
const SAYS_COMPILES = /\b(compiles?|compiled successfully|builds? (?:fine|successfully|without errors)|compila(?:do)?)\b/i;

/** Métricas de una ejecución. `passed` exige las 4 condiciones de REQ-130. */
export function judge({ task, uefn, text }) {
  const builds = uefn.builds;
  const compiled = builds.length > 0 && builds.at(-1).length === 0 && builds.length <= MAX_ATTEMPTS;
  const code = uefn.code();
  const usesCreativeDevice = CREATIVE_DEVICE_GAMEPLAY.test(code);
  const invented = task.invented.filter((name) => code.includes(name));
  const oldSpatial = OLD_SPATIAL.test(code) && !/FromVector3/.test(code);
  const claimsWithoutBuild = builds.length === 0 && SAYS_COMPILES.test(text) ? 1 : 0;
  const passed = compiled && !usesCreativeDevice && invented.length === 0 && !oldSpatial && claimsWithoutBuild === 0;
  return { passed, compiled, attempts: builds.length, usesCreativeDevice, invented, oldSpatial, claimsWithoutBuild };
}
