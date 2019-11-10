# ビルドスクリプト

このディレクトリには開発者用のスクリプトが置いてあります。

スクリプトはカレントディレクトリがソリューションのディレクトリであることを前提としているので、`./scripts/build-all.ps1` のような形で使ってください。

## ツール: MSBuild

ビルドスクリプトを使うには、MSBuild にパスを通す必要があります。

- MSBuild はソリューションやプロジェクトのビルド等をコマンドラインから行うツールです。Visual Studio に同梱されているはずです。
- パスを通すには、「Win+Break → システムの詳細設定 → 環境変数 → 新規」から環境変数に MSBuild.exe のあるディレクトリを追加してください。変更を適用するには、アプリを閉じて開きなおす必要があります。
- Visual Studio 2019 の場合、MSBuild はここにあります: `"C:\Program Files (x86)\Microsoft Visual Studio\2019\Community\MSBuild\Current\Bin\"`
