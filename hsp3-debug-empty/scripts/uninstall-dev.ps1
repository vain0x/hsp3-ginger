# デバッガーをバックアップから復元します。

if (!$env:HSP3_ROOT) {
    write-error '環境変数 HSP3_ROOT を設定してください。'
    exit 1
}

function uninstall($name) {
    $root = $env:HSP3_ROOT
    $src = "$root/.backup/$name"
    $dest = "$root/$name"

    if ($(test-path $dest)) {
        remove-item $dest
    }

    copy-item -path $src -destination $dest
}

uninstall hsp3debug.dll
uninstall hsp3debug_u8.dll
uninstall hsp3debug_64.dll
