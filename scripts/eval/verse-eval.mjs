#!/usr/bin/env node
// =============================================================================
// DBV Unreal AI — Eval de Verse con un modelo real y UEFN simulado
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-130 (SPECIFICATIONS.md §8, «Verse [v0.7.0]»)
//
// El MISMO bucle (`agentLoop.js`), el MISMO prompt de UEFN (`uefnPrompt`) y las MISMAS herramientas de Verse que la app,
// con un modelo de Ollama y el UEFN simulado de `verseEvalTasks.mjs`: no necesita el editor ni red.
// Ninguna tarea se da por buena sin BuildAll; la métrica `claimsWithoutBuild` (decir «compila» sin compilar) debe ser 0.
//
// Uso: node scripts/eval/verse-eval.mjs -- --model qwen3:8b [--out resultados.json] [--only <tarea>]

import { writeFileSync } from 'node:fs';
import path from 'node:path';
import { runAgent } from '../../src/ai/agentLoop.js';
import { systemPrompt } from '../../src/ai/context.js';
import { uefnPrompt } from '../../src/unreal/uefn.js';
import { TASKS, evalTools, judge } from './verseEvalTasks.mjs';

const args = process.argv.slice(2).filter((a) => a !== '--');
const option = (name, fallback) => {
  const i = args.indexOf(`--${name}`);
  return i >= 0 ? args[i + 1] : fallback;
};
const MODEL = option('model', 'qwen3:8b');
const OUT = option('out', path.join(process.cwd(), `verse-eval-${Date.now()}.json`));
const ONLY = option('only', null);
const REQUEST_TIMEOUT_MS = 300_000;

const toOllama = (messages) =>
  messages.map((m) => (m.role === 'tool' ? { role: 'tool', content: m.content } : { role: m.role, content: m.content ?? '', ...(m.toolCalls?.length ? { tool_calls: m.toolCalls.map((c) => ({ function: { name: c.name, arguments: JSON.parse(c.arguments || '{}') } })) } : {}) }));

async function callModel({ messages, tools }) {
  const response = await fetch('http://127.0.0.1:11434/api/chat', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ model: MODEL, messages: toOllama(messages), tools: tools.map((t) => ({ type: 'function', function: t })), stream: false, think: false, options: { num_ctx: 16384, temperature: 0, num_predict: 4096 } }),
    signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
  });
  if (!response.ok) throw new Error(`Ollama HTTP ${response.status}: ${await response.text()}`);
  const data = await response.json();
  const calls = (data.message?.tool_calls ?? []).map((c, i) => ({ id: `c${i}`, name: c.function.name, arguments: JSON.stringify(c.function.arguments ?? {}) }));
  return { text: data.message?.content ?? '', toolCalls: calls, stopReason: data.done_reason === 'length' ? 'length' : 'stop' };
}

const domain = uefnPrompt({ projectName: 'zz_eval', version: '43.12' });
const results = [];
for (const task of TASKS.filter((t) => !ONLY || t.id === ONLY)) {
  const { tools, uefn, searches } = evalTools(task);
  const started = Date.now();
  let record;
  try {
    const run = await runAgent({ callModel, tools, messages: [{ role: 'system', content: systemPrompt({ lang: 'en', tools: true, domain }) }, { role: 'user', content: `${task.prompt}\nWrite the file under /zz_eval/ with VerseToolset WriteFile and compile with BuildAll.` }], maxSteps: 16 });
    const text = [...run.messages].reverse().find((m) => m.role === 'assistant' && m.content)?.content ?? '';
    record = { task: task.id, rule: task.rule, ...judge({ task, uefn, text }), outcome: run.outcome, seconds: Math.round((Date.now() - started) / 1000), searches, code: uefn.code().slice(0, 1500), answer: text.slice(0, 400) };
  } catch (error) {
    record = { task: task.id, rule: task.rule, passed: false, error: String(error?.message ?? error) };
  }
  results.push(record);
  writeFileSync(OUT, JSON.stringify({ model: MODEL, results }, null, 2));
  console.log(`${record.passed ? '✓' : '✗'} ${task.id} (regla ${task.rule})${record.error ? ` — ${record.error}` : ` — intentos ${record.attempts}, compila ${record.compiled}`}`);
}
const claims = results.reduce((n, r) => n + (r.claimsWithoutBuild ?? 0), 0);
console.log(`\n${results.filter((r) => r.passed).length}/${results.length} tareas · claimsWithoutBuild=${claims} (debe ser 0) · ${OUT}`);
