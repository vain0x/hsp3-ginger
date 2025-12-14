# リリース

新しいバージョンをリリースするときの手順

- 作業が完了済みであることを確認する
    - TODO, FIXME コメント、デバッグ出力の消し忘れがないこと
    - テストが通ること
    - ドキュメントを更新すること
    - 機能が動くこと、不具合が再現しないこと
- ビルドとパッケージ作成のスクリプトを実行する

```sh
# vscode-ext/ham.vsix が生成される
./install
```

- 変更履歴 (CHANGELOG.md) を更新する
- バージョン番号を更新する
    - 変更箇所は前のバージョン番号で検索して見つける
- 作業を main ブランチにマージする
- マージコミットにバージョン番号のタグをつける
    - 例: `git tag v1.0.0`
- パッケージを公開 (publish) する
    - [Publishing Extensions (VSCode)](https://code.visualstudio.com/api/working-with-extensions/publishing-extension)
