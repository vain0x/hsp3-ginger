# Change Log
このプロジェクトのすべての注目すべき変更は、このファイルに記録されます。  
All notable changes to this project will be documented in this file.

## [Unreleased]
### Added
- [ ] コマンド実行時に、プロジェクト内に未保存の変更があるなら、そのファイル名を通知する。
- [x] よく使う定型文（スイッチ文、ループ文など）をスニペットに追加する。

### Changed

## 0.2.2 - 2019-03-31
- *fix* シングルクォーテーションのハイライトが貪欲になるのを修正しました。[#9](https://github.com/honobonosun/language-hsp3/issues/9)
- *add* シングルクォーテーション内でエスケープシーケンスがおかしい場合、invalid.illegal でハイライトされるように機能追加しました。

## 0.2.1 - 2019-02-07
- *fix* ハイライトされていないキーワードを発見して、ハイライトされるように修正しました。[#5](https://github.com/honobonosun/language-hsp3/issues/5#issuecomment-461335167)

## 0.2.0 - 2018-11-16
- *upgrade* grammarsフォルダのファイルたちの更新が完了しました。
- *feat* labelが位置定義以外でもハイライトされます。
- *update* トークンの認識方法を変更しました。
- *update* 一部のスペースを抜いた省略記述のハイライトに対応しました。（例：\#func name"name"int）
- *add* comdlg32.as定義がハイライトされます。
- *add* a2d.hspの定義がハイライトされます。
- *add* ユーザー定義名の途中で使用できない記号が出現すると、"invalid.illegal.hsp3"スコープ名でハイライトします。\#defineはこの影響を受けません。\#enumは`=`文字に、この影響を受けません。
- *change* 一部のスコープ名が変更されました。一部のシンタックステーマで、見た目が変わります。
  - \#define,\#const,\#enum のユーザー定義名に割当らてたスコープ名は"entity.name.section.hsp3"です。
  - labelのスコープ名は"markup.bold.(asterisk|labelname).hsp3"です。
- *fix* emojiがハイライトされない問題を修正しました。
  - \#define,\#deffunc,\#defcfunc,\#const,\#enum,\#comfunc,\#cmd,\#module,\#modfunc,\#modcfunc のユーザー定義名
  - local変数名
  - \#defineの%tタグ名
- *fix* シングルクォートで、エスケープシーケンスのハイライトが機能していない問題を修正しました。
- *fix* トークンの識別をemojiに対応させて、できるだけ厳格にハイライトします。
- *fix* トークンの60文字制限を[HSP3 Documentの記載通りとなる59文字](http://www.onionsoft.net/hsp/v35/doclib/hspprog.htm#TOLERANCE_LEVEL)へ引き下げました。
- *fix* [トークンが59文字以上になると、"invalid.deprecated.hsp3"スコープ名でハイライトされるように修正しました。](https://github.com/honobonosun/language-hsp3/issues/4)
- *fix* ユーザー定義名の最初の文字が全角の数字でも、"invalid.illegal.hsp3"スコープ名でハイライトされた問題を修正しました。

## 0.1.7 - 2018-11-02
- chell-uoxou様の[プルリクエスト #8](https://github.com/honobonosun/language-hsp3/pull/8)をマージしました。
  - 空白文字を含むパスをhspc.exeに渡せない問題が解決します。
  - exec関数とexecFile関数の切り替えをパッケージ設定画面からできるように変更しました。
- *dev* coffeelint.jsonを追加しました。
  - パッケージ開発者は、node.jsのcoffeelintを導入することで、linterの支援を受けられます。

## 0.1.6 - 2018-05-06
- ユーザー定義名とモジュール名で、入れ子名前空間の記入に対応しました。

## 0.1.5 - 2018-04-30
- モジュール名に複数の名前空間を記入すると正しく認識できない問題を修正しました。
- ユーザー定義名に複数の名前空間を記入すると正しく認識できない問題を修正しました。

## 0.1.4 - 2018-04-08
- Ubuntu環境でShift_JISエンコードのファイルを開いたとき、\\文字が¥文字に置き換えられて、マクロの改行を認識できない問題を修正しました。
- hsファイルの内部スコープを`source.hs`から`text.hs`に変更しました。
- モジュール名に名前空間を記入すると`invalid.illegal`でハイライトされる問題を修正しました。

## 0.1.3
- `apm publish patch`コマンドのリトライ。

## 0.1.2 - 2018-03-10
- HSP3.5で追加された\#packoptのicon,lang,versionオプションを認識できない問題を修正しました。
- \#defineで名前空間がハイライトされない問題を修正しました。

## 0.1.1 - 2018-02-18
- 設定テキストの誤記を修正しました。

## 0.1.0 - 2018-02-16
- 将来のhspcのアップデートに対応するために、引数の引き渡し方法を変更しました。
- 他のツールを使用できるようになりました。
  - ツール側が返す文字は、Shift_JISであることをパッケージは期待します。しかし、それが望めない場合、パッケージは任意のフォーマットでデコードすることで、解決を試みます。
    - デコードできるフォーマットの種類は、_iconv-lite_ が取り扱えるものに限ります。
- 入力補完は、autocomplete-hsp3 パッケージに譲渡しました。
  - autocomplete-hsp3 は開発中のため、現時点では未公開です。
- `#define`などで定義するシンボル名に、色分けが行われるようになりました。
- WinAPIの色分けキーワードに、_advapi32.as_ が追加されました。

## 0.0.3
- ss.pngファイルのパスをgithubに変更。

## 0.0.1 - 2018-01-04
- 初ベータリリース。
