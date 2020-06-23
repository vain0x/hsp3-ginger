# HSP3 Syntax for VSCode

HSP3 の言語・文法を VSCode に登録するための拡張機能です。

----

## 開発環境

この拡張機能を開発するための環境は以下の通りです。

次のアプリが必要です:

- VSCode
- [Node.js](https://nodejs.org)

はじめに Git Bash で `./setup` を実行してください。

その他のスクリプト:

- `./install`: 開発版の拡張機能を VSCode にインストールします。すでにインストールしたことのあるバージョン番号だと、中身が更新されないことがあります。
- `./uninstall`: 拡張機能をアンインストールします。VSCode の再起動が必要です。

## 参考

- [honobonosun/language-hsp3\: AtomにHSP3言語の構文色分けとコマンド実行を提供するパッケージ。](https://github.com/honobonosun/language-hsp3)
    - 文法の定義はこちらからお借りしたものの再配布になります。
- [honobonosun/vscode-language-hsp3\: VSCode 向けの HSP3 シンタックスハイライトとその他の機能を提供する拡張機能](https://github.com/honobonosun/vscode-language-hsp3)
    - VSCode 用の HSP3 言語パッケージの別実装です。
    - 差異としてはコメントとして `;` を優先して使う点などがあります。
