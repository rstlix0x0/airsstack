#!/usr/bin/env node
// airsstack concise — UserPromptSubmit hook.
//
// Detects concise activation, level switch, and deactivation in the user prompt
// (slash command + natural language), persists the active level to a
// brand-namespaced flag file, and re-injects the active level's directive every
// turn so terse mode survives the whole session instead of drifting back to
// verbose. Must never throw or block the prompt — every path fails silently.

const fs = require('fs');
const path = require('path');
const os = require('os');

const LEVELS = ['lite', 'full', 'ultra'];
const DEFAULT_LEVEL = 'full';

const stateRoot = process.env.AIRSSTACK_HOME || path.join(os.homedir(), '.airsstack');
const flagPath = path.join(stateRoot, 'cc', 'concise.json');

function writeLevel(level) {
  try {
    fs.mkdirSync(path.dirname(flagPath), { recursive: true });
    // Never write through a symlink planted at the flag path.
    try {
      if (fs.lstatSync(flagPath).isSymbolicLink()) fs.unlinkSync(flagPath);
    } catch (e) { /* missing is fine */ }
    fs.writeFileSync(flagPath, JSON.stringify({ level }) + '\n', { mode: 0o600 });
  } catch (e) { /* silent */ }
}

function clearLevel() {
  try { fs.unlinkSync(flagPath); } catch (e) { /* already off */ }
}

function readLevel() {
  try {
    const st = fs.lstatSync(flagPath);
    if (st.isSymbolicLink() || !st.isFile() || st.size > 1024) return null;
    const level = (JSON.parse(fs.readFileSync(flagPath, 'utf8')) || {}).level;
    return LEVELS.includes(level) ? level : null;
  } catch (e) { return null; }
}

function directive(level) {
  const common =
    'Keep ALL technical substance, code blocks, shell commands, and error text ' +
    'verbatim. Technical terms exact. Write normally (clarity over brevity) for ' +
    'security warnings, irreversible-action confirmations, and ordered multi-step ' +
    'instructions.';
  const byLevel = {
    lite: 'AIRSSTACK CONCISE: LITE. Drop filler (just/really/basically/actually/' +
          'simply), hedging, and pleasantries. Keep articles and complete sentences.',
    full: 'AIRSSTACK CONCISE: FULL. Drop articles where unambiguous, filler, ' +
          'hedging, pleasantries. Fragments OK. Prefer short synonyms.',
    ultra: 'AIRSSTACK CONCISE: ULTRA. Telegraphic. Maximal compression — fragments, ' +
           'bullets, minimal connective words.',
  };
  return byLevel[level] + ' ' + common;
}

let input = '';
process.stdin.on('data', c => { input += c; });
process.stdin.on('end', () => {
  try {
    const data = JSON.parse(input || '{}');
    const lower = (data.prompt || '').trim().toLowerCase();

    let handled = false;

    // Deactivation first, so "stop concise" never re-activates below.
    if (/\bnormal mode\b/.test(lower) ||
        /\bverbose mode\b/.test(lower) ||
        /\b(stop|disable|deactivate|turn off|exit)\b[^.]*\bconcise\b/.test(lower) ||
        /\bconcise\b[^.]*\b(off|stop|disable|deactivate|turn off)\b/.test(lower)) {
      clearLevel();
      handled = true;
    }

    // Slash command: /concise or /airsstack:concise [level|off]
    if (!handled) {
      const m = /^\/(?:airsstack:)?concise(?:\s+(\S+))?/.exec(lower);
      if (m) {
        const arg = m[1];
        if (!arg) writeLevel(DEFAULT_LEVEL);
        else if (['off', 'stop', 'disable'].includes(arg)) clearLevel();
        else if (LEVELS.includes(arg)) writeLevel(arg);
        // unknown arg → flag untouched (no silent overwrite)
        handled = true;
      }
    }

    // Natural-language activation: "concise mode", "be terse", "ultra concise"...
    if (!handled &&
        /\b(concise|terse)\b/.test(lower) &&
        /\b(mode|be|use|go|make it|turn on|enable|activate|talk)\b/.test(lower)) {
      const lvl = LEVELS.find(l => new RegExp('\\b' + l + '\\b').test(lower)) || DEFAULT_LEVEL;
      writeLevel(lvl);
    }

    // Persistence: re-inject the active level's directive every turn.
    const active = readLevel();
    if (active) {
      process.stdout.write(JSON.stringify({
        hookSpecificOutput: {
          hookEventName: 'UserPromptSubmit',
          additionalContext: directive(active),
        },
      }));
    }
  } catch (e) { /* silent — the hook must never block the prompt */ }
});
