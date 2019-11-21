# ビルドスクリプト

このディレクトリには開発者用のスクリプトが置いてあります。

スクリプトはカレントディレクトリがソリューションのディレクトリであることを前提としているので、`./scripts/build-all.ps1` のような形で使ってください。

## 準備

- PowerShell 6 をインストールしてください。

### 準備: 環境変数 HSP3_ROOT

一部のスクリプトは環境変数 HSP3_ROOT を必要とします。HSP のディレクトリへの絶対パスを指定してください。

### 準備: MSBuild

ビルドスクリプトを使うには、MSBuild にパスを通す必要があります。

- MSBuild はソリューションやプロジェクトのビルド等をコマンドラインから行うツールです。Visual Studio に同梱されているはずです。
- パスを通すには環境変数に MSBuild.exe のあるディレクトリを追加してください。
- Visual Studio 2019 の場合、MSBuild はここにあります: `"C:\Program Files (x86)\Microsoft Visual Studio\2019\Community\MSBuild\Current\Bin\"`

## 開発者用インストール

`./scripts/install-dev.ps1` でデバッグビルドにより生成された DLL へのシンボリックリンクを配置できます。

アンインストールは `./scripts/uninstall-dev.ps1` から行えます。(インストールしたリンクが削除され、代わりにバックアップされたデバッガーが復元されます。)

## ビルド

`./scripts/build-all.ps1` ですべての構成をビルドできます。
