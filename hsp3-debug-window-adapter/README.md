# HSP3 Debug Window Adapter

HSP3 のスクリプトをデバッグモードで起動・終了させるだけの Debug Adapter Protocol (DAP) 実装です。HSP3 標準のデバッグウィンドウを使用してデバッグするのに使えます。

## 設定

shift_jisのスクリプトをデバッグするには、UTF-8機能を無効にする必要があります。
「実行とデバッグ」タブから `launch.json` を作成し、以下のように `utf8Support` を `disabled` にしてください。

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

----
----

## 参考

- [Contribution Points | Visual Studio Code Extension API](https://code.visualstudio.com/api/references/contribution-points)
    - 拡張機能の package.json に書く内容のドキュメント
- [Built-in Commands | Visual Studio Code Extension API](https://code.visualstudio.com/api/references/commands)
    - コマンド機能のドキュメント
- [Debugger Extension](https://code.visualstudio.com/api/extension-guides/debugger-extension)
    - デバッグ機能を提供する拡張機能の作り方のガイド
