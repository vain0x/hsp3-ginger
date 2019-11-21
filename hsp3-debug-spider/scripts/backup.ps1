# デバッガーのバックアップを作成します。
# すでにバックアップがあるときは何もしません。

if (!$env:HSP3_ROOT) {
    write-error '環境変数 HSP3_ROOT を設定してください。'
    exit 1
}

function backup($name) {
    $root = $env:HSP3_ROOT
    $src = "$root/$name"
    $dest = "$root/.backup/$name"

    mkdir -force "$root/.backup"

    if (!$(test-path $src)) {
        echo "File $name missing"
        return
    }

    if ($(test-path $dest)) {
        echo "Backup $name already exists"
        return
    }

    echo "Backup $name"
    copy-item -path $src -destination $dest
}

backup hsp3debug.dll
backup hsp3debug_u8.dll
backup hsp3debug_64.dll
