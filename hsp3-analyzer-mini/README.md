# HSP3 アナライザー・ミニ

HSP3 の実用最小限な静的解析ツールです。Language Server Protocol (LSP) 準拠。

機能などは [vscode-ext/README.md](./vscode-ext/README.md) を参照。

- [ham-lsp](./ham-lsp): 解析機能および LSP サーバーの実装
- [vscode-ext](./vscode-ext): VSCode 拡張機能

## 開発環境

この拡張機能を開発する環境の構築手順は以下の通りです。

以下のツールをインストールしてください。

- [Node.js](https://nodejs.org)
    - node, npm
- [Rust](https://rustlang.org)
    - rustup, rustc, cargo

次に必要なパッケージのインストールを行います。

```sh
cd vscode-ext
npm install
```

以上で環境構築は完了です。

ビルドやインストールのコマンドは以下の通りです。

```sh
# LSP (Rust) のビルド
cargo build

# 拡張機能のビルド
cd vscode-ext
npm install
npm run build
```

開発版のインストール

```sh
./install
```

アンインストール:

```sh
./uninstall
```

## 参照

- [language-hsp3](https://github.com/honobonosun/vscode-language-hsp3)
    - リンク先の手順に従うと VSCode 上で HSP のデバッグ実行などが可能になるようです。
- [LSP学習記 #1](https://qiita.com/vain0x/items/d050fe7c8b342ed2004e)
    - LSP の学んだことをまとめた連載記事です。
