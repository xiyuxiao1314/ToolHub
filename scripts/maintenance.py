#!/usr/bin/env python3
"""ToolHub storage maintenance. Preview by default; never edits runtime data.

Python 3.11+ standard library only. Windows package removal uses the Shell's
RECYCLEONDELETE operation, with no permanent-delete fallback. Cargo target is
the only permanently deleted tree. No arbitrary cleanup paths are accepted.
"""
from __future__ import annotations
import argparse
from contextlib import closing
import ctypes
from datetime import datetime
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import shlex
import sqlite3
import stat
import subprocess
import sys
import time
import tomllib
import uuid

class Unsafe(RuntimeError):
    pass

def normalized(value):
    return str(value).replace('\\\\?\\', '').replace('\\\\', '\\').replace('\\', '/').casefold()

def inside(path, parent):
    p, root = normalized(os.path.abspath(path)), normalized(os.path.abspath(parent))
    return p == root or p.startswith(root.rstrip('/') + '/')

def plain(path):
    """Check every existing path component without resolving through a link."""
    p = Path(os.path.abspath(path))
    for item in [*reversed(p.parents), p]:
        if not os.path.lexists(item):
            continue
        info = item.lstat()
        if stat.S_ISLNK(info.st_mode) or getattr(info, 'st_file_attributes', 0) & 0x400:
            raise Unsafe(f'Linked/reparse path refused: {item}')
    return p

def tree_bytes(path):
    total = 0
    pending = [plain(path)]
    while pending:
        with os.scandir(pending.pop()) as entries:
            for entry in entries:
                info = entry.stat(follow_symlinks=False)
                if stat.S_ISLNK(info.st_mode) or getattr(info, 'st_file_attributes', 0) & 0x400:
                    raise Unsafe(f'Linked/reparse child refused: {entry.path}')
                if stat.S_ISDIR(info.st_mode):
                    pending.append(entry.path)
                elif stat.S_ISREG(info.st_mode):
                    total += info.st_size
    return total

def read_json(path):
    return json.loads(Path(path).read_text(encoding='utf-8-sig'))

def strings(value):
    if isinstance(value, str):
        yield value
        try:
            nested = json.loads(value)
        except (ValueError, TypeError):
            return
        if not isinstance(nested, str):
            yield from strings(nested)
    elif isinstance(value, dict):
        for item in value.values():
            yield from strings(item)
    elif isinstance(value, (list, tuple)):
        for item in value:
            yield from strings(item)

def windows_state():
    # Fixed read-only script. No candidate paths or shell command text interpolated.
    command = r'''
$ErrorActionPreference='Stop'
[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false)
$processes=@(Get-CimInstance Win32_Process | Select-Object Name,ExecutablePath,CommandLine)
$refs=@()
$shell=New-Object -ComObject WScript.Shell
foreach ($folder in @([Environment]::GetFolderPath('Desktop'),[Environment]::GetFolderPath('Programs'),[Environment]::GetFolderPath('Startup'))) {
  $link=Join-Path $folder 'ToolHub.lnk'
  if (Test-Path -LiteralPath $link) { $item=$shell.CreateShortcut($link); $refs+=@($item.TargetPath,$item.WorkingDirectory) }
}
foreach ($task in @(Get-ScheduledTask | Where-Object { $_.TaskName -like 'ToolHub*' })) { foreach ($action in $task.Actions) { $refs+=@($action.Execute,$action.Arguments,$action.WorkingDirectory) } }
@{processes=$processes;references=$refs} | ConvertTo-Json -Compress -Depth 5
'''
    executable = shutil.which('pwsh') or shutil.which('powershell')
    if not executable:
        raise Unsafe('PowerShell unavailable; cannot verify live references')
    result = subprocess.run([executable, '-NoProfile', '-NonInteractive', '-Command', command], capture_output=True, encoding='utf-8', timeout=40)
    if result.returncode or result.stdout is None:
        raise Unsafe('Process/shortcut/startup inspection failed; cleanup skipped')
    return json.loads(result.stdout.lstrip('\ufeff'))

