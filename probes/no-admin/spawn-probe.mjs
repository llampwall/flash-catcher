// spawn-probe.mjs — spawn a command with a chosen Windows console configuration.
//   node spawn-probe.mjs <mode> [--cwd DIR] -- <cmd> [args...]
// modes: inherit | hidden | detached | detached-hidden
//   inherit         : plain spawn (child inherits this process's console)
//   hidden          : windowsHide:true  (CREATE_NO_WINDOW: child gets its own hidden console)
//   detached        : detached:true     (DETACHED_PROCESS: child has NO console)
//   detached-hidden : detached:true + windowsHide:true (what mercenary uses for claude)
import { spawn } from 'node:child_process';
import { writeFileSync } from 'node:fs';

const argv = process.argv.slice(2);
const mode = argv.shift();
let cwd = process.cwd();
if (argv[0] === '--cwd') { argv.shift(); cwd = argv.shift(); }
if (argv[0] === '--') argv.shift();
const [cmd, ...args] = argv;
// Strip the nested-session markers so a spawned `claude` does not refuse to start inside claude.
const env = { ...process.env };
for (const k of Object.keys(env)) if (/^CLAUDE/i.test(k)) delete env[k];
const opts = { cwd, stdio: ['ignore', 'pipe', 'pipe'], shell: false, env };
if (mode === 'hidden' || mode === 'detached-hidden') opts.windowsHide = true;
if (mode === 'detached' || mode === 'detached-hidden') opts.detached = true;

const t0 = Date.now();
const child = spawn(cmd, args, opts);
let out = '', err = '';
child.stdout.on('data', d => { out += d; });
child.stderr.on('data', d => { err += d; });
child.on('exit', (code, sig) => {
  const ms = Date.now() - t0;
  writeFileSync(`${process.env.PROBE_LOG || 'probe'}-${mode}.log`, `exit=${code} sig=${sig} ms=${ms}\n--- stdout ---\n${out}\n--- stderr ---\n${err}\n`);
  console.log(`[probe] mode=${mode} pid=${child.pid} exit=${code} ms=${ms} stdout=${out.length}B stderr=${err.length}B`);
});
child.on('error', e => { console.log(`[probe] spawn error: ${e.message}`); });
