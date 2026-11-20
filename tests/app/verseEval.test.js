// =============================================================================
// DBV Unreal AI — Tests: la eval de Verse y sus jueces
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// La eval de verdad (`scripts/eval/verse-eval.mjs`) necesita un modelo. Aquí, un modelo guionizado recorre el mismo
// bucle con el UEFN simulado para comprobar que los jueces aprueban lo bueno y detectan cada infracción.

import { describe, expect, it } from 'vitest';
import { runAgent } from '../../src/ai/agentLoop.js';
import { MAX_ATTEMPTS, TASKS, evalTools, judge, searchDigest } from '../../scripts/eval/verseEvalTasks.mjs';

const scripted = (steps, answer) => {
  let index = 0;
  return async () => {
    const step = steps[index];
    index += 1;
    return step ? { text: '', toolCalls: [{ id: `c${index}`, name: step[0], arguments: JSON.stringify(step[1]) }], stopReason: 'stop' } : { text: answer, toolCalls: [], stopReason: 'stop' };
  };
};

const write = (content) => ['unreal_call_tool', { toolset: 'VerseToolset.VerseToolset', tool: 'WriteFile', arguments: { path: '/zz_eval/zz_task.verse', content, bCreateIfMissing: true } }];
const build = ['unreal_call_tool', { toolset: 'VerseToolset.VerseToolset', tool: 'BuildAll', arguments: {} }];

async function run(taskId, steps, answer = 'Done.') {
  const task = TASKS.find((t) => t.id === taskId);
  const env = evalTools(task);
  const result = await runAgent({ callModel: scripted(steps, answer), tools: env.tools, messages: [{ role: 'user', content: task.prompt }], maxSteps: 20 });
  const text = [...result.messages].reverse().find((m) => m.role === 'assistant' && m.content)?.content ?? '';
  return { ...judge({ task, uefn: env.uefn, text }), env };
}

const fuse = `zz_fuse := class<final_super>(component):
    OnSimulate<override>()<suspends>:void =
        race:
            Sleep(10.0)
            CancelEvent.Await()
        Print("ZZTEST exploded")`;

describe('eval de Verse (REQ-130, §8)', () => {
  it('hay 12 tareas que cubren las 10 reglas del estudio', () => {
    // @spec REQ-130
    expect(TASKS).toHaveLength(12);
    expect(new Set(TASKS.map((t) => t.rule))).toEqual(new Set([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]));
  });

  it('aprueba una tarea que compila, con componentes y sin firmas inventadas', async () => {
    // @spec REQ-130
    const result = await run('cancellable_countdown', [['verse_reference', { topic: 'concurrency' }], write(fuse), build], 'It compiles: BuildAll returned no errors.');
    expect(result.passed).toBe(true);
    expect(result.attempts).toBe(1);
  });

  it('corregir tras el error del BuildAll simulado cuenta como intento y aprueba', async () => {
    // @spec REQ-130
    const bad = 'zz_avg := class<final_super>(component):\n    Avg(S:[]int):int =\n        Total:int = 10\n        Total / S.Length';
    const good = 'zz_avg := class<final_super>(component):\n    Avg(S:[]int):int =\n        Total:int = 10\n        Floor[Total / S.Length]';
    const result = await run('average_score', [write(bad), build, write(good), build]);
    expect(result).toMatchObject({ passed: true, attempts: 2 });
    expect(result.env.uefn.builds[0][0].code).toBe(3509);
  });

  it('no aprueba sin BuildAll, y decir «compila» sin compilar se cuenta', async () => {
    // @spec REQ-130
    const result = await run('cancellable_countdown', [write(fuse)], 'It compiles.');
    expect(result.passed).toBe(false);
    expect(result.claimsWithoutBuild).toBe(1);
  });

  it('detecta la jugabilidad en creative_device', async () => {
    // @spec REQ-130
    const device = `zz_fuse := class(creative_device):\n${fuse.split('\n').slice(1).join('\n')}`;
    const result = await run('cancellable_countdown', [write(device), build]);
    expect(result.usesCreativeDevice).toBe(true);
    expect(result.passed).toBe(false);
  });

  it('detecta firmas inventadas y el SpatialMath obsoleto sin conversión', async () => {
    // @spec REQ-130
    const invented = await run('damage_multiplier', [write('zz_d := class<final_super>(component):\n    D(B:int, M:float):int = Round[ToFloat(B) * M]'), build]);
    expect(invented.invented).toContain('ToFloat(');
    const old = await run('teleport_player', [write('using { /UnrealEngine.com/Temporary/SpatialMath }\nzz_t := class<final_super>(component):\n    Go(C:fort_character):void =\n        if (C.TeleportTo[vector3{}, rotation{}]) {}'), build]);
    expect(old.oldSpatial).toBe(true);
    expect(old.passed).toBe(false);
  });

  it('pasar el tope de intentos no aprueba', async () => {
    // @spec REQ-130
    const steps = Array.from({ length: MAX_ATTEMPTS + 1 }, () => [write('x := 1'), build]).flat();
    const result = await run('safe_loop', steps);
    expect(result.attempts).toBe(MAX_ATTEMPTS + 1);
    expect(result.passed).toBe(false);
  });

  it('verse_api_search encuentra las firmas del digest y no inventa', () => {
    // @spec REQ-130
    expect(searchDigest('TeamMemberSpawnedEvent')).toContain('listenable(agent)');
    expect(searchDigest('OnPlayerJoined')).toContain('Do not invent');
  });
});