def registry_refs(registry):
    if not registry.exists():
        return []
    plain(registry)
    with closing(sqlite3.connect(registry.as_uri() + '?mode=ro', uri=True, timeout=3)) as db:
        tables = {r[0] for r in db.execute("SELECT name FROM sqlite_master WHERE type='table'")}
        refs = []
        for table in ['tool_instances', 'program_entries', 'skills', 'artifact_packages', 'artifact_roots']:
            if table in tables:
                for row in db.execute('SELECT * FROM ' + table):
                    refs.extend(strings(row))
        if 'program_entries' in tables:
            for (raw,) in db.execute('SELECT entry_json FROM program_entries'):
                entry = json.loads(raw)
                cwd = entry.get('cwd')
                if not cwd:
                    continue
                for text in [entry.get('command', ''), *entry.get('args', [])]:
                    if not isinstance(text, str):
                        continue
                    # This is conservative path inspection, never shell execution.
                    try:
                        tokens = [text, *shlex.split(text, posix=False)]
                    except ValueError:
                        # Unparseable registered commands may still depend on cwd/cache.
                        tokens = [text, 'target', 'dist']
                    for token in tokens:
                        token = token.strip("\"'")
                        if token:
                            refs.append(os.path.abspath(Path(cwd) / os.path.expandvars(token)))
        if 'settings' in tables:
            row = db.execute("SELECT value_json FROM settings WHERE key='agent_inventory'").fetchone()
            if row:
                refs.extend(strings(row[0]))
        # Artifact records use root + relative file snapshots; protect their joined paths too.
        if 'artifact_packages' in tables:
            for (raw,) in db.execute('SELECT package_json FROM artifact_packages'):
                package = json.loads(raw)
                root = package.get('root')
                if root:
                    for file in package.get('files', []):
                        name = file.get('relative_path', file.get('path')) if isinstance(file, dict) else file
                        if name:
                            refs.append(str(Path(root) / name))
    return refs

def config_files(home, roaming, repo):
    relative = ['.codex/config.toml', '.codex/skills/toolhub-integration/SKILL.md', '.config/mimocode/mimocode.jsonc', '.config/mimocode/skills/toolhub-integration/SKILL.md', '.config/opencode/opencode.json', '.config/opencode/opencode.jsonc', '.cursor/mcp.json', '.claude.json', '.gemini/settings.json', '.qwen/settings.json', '.codeium/windsurf/mcp_config.json', '.antigravity/mcp_config.json']
    paths = [home / p for p in relative]
    if roaming:
        paths += [roaming / p for p in ['Claude/claude_desktop_config.json', 'Code/User/mcp.json', 'VSCodium/User/mcp.json']]
        storage = roaming / 'Code/User/globalStorage'
        if storage.exists():
            for pattern in ['*/settings/cline_mcp_settings.json', '*/settings/mcp_settings.json']:
                paths.extend(storage.glob(pattern))
    if repo:
        paths += [repo / p for p in ['.mcp.json', '.cursor/mcp.json', '.vscode/mcp.json', '.codex/config.toml']]
    return paths

