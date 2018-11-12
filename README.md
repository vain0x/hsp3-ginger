# GINGER the HSP3 Debugger

*WIP: 開発中*

## 概要

*GINGER* は、VSCode で HSP3 のスクリプトのデバッグ実行を可能にするプロジェクト。

## 使いかた

まだ使えません。

## 構造

- `hsp3debug` (hsp3-debug-ginger)
    - C++。Windows 用の DLL を生成する。
    - HSPランタイムからアタッチされる。
    - `adapter` にアタッチして、イベントの仲介を行う。
    - NOTE: HSPランタイムが `adapter` に直接アタッチするようにしたほうがいいが、できなさそう。
- `adapter` (hsp3-debug-adapter)
    - Rust。Windows 用の DLL を生成する。
    - hsp3debug からアタッチされる。
    - WebSocket クライアントを実行して、VSCode 側と直接やりとりする。
    - HSPランタイムの内部状態を解析して、デバッガーアダプタープロトコルにのっとったメッセージを生成する。
- `vscode-ext` (vscode-hsp3-debug-ginger)
    - TypeScript
    - VSCode のデバッガー拡張機能から呼び出される。
    - WebSocket サーバーを実行して、 `adapter` とやりとりする。

## 開発

GINGER の開発者のためのメモ: [dev.md](./dev.md)
