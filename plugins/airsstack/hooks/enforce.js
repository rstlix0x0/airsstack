#!/usr/bin/env node
// airsstack rule-enforcement dispatcher — PreToolUse(Edit|Write) hook.
//
// Behavior-parity fallback for enforce.py. Reads the installed-plugins
// registry, keeps only airsstack-marketplace plugins, loads each one's
// enforcement.json, and surfaces the matching guideline skill via
// additionalContext once per stack:phase per session. Fail-open: never
// blocks, denies, or throws out of the handler.

const fs = require('fs');
const path = require('path');
const os = require('os');

const MARKETPLACE_SUFFIX = '@airsstack';
const MARKER_MAX_AGE = 24 * 3600 * 1000; // ms

function registryPath() {
  return process.env.AIRSSTACK_ENFORCE_REGISTRY ||
    path.join(os.homedir(), '.claude', 'plugins', 'installed_plugins.json');
}

function sddRoot() {
  const home = process.env.AIRSSTACK_HOME || path.join(os.homedir(), '.airsstack');
  return path.join(home, 'cc', 'plugins', 'sdd');
}

function markerDir() {
  return process.env.TMPDIR || '/tmp';
}

function readRegistry() {
  let plugins;
  try {
    const data = JSON.parse(fs.readFileSync(registryPath(), 'utf8'));
    plugins = (data && data.plugins) || {};
  } catch (e) { return []; }
  const seen = new Set(); const paths = [];
  for (const key of Object.keys(plugins)) {
    if (!key.endsWith(MARKETPLACE_SUFFIX)) continue; // scope guard
    const records = plugins[key];
    if (!Array.isArray(records)) continue;
    for (const rec of records) {
      if (rec && typeof rec === 'object' && rec.installPath) {
        const p = rec.installPath;
        if (!seen.has(p)) { seen.add(p); paths.push(p); }
      }
    }
  }
  return paths;
}

function loadManifests(paths) {
  const out = [];
  for (const p of paths) {
    let m;
    try { m = JSON.parse(fs.readFileSync(path.join(p, 'enforcement.json'), 'utf8')); }
    catch (e) { continue; } // absent or malformed → skip, keep the rest
    if (!m || typeof m !== 'object' || Array.isArray(m)) continue;
    if (!m.stack || !m.skill) continue;
    out.push({
      stack: m.stack,
      skill: m.skill,
      detect: m.detect || [],
      match: m.match || [],
      phase: m.phase || ['code', 'design'],
    });
  }
  return out;
}

function globToRegExp(seg) {
  // Translate a single glob segment to a full-match regex with the same
  // semantics as Python fnmatch.translate for the supported metacharacters:
  //   *       → .*   (any sequence)
  //   ?       → .    (any single character)
  //   [seq]   → [seq] character class
  //   [!seq]  → [^seq] negated character class
  //   unclosed [ → literal \[  (fnmatch behavior)
  // All other regex metacharacters are escaped as literals.
  const s = String(seg);
  let re = '';
  let i = 0;
  while (i < s.length) {
    const ch = s[i];
    if (ch === '*') {
      re += '.*';
      i++;
    } else if (ch === '?') {
      re += '.';
      i++;
    } else if (ch === '[') {
      // Scan ahead to find the closing ]
      let j = i + 1;
      // leading ! or ] right after [ don't close the class
      if (j < s.length && s[j] === '!') j++;
      if (j < s.length && s[j] === ']') j++;
      while (j < s.length && s[j] !== ']') j++;
      if (j >= s.length) {
        // unclosed [ — treat as literal
        re += '\\[';
        i++;
      } else {
        // valid character class [i..j]
        let inner = s.slice(i + 1, j); // content between [ and ]
        const negated = inner.startsWith('!');
        if (negated) inner = inner.slice(1);
        // Escape regex metacharacters inside the class, except - and ]
        // (] is already the closing delimiter; - is valid in classes as-is)
        // We must escape ^, \, and any chars that need escaping inside [...].
        inner = inner.replace(/\\/g, '\\\\').replace(/\^/g, '\\^');
        re += '[' + (negated ? '^' : '') + inner + ']';
        i = j + 1;
      }
    } else {
      re += ch.replace(/[.+^${}()|[\]\\]/g, '\\$&');
      i++;
    }
  }
  return new RegExp('^' + re + '$');
}

