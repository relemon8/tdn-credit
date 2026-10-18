#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Collect already packaged targets, checksums, clean source, and platform notes."""
import argparse,hashlib,json,re,shutil
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
VERSION=re.search(r'^version = "([^"]+)"',(ROOT/'Cargo.toml').read_text(),re.M)[1]
LABELS={
 'x86_64-unknown-linux-musl':'Linux-x86_64',
 'aarch64-unknown-linux-musl':'Linux-ARM64',
 'aarch64-unknown-openbsd-7.9':'OpenBSD-7.9-ARM64',
 'x86_64-unknown-openbsd-7.9':'OpenBSD-7.9-x86_64',
 'aarch64-apple-darwin':'macOS-AppleSilicon',
 'x86_64-apple-darwin':'macOS-Intel-EXPERIMENTAL',
 'x86_64-pc-windows-gnu':'Windows-x64-EXPERIMENTAL',
 'x86_64-pc-windows-msvc':'Windows-x64',
}
def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--out',type=Path,required=True);args=p.parse_args()
 dist=ROOT/'dist'/VERSION;args.out.mkdir(parents=True,exist_ok=True)
 downloads=args.out/'Downloads';downloads.mkdir(exist_ok=True)
 records=[]
 for path in sorted(dist.glob('*.manifest.json')):
  info=json.loads(path.read_text())
  if info['version']!=VERSION:raise ValueError('mixed release versions')
  target=info['target'];name=f'tdn-credit-{VERSION}-{target}';src=dist/name
  label='Source/tdn-credit' if target=='source' else LABELS.get(target,target)
  dest=args.out/label
  if dest.exists():shutil.rmtree(dest)
  dest.parent.mkdir(parents=True,exist_ok=True);shutil.copytree(src,dest)
  for archive in info['archives']:
   source=dist/archive['file']
   if digest(source)!=archive['sha256']:raise ValueError('checksum mismatch')
   shutil.copy2(source,downloads/source.name)
  records.append(info)
 guides=args.out/'Platform-Guides';guides.mkdir(exist_ok=True)
 (guides/'Android.md').write_text('''# Android\n\nUIはスマートフォンの幅に対応していますが、APKやネイティブアプリはまだ配布していません。\nAndroid向けRust Coreのコンパイル確認までで、実機起動は未検証です。\nゲームはローカルRustサーバーを必要とし、HTMLだけを端末へコピーしても起動しません。\n将来のAndroid版も同じSimulation Coreを利用する方針です。\n''')
 (guides/'OpenBSD.md').write_text('''# OpenBSD\n\n7.9の対応CPU用バイナリが同梱されている場合はBUILD_INFO.jsonを確認してください。\n対応CPUがない場合はSource/tdn-creditを使い、OpenBSD上で次を実行します。\n\n```sh\ndoas pkg_add rust python3\ncargo test --workspace --locked\ncargo build --release --locked -p tdn-credit\n./target/release/tdn-credit --portable --no-open\n```\n\nブラウザで http://127.0.0.1:3000 を開きます。\n''')
 (guides/'Updating.md').write_text('''# 保存と更新\n\nstart.sh、TDN-CREDIT.command、start.batは配布フォルダ内のdataへ保存します。\n版を更新する場合はゲームを終了してから、旧フォルダのdataを新しいフォルダへコピーしてください。\n実行ファイルを引数なしで起動するとOSの標準保存先を使い、以前の同じ保存先を再利用できます。\nv0.1.1は旧版の互換セーブを読めますが、24か月を超えて進めたセーブは旧版では読めません。\n''')
 shutil.copy2(ROOT/'docs/github-release.md',args.out/'GitHub公開手順.md')
 shutil.copy2(ROOT/'LICENSE',args.out/'LICENSE')
 rows=['| 対象 | フォルダ | この配布物の確認状況 |','|---|---|---|']
 for info in records:
  if info['target']!='source':rows.append(f"| {info['target']} | {LABELS.get(info['target'],info['target'])} | {info['verification']} |")
 (args.out/'README.md').write_text(f'''# TDN CREDIT v{VERSION} 公開用セット\n\nまず、自分のOS・CPU用のフォルダを開き「はじめに.txt」を読んでください。\nLinuxはstart.sh、macOSはTDN-CREDIT.command、Windowsはstart.batから起動できます。\nブラウザで http://127.0.0.1:3000 を開きます。\n\n'''+ '\n'.join(rows)+'''\n\nEXPERIMENTALはビルドのみで実機未検証です。AndroidのAPKは含まれていません。\nOpenBSDの未同梱CPUはソースからビルドできます。各フォルダのBUILD_INFO.jsonが検証の記録です。\n\n## GitHubへ載せるもの\n\n- リポジトリ: Source/tdn-creditの中身。README・LICENSE・CI・全ソースを含みます。\n- Releases添付: Downloadsの対応OS用zipまたはtar.gz、ソース圧縮ファイル、SHA256SUMS、RELEASE_MANIFEST.json。\n- 詳細はGitHub公開手順.mdを参照してください。このセットを作るだけでは自動公開しません。\n\n## ゲームについて\n\n初期65億円、架空10社・初期20債券。24か月の評価後も同じ資産で継続できます。\n完全無料・MIT。開発にはOpenAI Codexを使用しています。課金・広告・外部API・アカウント不要。\n資金・損益は架空です。実際のお金は使用せず、換金もできません。投資助言ではありません。\n''')
 manifest={'application':'TDN CREDIT','version':VERSION,'test_count':40,'artifacts':records,'repository':'https://github.com/relemon8/tdn-credit','release':'https://github.com/relemon8/tdn-credit/releases/tag/v0.1.1'}
 (args.out/'RELEASE_MANIFEST.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n')
 checks=[f"{digest(path)}  Downloads/{path.name}" for path in sorted(downloads.iterdir()) if path.is_file()]
 checks.append(f"{digest(args.out/'RELEASE_MANIFEST.json')}  RELEASE_MANIFEST.json")
 (args.out/'SHA256SUMS').write_text('\n'.join(checks)+'\n')
 print(args.out)
if __name__=='__main__':main()
