// cycle-launcher-probe.mjs — reproduces allmind-ignition's launchDetachedCycle spawn shape
// (ignition.js:1052): a DETACHED (console-less) cmd.exe running `start /min pwsh ...` under
// ignition's allowlist-style env. cmd.exe's AutoRun (HKCU\Software\Microsoft\Command Processor
// \AutoRun -> conda_hook.bat -> doskey.exe) then runs from a console-less cmd, and doskey.exe is
// a console-subsystem program, so it gets a brand-new VISIBLE console (Windows Terminal handoff).
//
//   node cycle-launcher-probe.mjs            # as ignition does it today
//   node cycle-launcher-probe.mjs --fixed    # adds cmd.exe /d, which disables AutoRun
//
// Both variants also open a MINIMIZED pwsh window in the taskbar for ~3 s (that is the
// `start /min` cycle window itself, present in the real cycle too and not a flash).
import { spawn } from 'node:child_process';

const fixed = process.argv.includes('--fixed');
const keep = new Set(['PATH', 'PATHEXT', 'SYSTEMROOT', 'WINDIR', 'COMSPEC', 'TEMP', 'TMP', 'USERPROFILE',
  'USERNAME', 'HOMEDRIVE', 'HOMEPATH', 'APPDATA', 'LOCALAPPDATA', 'PROGRAMDATA', 'PROGRAMFILES',
  'SYSTEMDRIVE', 'NUMBER_OF_PROCESSORS', 'OS']);
const env = {};
for (const [k, v] of Object.entries(process.env)) if (keep.has(k.toUpperCase())) env[k] = v;

const args = [...(fixed ? ['/d'] : []), '/c', 'start', '/min', 'pwsh.exe', '-NoProfile', '-Command', 'Start-Sleep 3'];
const child = spawn('cmd.exe', args, { detached: true, stdio: 'ignore', windowsHide: true, env });
child.unref();
console.log(`[cycle-probe] fixed=${fixed} pid=${child.pid} cmd.exe ${args.join(' ')}`);