class Context:
    def __init__(self, repo=None, install_root=None, home=None, extra_refs=(), state_reader=None):
        self.repo = plain(repo) if repo else None
        self.home = Path(home or Path.home()).absolute()
        self.install_root = plain(install_root or Path(os.environ.get('LOCALAPPDATA', self.home / '.local/share')) / 'ToolHub')
        self.registry = self.home / '.toolhub/registry.sqlite'
        self.state_reader = state_reader or windows_state
        self.extra_refs = [Path(p).absolute() for p in extra_refs]
        if self.repo:
            data = tomllib.loads((self.repo / 'Cargo.toml').read_text(encoding='utf-8'))
            if 'apps/desktop' not in data.get('workspace', {}).get('members', []) or not (self.repo / 'apps/daemon').is_dir():
                raise Unsafe('Expected a ToolHub Cargo workspace')
        self.refresh()

    def refresh(self):
        state = self.state_reader()
        self.processes = state.get('processes', [])
        self.refs = list(strings(state.get('references', []))) + registry_refs(self.registry)
        for process in self.processes:
            self.refs += [s for s in [process.get('ExecutablePath'), process.get('CommandLine')] if s]
        self.current = None
        current_file = self.install_root / 'current.json'
        self.keep_ids = set()
        if current_file.exists():
            self.current = read_json(plain(current_file))
            self.refs.extend(strings(self.current))
            current_path = plain(self.current['path'])
            if not inside(current_path, self.install_root / 'versions') or current_path.parent != self.install_root / 'versions':
                raise Unsafe('Active install path is outside its versions directory')
            self.keep_ids.add(self.current['build_id'])
            previous = current_path / 'current.before.json'
            if previous.is_file():
                self.keep_ids.add(read_json(plain(previous)).get('build_id', ''))
        roaming = Path(os.environ['APPDATA']) if 'APPDATA' in os.environ else None
        for path in [*config_files(self.home, roaming, self.repo), *self.extra_refs]:
            if path.exists():
                plain(path)
                if path.stat().st_size > 4 * 1024 * 1024:
                    raise Unsafe(f'Reference file exceeds inspection limit: {path}')
                self.refs.append(path.read_text(encoding='utf-8-sig'))
        self.normal_refs = [normalized(os.path.expandvars(s)) for s in self.refs]
        self.building = any((p.get('Name') or '').lower() in {'cargo.exe', 'rustc.exe', 'rustdoc.exe', 'cargo', 'rustc', 'rustdoc'} for p in self.processes)

    def reference_reason(self, path):
        prefix = normalized(path).rstrip('/')
        # Config text may quote/escape paths; keep any literal reference conservatively.
        if any(prefix in reference for reference in self.normal_refs):
            return 'running process, current install, configuration or registered data references this path'
        return None

def package_info(path, installed=False):
    plain(path)
    manifest = path / 'package.json'
    if not manifest.is_file() or not (path / 'SHA256SUMS.txt').is_file():
        return None
    p = read_json(plain(manifest))
    build = p.get('build_id', '')
    if not isinstance(build, str) or p.get('schema') != 'toolhub.package/v1' or not re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9-]{1,63}', build):
        return None
    if path.name != (build if installed else 'toolhub-' + build):
        return None
    if not all((path / name).is_file() for name in ['toolhub.exe', 'toolhubd.exe', 'toolhub-desktop.exe']):
        return None
    try:
        stamp = datetime.fromisoformat(p.get('created_at', '')).timestamp()
    except (ValueError, TypeError):
        stamp = path.stat().st_mtime
    return {'id':build, 'stamp':stamp, 'mtime':path.stat().st_mtime, 'digest':hashlib.sha256(manifest.read_bytes()).hexdigest()}