function basenameMatch(filePath, globs) {
  const base = path.basename(filePath);
  for (const g of globs) {
    const seg = String(g).split('/').pop();
    if (globToRegExp(seg).test(base)) return true;
  }
  return false;
}

function markerActive(cwd, markers) {
  if (!markers || markers.length === 0) return false;
  let d = path.resolve(cwd || '.');
  for (;;) {
    for (const m of markers) {
      try { if (fs.statSync(path.join(d, m)).isFile()) return true; } catch (e) { /* none */ }
    }
    const parent = path.dirname(d);
    if (parent === d) return false;
    d = parent;
  }
}

function isDesignDoc(filePath) {
  const fp = path.resolve(filePath);
  const root = path.resolve(sddRoot());
  if (!(fp === root || fp.startsWith(root + path.sep))) return false;
  return fp.includes('/specs/') || fp.includes('/plans/');
}

function matches(filePath, cwd, manifests) {
  const hits = [];
  const design = isDesignDoc(filePath);
  for (const m of manifests) {
    if (design) {
      if (m.phase.includes('design') && markerActive(cwd, m.detect)) {
        hits.push([m.stack, 'design', m.skill]);
      }
    } else if (m.phase.includes('code') && basenameMatch(filePath, m.match)) {
      hits.push([m.stack, 'code', m.skill]);
    }
  }
  return hits;
}

function pointer(stack, skill) {
  return stack + ' work is in play. The ' + skill + ' skill is MANDATORY for ' +
    'this work — load it now via Skill before proceeding, and apply its ' +
    'rules (Definition of Done + architecture).';
}

function markerPath(sessionId) {
  const safe = String(sessionId || 'nosession').replace(/[^A-Za-z0-9_-]/g, '-');
  return path.join(markerDir(), 'airsstack-enforce-' + safe);
}

function pruneMarkers() {
  try {
    const now = Date.now();
    for (const name of fs.readdirSync(markerDir())) {
      if (!name.startsWith('airsstack-enforce-')) continue;
      const p = path.join(markerDir(), name);
      try {
        if (now - fs.statSync(p).mtimeMs > MARKER_MAX_AGE) fs.unlinkSync(p);
      } catch (e) { /* ignore */ }
    }
  } catch (e) { /* ignore */ }
}

function already(sessionId) {
  try {
    return new Set(
      fs.readFileSync(markerPath(sessionId), 'utf8')
        .split('\n').map(s => s.trim()).filter(Boolean)
    );
  } catch (e) { return new Set(); }
}

function record(sessionId, keys) {
  try { fs.appendFileSync(markerPath(sessionId), keys.map(k => k + '\n').join('')); }
  catch (e) { /* best-effort */ }
}

let input = '';
process.stdin.on('data', c => { input += c; });
process.stdin.on('end', () => {
  try {
    const data = JSON.parse(input || '{}');
    const filePath = (data.tool_input || {}).file_path;
    if (!filePath) return;
    const cwd = data.cwd || process.cwd();
    const sessionId = data.session_id || '';

    pruneMarkers();

    const manifests = loadManifests(readRegistry());
    if (manifests.length === 0) return;

    const hits = matches(filePath, cwd, manifests);
    if (hits.length === 0) return;

    const seen = already(sessionId);
    const pointers = []; const newKeys = [];
    for (const [stack, phase, skill] of hits) {
      const key = stack + ':' + phase;
      if (seen.has(key) || newKeys.includes(key)) continue;
      newKeys.push(key);
      pointers.push(pointer(stack, skill));
    }
    if (pointers.length === 0) return;

    record(sessionId, newKeys);
    process.stdout.write(JSON.stringify({
      hookSpecificOutput: {
        hookEventName: 'PreToolUse',
        additionalContext: pointers.join('\n'),
        permissionDecision: 'defer',
      },
    }));
  } catch (e) { /* fail-open: never block an edit */ }
});
