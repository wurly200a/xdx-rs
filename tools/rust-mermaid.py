#!/usr/bin/env python3
"""
rust-mermaid.py — Generate a Mermaid classDiagram from a Rust workspace.

Backends:
  - cargo metadata       : inter-crate dependency edges
  - cargo modules structure : per-crate public symbol extraction

Usage:
  python tools/rust-mermaid.py [OPTIONS] [CRATE...]

  CRATE...              Workspace crate names to include.
                        Omit to use --preset or all workspace crates.

Options:
  --preset NAME         Named crate list from rust-mermaid.toml.
  --symbols TYPES       Comma-separated symbol types: struct,enum,trait,fn,type
                        Default: struct,enum,trait
  --no-symbols          Show crate nodes only, no symbol members.
  --out FILE            Write Mermaid to FILE instead of stdout.
  --workspace DIR       Workspace root (default: current directory).
  -h, --help            Show this help.

rust-mermaid.toml (place in workspace root):
  [preset.release]
  crates = ["xdx-gui", "xdx-core", "xdx-midi", "xdx-synth"]

  [preset.validation]
  crates = ["xdx-compare", "xdx-core", "xdx-e2e", "xdx-eg-viewer", "xdx-midi", "xdx-synth"]

Symbol visibility: only items with exactly `pub` are included.
`pub(crate)`, `pub(self)`, `pub(super)` are silently excluded.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path


# ---------------------------------------------------------------------------
# Minimal TOML parser (Python < 3.11 fallback, no third-party deps required)
# ---------------------------------------------------------------------------

def _parse_toml_simple(text: str) -> dict:
    root: dict = {}

    def _navigate(d: dict, keys: list[str]) -> dict:
        for k in keys:
            d = d.setdefault(k, {})
        return d

    current: dict = root
    for line in text.splitlines():
        line = line.strip()
        if not line or line.startswith('#'):
            continue
        m = re.match(r'^\[([^\]]+)\]$', line)
        if m:
            parts = [p.strip() for p in m.group(1).split('.')]
            current = _navigate(root, parts)
            continue
        m = re.match(r'^([\w-]+)\s*=\s*(.+)$', line)
        if m:
            key, val = m.group(1), m.group(2).strip()
            if val.startswith('['):
                current[key] = re.findall(r'"([^"]+)"', val)
            elif val.startswith('"'):
                current[key] = val.strip('"')
            else:
                current[key] = val
    return root


def _load_toml(path: Path) -> dict:
    text = path.read_text(encoding='utf-8')
    try:
        import tomllib          # Python 3.11+
        return tomllib.loads(text)
    except ImportError:
        pass
    try:
        import tomli            # popular third-party fallback
        return tomli.loads(text)
    except ImportError:
        pass
    return _parse_toml_simple(text)


# ---------------------------------------------------------------------------
# cargo metadata — inter-crate dependency graph
# ---------------------------------------------------------------------------

def cargo_metadata(workspace_dir: Path) -> dict:
    r = subprocess.run(
        ['cargo', 'metadata', '--format-version', '1', '--no-deps'],
        cwd=workspace_dir, capture_output=True,
        encoding='utf-8', errors='replace',
    )
    if r.returncode != 0:
        print(f'cargo metadata failed:\n{r.stderr}', file=sys.stderr)
        sys.exit(1)
    return json.loads(r.stdout)


def workspace_packages(meta: dict) -> dict[str, dict]:
    members = set(meta.get('workspace_members', []))
    return {pkg['name']: pkg for pkg in meta['packages'] if pkg['id'] in members}


def internal_deps(pkg: dict, workspace_names: set[str]) -> list[str]:
    return [d['name'] for d in pkg.get('dependencies', []) if d['name'] in workspace_names]


# ---------------------------------------------------------------------------
# cargo modules structure — per-crate public symbol extraction
# ---------------------------------------------------------------------------

_ANSI = re.compile(r'\x1b\[[0-9;]*m')
_KINDS = {'struct', 'enum', 'trait', 'fn', 'type'}

# Matches "kind name: pub" — exact pub only, not pub(crate) / pub(self) / pub(super)
_ITEM_RE = re.compile(
    r'\b(struct|enum|trait|fn|type)\s+(\w+):\s+pub\s*$'
)


def _target_flag(pkg: dict) -> list[str] | None:
    """
    Return the cargo-modules target flag(s) for this package, or None if no
    analyzable target exists (e.g. examples-only packages).

    Priority:
      1. lib target  → ['--lib']
      2. bin whose name == package name  → ['--bin', name]
      3. any single bin  → ['--bin', name]
      4. no suitable target  → None
    """
    targets = pkg.get('targets', [])
    lib_targets = [t for t in targets if 'lib' in t.get('kind', [])]
    bin_targets = [t for t in targets if 'bin' in t.get('kind', [])]

    if lib_targets:
        return ['--lib']
    # prefer the bin whose name matches the package
    named = [t for t in bin_targets if t['name'] == pkg['name']]
    if named:
        return ['--bin', named[0]['name']]
    if len(bin_targets) == 1:
        return ['--bin', bin_targets[0]['name']]
    if bin_targets:
        # multiple bins, no obvious primary — use the first alphabetically
        return ['--bin', sorted(b['name'] for b in bin_targets)[0]]
    return None


def _run_structure(
    crate: str, pkg: dict, workspace_dir: Path, symbol_types: list[str]
) -> str:
    target_flag = _target_flag(pkg)
    if target_flag is None:
        return ''  # no analyzable target (e.g. examples-only)

    cmd = ['cargo', 'modules', 'structure', '-p', crate] + target_flag
    if 'fn'    not in symbol_types: cmd.append('--no-fns')
    if 'trait' not in symbol_types: cmd.append('--no-traits')
    if not {'struct', 'enum', 'type'} & set(symbol_types): cmd.append('--no-types')
    r = subprocess.run(
        cmd, cwd=workspace_dir, capture_output=True,
        encoding='utf-8', errors='replace',
    )
    if r.returncode != 0:
        print(f'cargo modules structure failed for {crate}:\n{r.stderr}', file=sys.stderr)
        return ''
    return r.stdout


def extract_pub_symbols(
    crate: str, pkg: dict, workspace_dir: Path, symbol_types: list[str]
) -> dict[str, list[str]]:
    raw = _run_structure(crate, pkg, workspace_dir, symbol_types)
    symbols: dict[str, list[str]] = {t: [] for t in symbol_types}
    for line in _ANSI.sub('', raw).splitlines():
        m = _ITEM_RE.search(line)
        if m:
            kind, name = m.group(1), m.group(2)
            if kind in symbols and name not in symbols[kind]:
                symbols[kind].append(name)
    return symbols


# ---------------------------------------------------------------------------
# Mermaid generation
# ---------------------------------------------------------------------------

_PREFIX = {'struct': 'S', 'enum': 'E', 'trait': 'T', 'fn': 'F', 'type': 'A'}


def _safe(name: str) -> str:
    return name.replace('-', '_')


def generate_diagram(
    packages: dict[str, dict],
    crate_filter: list[str],
    symbol_types: list[str],
    workspace_dir: Path,
) -> str:
    workspace_names = set(packages.keys())

    if crate_filter:
        missing = [n for n in crate_filter if n not in packages]
        if missing:
            print(f'Warning: unknown crates ignored: {", ".join(missing)}', file=sys.stderr)
        selected = {n: packages[n] for n in crate_filter if n in packages}
    else:
        selected = packages

    lines = ['classDiagram']

    for name in sorted(selected):
        safe = _safe(name)
        symbols: dict[str, list[str]] = {}
        if symbol_types:
            symbols = extract_pub_symbols(name, selected[name], workspace_dir, symbol_types)

        members = [
            f'        +{sym} [{_PREFIX[t]}]'
            for t in symbol_types
            for sym in sorted(symbols.get(t, []))
        ]
        if members:
            lines.append(f'    class {safe} {{')
            lines.extend(members)
            lines.append('    }')
        else:
            lines.append(f'    class {safe}')

    lines.append('')

    for name in sorted(selected):
        safe = _safe(name)
        for dep in sorted(internal_deps(selected[name], workspace_names)):
            if dep in selected:
                lines.append(f'    {safe} ..> {_safe(dep)} : uses')

    return '\n'.join(lines)


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main() -> None:
    parser = argparse.ArgumentParser(
        prog='rust-mermaid',
        description='Generate a Mermaid classDiagram from a Rust workspace.',
    )
    parser.add_argument('crates', nargs='*', help='Workspace crate names to include')
    parser.add_argument('--preset',  metavar='NAME', help='Named preset in rust-mermaid.toml')
    parser.add_argument('--symbols', default='struct,enum,trait',
                        help='Symbol types: struct,enum,trait,fn,type  (default: struct,enum,trait)')
    parser.add_argument('--no-symbols', action='store_true',
                        help='Show crate nodes only')
    parser.add_argument('--title',     metavar='TEXT',
                        help='Diagram title for .md output (default: preset name or "All Crates")')
    parser.add_argument('--out',       metavar='FILE', help='Output file (default: stdout)')
    parser.add_argument('--workspace', metavar='DIR',  default='.',
                        help='Workspace root (default: cwd)')
    args = parser.parse_args()

    workspace_dir = Path(args.workspace).resolve()

    config: dict = {}
    config_path = workspace_dir / 'rust-mermaid.toml'
    if config_path.exists():
        config = _load_toml(config_path)

    crate_filter: list[str] = list(args.crates)
    if not crate_filter and args.preset:
        crate_filter = config.get('preset', {}).get(args.preset, {}).get('crates', [])
        if not crate_filter:
            print(f"Error: preset '{args.preset}' not found in rust-mermaid.toml", file=sys.stderr)
            sys.exit(1)

    symbol_types: list[str] = []
    if not args.no_symbols:
        valid = {t for t in _KINDS}
        symbol_types = [s.strip() for s in args.symbols.split(',') if s.strip() in valid]
        unknown = [s.strip() for s in args.symbols.split(',') if s.strip() and s.strip() not in valid]
        if unknown:
            print(f"Warning: unknown symbol types ignored: {', '.join(unknown)}", file=sys.stderr)

    meta     = cargo_metadata(workspace_dir)
    packages = workspace_packages(meta)
    diagram  = generate_diagram(packages, crate_filter, symbol_types, workspace_dir)

    # Wrap in GitHub-renderable markdown when writing to a .md file
    out_path = Path(args.out) if args.out else None
    if out_path and out_path.suffix == '.md':
        title = args.title or (args.preset.capitalize() if args.preset else 'All Crates')
        regen_cmd = ' '.join(
            ['python tools/rust-mermaid.py']
            + (['--preset', args.preset] if args.preset else [])
            + ['--symbols', args.symbols]
            + ['--out', args.out]
        )
        output = (
            f'<!-- AUTO-GENERATED — do not edit manually.\n'
            f'     Regenerate: {regen_cmd} -->\n\n'
            f'# Crate Architecture — {title}\n\n'
            f'```mermaid\n'
            f'{diagram}\n'
            f'```\n'
        )
    else:
        output = diagram + '\n'

    if out_path:
        out_path.write_text(output, encoding='utf-8')
        print(f'Written to {args.out}', file=sys.stderr)
    else:
        print(output)


if __name__ == '__main__':
    main()
