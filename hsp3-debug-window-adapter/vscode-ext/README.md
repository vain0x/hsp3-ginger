# HSP3 Debug Window Adapter for VSCode

HSP3 のデバッグ機能を有効化する VSCode 拡張機能です。VSCode のデバッグツールではなく、HSP3 標準のデバッグウィンドウを使用します。

## インストール

VSCode にて以下の拡張機能をインストールしてください。

- `vain0x.hsp3-vscode-syntax`
- `vain0x.hsp3-debug-window-adapter`

設定を開き、HSP3 のインストールディレクトリを設定してください。例:

```json
{
    "hsp3-debug-window-adapter.hsp3-root": "C:/Program Files (x86)/hsp351"
}
```

## 開発環境

この拡張機能の開発環境を構築する手順は以下の通りです。

- Node.js をインストールしてください。

PowerShell でこのディレクトリを開き、以下のコマンドを実行します。

```pwsh
npm install
```

開発中の拡張機能のインストール・アンインストールは以下のスクリプトを使用します。

```pwsh
# インストール
./install.ps1
```

```pswh
# アンインストール
./uninstlal.ps1
```
