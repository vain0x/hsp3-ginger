# 更新履歴

([GitHub Releases](https://github.com/vain0x/hsp3-ginger/releases) も参照。)

## [0.5.0] - 2024-01-25

#### 追加

- ドキュメントシンボル機能を無効化するための設定を追加しました。
    - `hsp3-analyzer-mini.documentSymbol.enabled: false` を指定すると無効になります
    - 「アウトライン」が生成されなくなります

#### 修正

- 「VC++ランタイムライブラリ」をインストールしていない環境でも動作するように変更しました。([#10](https://github.com/vain0x/hsp3-ginger/issues/10))
    (HAMの実行ファイルにランタイムライブラリが静的リンクされるようになります。)
- ラベルのシンボルが重複して定義される問題を修正しました。([#11](https://github.com/vain0x/hsp3-ginger/issues/11))

## [0.4.0] - 2021-11-01

- セマンティックハイライトを実装しました。
    - (トークンの種類による色分け)

→ [merge](https://github.com/vain0x/hsp3-ginger/commit/c5924b60686d4bf1769569c013c8110f7636732c)

## [0.3.1] - 2021-06-04

- 構文解析器を改善しました。
    - 以前のバージョンの構文解析器は複数行の文字列リテラルなどを処理できませんでしたが、できるようになりました。
- 新機能を追加しました。
    - 入力補完
    - シグネチャヘルプ
    - コードフォーマット
    - コードアクション
        - インクルードガードを生成する機能
        - カンマの両側を交換する機能
- \[実験\] 構文リント、意味検査

## [0.3.0]


→ [merge](https://github.com/vain0x/hsp3-ginger/commit/d2788085d71c8d8fdf31e445a8e262c08e18fba8)

# [0.2.1] - 2020-06-04


→ [merge](https://github.com/vain0x/hsp3-ginger/commit/a12e2e2d0871a6900ccea753d024317bc33692c7)

## [0.1.1] - 2019-11-29


- 文字列リテラルの内容次第でクラッシュするバグを修正

## [0.1.0] - 2019-11-23

- 初回リリース



> [keep a changelog](https://keepachangelog.com/ja/)

[0.5.0]: https://github.com/vain0x/hsp3-ginger/releases/tag/hsp3-analyzer-mini-v0.5.0
[0.4.0]: https://github.com/vain0x/hsp3-ginger/releases/tag/hsp3-analyzer-mini-v0.4.0
[0.3.1]: https://github.com/vain0x/hsp3-ginger/releases/tag/hsp3-analyzer-mini-v0.3.1
[0.3.0]: https://github.com/vain0x/hsp3-ginger/releases/tag/hsp3-analyzer-mini-v0.3.0
[0.2.1]: https://github.com/vain0x/hsp3-ginger/releases/tag/hsp3-analyzer-mini-v0.2.1
[0.1.1]: https://github.com/vain0x/hsp3-ginger/releases/tag/hsp3-analyzer-mini-v0.1.1
[0.1.0]: https://github.com/vain0x/hsp3-ginger/releases/tag/hsp3-analyzer-mini-v0.1.0