def plan(context, scopes, keep=2):
    if keep < 2:
        raise Unsafe('Keep at least current + one rollback package')
    rows = []
    if 'build' in scopes and context.repo:
        target = context.repo / 'target'
        if target.exists():
            reason = context.reference_reason(target)
            if context.building:
                reason = 'Cargo/rustc build is running'
            if not ((target / '.rustc_info.json').is_file() or (target / 'CACHEDIR.TAG').is_file()):
                reason = 'Cargo cache marker missing'
            tracked = subprocess.run(['git', '-C', str(context.repo), 'ls-files', '--', 'target'], capture_output=True, text=True, check=True).stdout
            if tracked.strip():
                reason = 'Tracked source exists inside target'
            try:
                size = tree_bytes(target)
            except Unsafe as error:
                size, reason = 0, str(error)
            rows.append({'kind':'build', 'path':str(target), 'bytes':size, 'blocked':reason, 'action':'delete_regenerable_cache'})
    for scope, root, installed in [('packages', context.repo / 'dist' if context.repo else None, False), ('versions', context.install_root / 'versions', True)]:
        if scope not in scopes or root is None or not root.exists():
            continue
        plain(root)
        packages = []
        for path in root.iterdir():
            if not path.is_dir():
                continue
            try:
                info = package_info(path, installed)
                if info:
                    packages.append((path, info))
            except (Unsafe, ValueError, OSError):
                continue # Unrecognized/linked/corrupt directories are never cleanup candidates.
        packages.sort(key=lambda item:(item[1]['stamp'], item[1]['mtime']), reverse=True)
        retained = context.keep_ids | {info['id'] for _,info in packages[:keep]}
        for path, info in packages:
            reason = 'retained current/recent/rollback package' if info['id'] in retained else context.reference_reason(path)
            try:
                size = tree_bytes(path)
            except Unsafe as error:
                size, reason = 0, str(error)
            rows.append({'kind':scope, 'path':str(path), 'bytes':size, 'blocked':reason, 'action':'recycle_package', 'digest':info['digest']})
    return rows

def recycle_windows(path):
    """IFileOperation, Windows 8+: recycle only, no rmtree fallback on error."""
    if os.name != 'nt':
        raise Unsafe('Package recycling is Windows-only; nothing removed')
    class GUID(ctypes.Structure):
        _fields_ = [('bytes', ctypes.c_ubyte * 16)]
        def __init__(self, text):
            super().__init__(); self.bytes[:] = uuid.UUID(text).bytes_le
    HRESULT = ctypes.c_long
    ole = ctypes.OleDLL('ole32')
    shell = ctypes.OleDLL('shell32')
    operation, item = ctypes.c_void_p(), ctypes.c_void_p()
    def check(code):
        if code < 0:
            raise OSError(f'Windows recycle failed: 0x{code & 0xffffffff:08x}')
    def call(obj, index, argtypes, *values):
        table = ctypes.cast(obj, ctypes.POINTER(ctypes.POINTER(ctypes.c_void_p))).contents
        return ctypes.WINFUNCTYPE(HRESULT, ctypes.c_void_p, *argtypes)(table[index])(obj, *values)
    check(ole.CoInitializeEx(None, 2))
    try:
        check(ole.CoCreateInstance(ctypes.byref(GUID('3ad05575-8857-4850-9277-11b85bdb8e09')), None, 1, ctypes.byref(GUID('947aab5f-0a5c-4c13-b4d6-4bf7836fc9f8')), ctypes.byref(operation)))
        flags = 0x00080000 | 0x00100000 | 0x4000 | 0x400 | 0x40 | 0x10 | 0x4
        check(call(operation, 5, [ctypes.c_uint], flags))
        create = shell.SHCreateItemFromParsingName
        create.argtypes = [ctypes.c_wchar_p, ctypes.c_void_p, ctypes.POINTER(GUID), ctypes.POINTER(ctypes.c_void_p)]
        check(create(str(path), None, ctypes.byref(GUID('43826d1e-e718-42ee-bc55-a1e261c37bfe')), ctypes.byref(item)))
        check(call(operation, 18, [ctypes.c_void_p, ctypes.c_void_p], item, None))
        check(call(operation, 21, []))
        aborted = ctypes.c_int()
        check(call(operation, 22, [ctypes.POINTER(ctypes.c_int)], ctypes.byref(aborted)))
        if aborted.value or path.exists():
            raise Unsafe('Recycle was aborted or incomplete; no delete fallback attempted')
    finally:
        if item: call(item, 2, [])
        if operation: call(operation, 2, [])
        ole.CoUninitialize()

