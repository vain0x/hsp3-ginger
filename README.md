# HSP3 GINGER

HSP3 の開発ツールを作るプロジェクト。

## 開発ツール

### hsp3-ginger

[hsp3-ginger](hsp3-ginger)

- コマンドラインで HSP3 スクリプトのコンパイルや実行を行うためのユーティリティーです。
- 言語: HSP3

### hsp3-debug-window-adapter

[hsp3-debug-window-adapter](hsp3-debug-window-adapter)

- HSP3 のデバッグ実行を起動・終了するだけの Debug Adapter Protocol (DAP) 実装です。
- VSCode などの DAP に対応したエディタにて、標準のデバッグウィンドウを用いてデバッグ実行するのに使用します。
- 言語: TypeScript

### hsp3-analyzer-mini

[hsp3-analyzer-mini](hsp3-analyzer-mini)

- HSP3 の実用最小限な静的解析ツールです。Language Server Protocol (LSP) 準拠。
- VSCode に拡張機能をインストールすることで入力補完やホバーなどの支援を受けられます。
- 言語: Rust
- 状況: 最低限の実用可能なレベル

---
---

以下のプロジェクトは実用段階にはありません。

## 開発ツール \[開発中\]

### hsp3-debug-ginger

[hsp3-debug-ginger](hsp3-debug-ginger)

- VSCode 用デバッガー
- Debug Adapter Protocol 対応
- 言語: Rust
- 状況: アルファ版リリース済み。まだ実用レベルではありません。

### hsp3-forgery

[hsp3-forgery](hsp3-forgery)

- 静的解析ツール
- 言語: Rust
- 状況: まだ構文解析の一部しか動きません。

## 開発ツール \[サンプル\]

### hsp3-debug-empty

[hsp3-debug-empty](hsp3-debug-empty)

- 何もしないデバッガー
- 新しいデバッガーを作るときの土台
- 言語: C++

### hsp3-debug-self

[hsp3-debug-self](hsp3-debug-self)

- サーバーとクライアントに分離したデバッガー
- 言語: C++ (サーバー), HSP (クライアント)
- 状況: 概念実証レベル。コンセプトは [knowbug v2](https://github.com/vain0x) に引き継がれました。

### hsp3-debug-spider

[hsp3-debug-spider](hsp3-debug-spider)

- デバッガー
- Web サーバーとブラウザを起動することで、GUI を HTML/CSS により実装しています。
- 言語: Rust (サーバー),　JavaScript (クライアント), C# (ブラウザ)
- 状況: 概念実証 (proof-of-concept) 済み。実用レベルではありません。

## 拡張プラグイン \[開発中\]

### hsp3-vartype-int64

[hsp3-vartype-int64](hsp3-vartype-int64)

- int64 型を追加するプラグイン

---
---

以下のプロジェクトは開発終了しました

### \[開発終了\]

### ~~hsp3-vscode-syntax~~

[hsp3-vscode-syntax](hsp3-vscode-syntax)

- language-hsp3 など、他の拡張機能を利用してください
