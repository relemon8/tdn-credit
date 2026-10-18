# v0.1.1の配布・検証

アプリ版0.1.1、セーブ互換用エンジン0.1.0、データ2、スキーマ1。

- macOS arm64: 全40テスト、24か月のHTTPリプレイ、語録ON/OFFの金融一致、Firefox全5画面（1440px・390px）を確認。
- Linux x86_64 / aarch64: 新規musl静的ビルドをQEMU Linux内で実行。24か月ヘッドレス、40か月HTTP進行、借り換え債の生成、手動セーブ・ロード、プロセス再起動後の復元に成功。ゲストに外部ネットワークなし。
- OpenBSD 7.9 arm64: ネイティブ環境で全40テスト、24か月ヘッドレス、40か月HTTP操作・新債生成・売買・保存復元を確認。配布バイナリはdev profileをstripしたもの（最適化なし）。
- macOS x86_64: クロスビルド成功。Intel実機とRosetta実行は未確認。
- Windows x86_64 GNU: PE実行ファイルのクロスビルド成功。Windowsでの実行・ブラウザ・保存パスは未検証。試験版扱い。
- Android: aarch64向けCoreコンパイル確認。APKは未作成、単体実機起動は未検証。

OpenBSDなど追加の確認結果は、各配布フォルダのBUILD_INFO.jsonと配布全体のRELEASE_MANIFEST.jsonに記録します。
過去版のテスト実績を、そのまま新しいバイナリの動作保証には使いません。

macOSの配布物は署名・公証を行っていません。バイナリの起動にOS側の確認が必要になる場合があります。
GitHub ActionsのRelease packagesを手動実行すると、OSごとのネイティブ検証と配布物生成を行えます。
Windows/Coreのみの検証を、Androidアプリの完成と表示しません。

配布ファイルのSHA256はSHA256SUMSへ記載します。ソース配布には個人のセーブ、.git、target、環境設定ファイルを含めません。
