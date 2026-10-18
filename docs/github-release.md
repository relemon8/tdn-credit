# GitHubで公開する手順

公開するソースと、遊ぶための実行ファイルは分けて配置します。
リポジトリにはソース、README、LICENSE、Cargo.lock、docs、scripts、.githubを入れ、
実行ファイルと圧縮ファイルはReleasesへ添付します。セーブやAPIキーは含めません。

## 自分で公開する場合

1. GitHubでアカウントを作りログインします。
2. 新規リポジトリを作成します。例: `tdn-credit`。公開するならPublicを選択し、READMEやLicenseの自動追加は選びません。同梱済みです。
3. GitHub CLIを使う場合は、ターミナルで `gh auth login --hostname github.com --git-protocol https --web` を実行し、自分のブラウザで認証します。パスワードやトークンをチャットやソースへ貼らないでください。
4. ソースのフォルダで `git init -b main` → `git add .` → `git commit -m "Release TDN CREDIT v0.1.1"` を実行します。
5. `git remote add origin https://github.com/自分のユーザー名/tdn-credit.git` → `git push -u origin main` でソースを登録します。
6. リポジトリのReleasesから新規Releaseを作成し、タグを `v0.1.1` にします。
7. OS/CPU別zipまたはtar.gz、ソースの圧縮ファイル、SHA256SUMS、RELEASE_MANIFEST.jsonを添付します。更新内容はCHANGELOGを参照できます。
8. 公開前に各添付ファイルの対象OSと動作確認状況を確認します。異なる版のバイナリを同じReleaseへ混ぜないでください。

GitHub CLIがない場合、公開用ソースの中身をGitHubのAdd file → Upload filesで登録する方法もあります。
隠しフォルダ`.github`を含めるにはGitを使う方が確実です。

## この開発環境へ任せる場合

GitHub認証を自分で済ませ、公開先のリポジトリURLを伝えてください。
こちらでソースの登録、ビルド結果の確認、Releaseへの配布物添付を実施できます。
このファイルに書かれた手順だけでは、GitHub上への公開は行われません。

## 各OSの配布物を作る

`scripts/package.sh` はローカルOS用をビルドして、README、ライセンス、起動スクリプトと一緒に圧縮します。
開発者側ではPython 3が必要ですが、遊ぶ人には不要です。

```sh
./scripts/package.sh
python3 scripts/package-release.py --source
```

既に作ったクロスビルドをまとめる場合:

```sh
python3 scripts/package-release.py --binary target/x86_64-unknown-linux-musl/release/tdn-credit --target x86_64-unknown-linux-musl --status "Linux実行確認済み"
```

GitHub Actionsの「Release packages」を手動実行すると、Linux → OpenBSD → macOS → Android Core → Windowsの順に検証し、成功したOSの配布アーカイブをArtifactsへ保存します。
これはReleaseを自動公開する処理ではありません。未成功のOSやCoreだけの検証を、実行確認済みの製品として扱わないでください。

参考: [GitHub公式のランナー一覧](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)、[Artifactsの使い方](https://docs.github.com/en/actions/tutorials/store-and-share-data)。
