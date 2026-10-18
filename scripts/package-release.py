#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Build-time only: package one binary or a clean source tree; never includes saves."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import tarfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]
VERSION = re.search(r'^version = "([^"]+)"', (ROOT / 'Cargo.toml').read_text(), re.M)[1]

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def archives(folder):
    tarpath = folder.with_name(folder.name + '.tar.gz')
    with tarfile.open(tarpath, 'w:gz', format=tarfile.PAX_FORMAT) as archive:
        
        def normalized(info):
            if Path(info.name).name == '.DS_Store': return None
            info.uid = info.gid = 0
            info.uname = info.gname = ''
            return info
        archive.add(folder, arcname=folder.name, filter=normalized)
    zippath = folder.with_name(folder.name + '.zip')
    with zipfile.ZipFile(zippath, 'w', zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        for path in sorted(folder.rglob('*')):
            if path.is_file():
                archive.write(path, str(Path(folder.name) / path.relative_to(folder)))
    return [{'file': p.name, 'sha256': digest(p)} for p in [tarpath, zippath]]

def clean_copy(src, dst):
    shutil.copytree(src, dst, ignore=shutil.ignore_patterns('target', '.git', '__pycache__', '*.pyc', '.DS_Store', '*.sqlite*', '.env', '.env.*'))

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path)
    parser.add_argument('--target')
    parser.add_argument('--source', action='store_true')
    parser.add_argument('--status', default='build-only; runtime verification pending')
    parser.add_argument('--out', type=Path, default=ROOT / 'dist' / VERSION)
    args = parser.parse_args()
    if not args.source and (not args.binary or not args.target):
        parser.error('specify --source, or --binary and --target')
    args.out.mkdir(parents=True, exist_ok=True)
    label = 'source' if args.source else args.target
    if not re.fullmatch(r'[a-zA-Z0-9_.-]+', label):
        parser.error('invalid target label')
    folder = args.out / f'tdn-credit-{VERSION}-{label}'
    if folder.exists():
        shutil.rmtree(folder)
    folder.mkdir()
    for name in ['README.md', 'LICENSE', 'THIRD_PARTY_NOTICES.md', 'CHANGELOG.md']:
        shutil.copy2(ROOT / name, folder / name)
    clean_copy(ROOT / 'docs', folder / 'docs')
    clean_copy(ROOT / 'assets' / 'licenses', folder / 'licenses')
    info = dict(application='TDN CREDIT', version=VERSION, target=label, verification=args.status,
                save_schema=1, save_engine='0.1.0', save_data=2)
    if args.source:
        for name in ['Cargo.toml', 'Cargo.lock', 'deny.toml', '.gitignore', '.gitattributes']:
            shutil.copy2(ROOT / name, folder / name)
        for name in ['app', 'crates', 'assets', 'scripts', '.github']:
            clean_copy(ROOT / name, folder / name)
    else:
        binary_name = 'tdn-credit.exe' if 'windows' in label else 'tdn-credit'
        shutil.copy2(args.binary, folder / binary_name)
        (folder / binary_name).chmod(0o755)
        info['binary_sha256'] = digest(folder / binary_name)
        quick = f'''TDN CREDIT {VERSION} / {label}\n\n動作確認状況: {args.status}\n\n【起動】\n展開したフォルダで ./tdn-credit --portable を実行します。\nWindowsは tdn-credit.exe、または start.bat を起動します。\nブラウザで http://127.0.0.1:3000 を開きます。Ctrl+Cで終了。\n同梱の起動スクリプトを使うと、このフォルダ内のdataに保存します。\n\n【必要なもの】\n対応OSとブラウザだけ。Node、Rust、Python、APIキーは不要です。\n\n【ゲーム】\n初期65億円。24か月で成績評価し、その後も継続できます。\n画面下部の遊び方から30秒版・詳しい版を読めます。\n\n【免責】\n資金・企業・証券は架空です。実際のお金は使わず換金もできません。\n投資助言ではありません。完全無料・MITライセンス。OpenAI Codexを使用して開発しています。\n詳細はREADME.md、LICENSE、THIRD_PARTY_NOTICES.mdを参照してください。\n'''
        (folder / 'はじめに.txt').write_text(quick, encoding='utf-8')
        if 'windows' in label:
            (folder / 'start.bat').write_bytes(b'@echo off\r\ncd /d "%~dp0"\r\ntdn-credit.exe --portable\r\npause\r\n')
        else:
            launcher = '#!/bin/sh\nset -eu\ncd -P "$(dirname "$0")"\nexec ./tdn-credit --portable "$@"\n'
            (folder / 'start.sh').write_text(launcher)
            (folder / 'start.sh').chmod(0o755)
            if 'apple' in label:
                shutil.copy2(folder / 'start.sh', folder / 'TDN-CREDIT.command')
    (folder / 'BUILD_INFO.json').write_text(json.dumps(info, ensure_ascii=False, indent=2) + '\n')
    info['archives'] = archives(folder)
    (args.out / f'{folder.name}.manifest.json').write_text(json.dumps(info, ensure_ascii=False, indent=2) + '\n')
    print(folder)

if __name__ == '__main__':
    main()