def apply(context, rows, scopes, keep=2, recycler=recycle_windows):
    results = []
    for row in rows:
        if row['blocked']:
            continue
        # Rebuild the plan against fresh live/config/data references before every mutation.
        context.refresh()
        fresh = next((r for r in plan(context, scopes, keep) if r['path'] == row['path']), None)
        if not fresh or fresh['blocked'] or fresh.get('digest') != row.get('digest'):
            results.append({**row, 'result':'skipped_changed_context'}); continue
        path = plain(row['path'])
        parent = context.repo if row['kind'] == 'build' else context.repo / 'dist' if row['kind'] == 'packages' else context.install_root / 'versions'
        if path.parent != plain(parent) or path == parent or not inside(path, parent):
            raise Unsafe('Cleanup path escaped its intended parent')
        tree_bytes(path) # Reject nested links before recursive operations.
        try:
            if row['kind'] == 'build':
                if path.name != 'target': raise Unsafe('Unexpected build cache path')
                shutil.rmtree(path)
            else:
                recycler(path)
            results.append({**fresh, 'result':'deleted' if row['kind'] == 'build' else 'recycled'})
        except (OSError, Unsafe) as error:
            results.append({**fresh, 'result':'failed', 'error':str(error)})
    return results

def validate_report(path):
    path = plain(path)
    if path.suffix.lower() != '.json':
        raise Unsafe('Maintenance report must use a .json filename')
    if path.exists():
        try:
            old = read_json(path)
        except (ValueError, UnicodeError):
            raise Unsafe('Refusing to overwrite a non-maintenance report')
        if not isinstance(old, dict) or old.get('schema') != 'toolhub.maintenance/v1':
            raise Unsafe('Refusing to overwrite a non-maintenance report')
    return path

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo', type=Path)
    parser.add_argument('--install-root', type=Path)
    parser.add_argument('--scope', choices=['all','build','packages','versions'], default='all')
    parser.add_argument('--keep', type=int, default=2)
    parser.add_argument('--reference-file', action='append', type=Path, default=[])
    parser.add_argument('--apply', action='store_true')
    parser.add_argument('--report', type=Path)
    opts = parser.parse_args()
    if opts.report:
        opts.report = validate_report(opts.report)
    repo = opts.repo
    if repo is None:
        repo = next((p for p in Path(__file__).resolve().parents if (p/'Cargo.toml').is_file()), None)
    if os.name != 'nt':
        parser.error('Native process/reference inspection and recycling currently require Windows')
    scopes = {'build','packages','versions'} if opts.scope == 'all' else {opts.scope}
    context = Context(repo, opts.install_root, extra_refs=opts.reference_file)
    rows = plan(context, scopes, opts.keep)
    report = {'schema':'toolhub.maintenance/v1', 'at':time.time(), 'applied':opts.apply, 'plan':rows, 'results':[], 'cache_bytes':sum(r['bytes'] for r in rows if not r['blocked'] and r['kind']=='build'), 'package_bytes':sum(r['bytes'] for r in rows if not r['blocked'] and r['kind']!='build')}
    if opts.apply:
        report['results'] = apply(context, rows, scopes, opts.keep)
    for row in rows:
        print(f"{'KEEP' if row['blocked'] else 'CLEAN'} {row['bytes']/2**30:.3f} GiB {row['path']}" + (f" — {row['blocked']}" if row['blocked'] else ''))
    for row in report['results']:
        print(f"{row['result'].upper()} {row['path']}")
    if opts.report:
        opts.report.parent.mkdir(parents=True, exist_ok=True)
        opts.report.write_text(json.dumps(report,ensure_ascii=False,indent=2), encoding='utf-8')
    print(f"{'Applied' if opts.apply else 'Preview only'}; cache {report['cache_bytes']/2**30:.3f} GiB, packages {report['package_bytes']/2**30:.3f} GiB (packages go to Recycle Bin; space is not freed until it is emptied).")
    return 1 if any(r['result']=='failed' for r in report['results']) else 0

if __name__ == '__main__':
    try:
        raise SystemExit(main())
    except (Unsafe, OSError, ValueError, sqlite3.Error, subprocess.SubprocessError) as error:
        print(f'Maintenance refused: {error}', file=sys.stderr)
        raise SystemExit(1)
