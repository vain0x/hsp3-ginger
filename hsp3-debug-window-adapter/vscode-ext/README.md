# HSP3 Debug Window Adapter for VSCode

HSP3のスクリプトをデバッグ実行できるようにする、VSCode拡張機能です。
VSCodeのデバッグツールではなく、HSP3標準のデバッグウィンドウを使用します。

## インストール

VSCodeで次の拡張機能を探し、インストールしてください。

- `vain0x.hsp3-debug-window-adapter`

設定を開き、HSP3 のインストールディレクトリを設定してください。例:

```json
{
    "hsp3-debug-window-adapter.hsp3-root": "C:/Program Files (x86)/hsp36"
}
```

#### 備考

- インストールディレクトリはシステム変数 `dir_exe` で確認できます。
- パスの区切りは `\\` と書くか、`/` を使ってください。

## 既知の不具合

- shift_jis のソースコード・ランタイムではうまく動かないかもしれません。

## 設定

スクリプトのエンコーディングは自動で判定されます。
実行するスクリプトに以下のどちらかの記述が含まれている場合、スクリプトをUTF-8と認識します。

```hsp
    #include "hsp3utf.as"
```

```hsp
    #include "hsp3_64.as"
```

判定がうまくいかない場合は、設定によりUTF-8サポートの機能を有効・無効にできます。
「実行とデバッグ」タブから `launch.json` を作成し、以下のように `utf8Support` の値を設定してください。

```json
{
    /* 略 */
    "configurations": [
        {
            /* 略 */,
            "utf8Support": "disabled"
        }
    ]
}
```

指定できる値は以下の通りです:

- `enabled`: 入力されるスクリプトはUTF-8エンコーディングとみなされ、生成されるデータはUTF-8エンコーディングになります
- `disabled`: 入力されるスクリプトはshift_jisエンコーディングとみなされ、生成されるデータもshift_jisエンコーディングになります
- その他
    - `auto`: 自動判定 (既定値)
    - `input`: 入力されるスクリプトはUTF-8で、生成されるデータはshift_jisになります
    - `output`: 入力されるスクリプトはshift_jisで、生成されるデータはUTF-8になります

----

## 開発環境

この拡張機能の開発環境を構築する手順は以下の通りです。

- Node.js をインストールしてください。

PowerShell でこのディレクトリを開き、以下のコマンドを実行します。

```pwsh
npm ci --ignore-scripts
```

開発中の拡張機能のインストール・アンインストールは以下のスクリプトを使用します。

```pwsh
# インストール
./install.ps1
```

```pwsh
# アンインストール
./install.ps1
```

## ビルダー

スクリプトのコンパイル・実行のためにビルダーというツールを使っています。

- [dist/builder.hsp](dist/builder.hsp) (スクリプト)
- [dist/builder.md](dist/builder.md) (説明)

## 開発者用のドキュメント

→ [development.md](development.md)
