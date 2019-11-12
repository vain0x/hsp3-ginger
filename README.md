# HSP3 GINGER

HSP3 開発ツールを作るプロジェクト。

## プロジェクト

### hsp3-debug-empty

[projects/hsp3-debug-empty](projects/hsp3-debug-empty)

- 何もしないデバッガー
- 新しいデバッガーを作るときの土台
- 言語: C++

### hsp3-debug-ginger

[projects/hsp3-debug-ginger](projects/hsp3-debug-ginger)

- VSCode 用デバッガー
- Debug Adapter Protocol 対応
- 言語: Rust
- 状況: アルファ版リリース済み。まだ実用レベルではありません。

### hsp3-debug-spider

[projects/hsp3-debug-spider](projects/hsp3-debug-spider)

- デバッガー
- Web サーバーとブラウザを起動することで、GUI を HTML/CSS により実装しています。
- 言語: Rust (サーバー),　JavaScript (クライアント), C# (ブラウザ)
- 状況: 概念実証 (proof-of-concept) 済み。実用レベルではありません。

### hsp3-ginger

[projects/hsp3-ginger](projects/hsp3-ginger)

- コマンドラインコンパイラを作ろうとしていたもの。
- 状況: 作業途中

### vscode-ext

[editors/vscode-ext](editors/vscode-ext)

- VSCode 拡張機能
- 言語: TypeScript
- 状況: シンタックスハイライトのみ
- 備考: [honobonosun/vscode-language-hsp3](https://github.com/honobonosun/vscode-language-hsp3) を使ってください。

## サブツリー

lib/ 以下は他のリポジトリのコードの再配布です。

### lib/language-hsp3:

[language-hsp3](https://github.com/honobonosun/language-hsp3)

- vscode-ext から参照されます。
