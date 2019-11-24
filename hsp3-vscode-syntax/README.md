# HSP3 Syntax for VSCode

HSP3 の言語・文法を VSCode に登録するための拡張機能です。

## 開発環境

- [Node.js](https://nodejs.org)

PowerShell を起動して以下のコマンドを使用してください。

```pwsh
npm install
./setup.ps1
```

## インストール

インストール:

```pwsh
./install.ps1
```

アンインストール:

```pwsh
./uninstall.ps1
```

## 参考

- [honobonosun/language-hsp3\: AtomにHSP3言語の構文色分けとコマンド実行を提供するパッケージ。](https://github.com/honobonosun/language-hsp3)
    - 文法の定義はこちらからお借りしたものの再配布になります。
- [honobonosun/vscode-language-hsp3\: VSCode 向けの HSP3 シンタックスハイライトとその他の機能を提供する拡張機能](https://github.com/honobonosun/vscode-language-hsp3)
    - VSCode 用の HSP3 言語パッケージの別実装です。
    - 差異としてはコメントとして `;` を優先して使う点などがあります。
