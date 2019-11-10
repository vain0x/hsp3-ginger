# デバッガーのデバッグビルドのシンボリックリンクをインストールします。
# シンボリックリンクの作成に管理者権限が必要なため、管理者権限を要求します。

if (!$env:HSP3_ROOT) {
    write-error '環境変数 HSP3_ROOT を設定してください。'
    exit 1
}

# 権限昇格
if (!$(test-path .lock)) {
    new-item .lock
    start-process pwsh -argumentList @('/c', './scripts/install-dev.ps1; pause') -verb runas
    return
}
remove-item .lock

function install($name, $platform, $config) {
    $root = (get-item $env:HSP3_ROOT).fullName
    $src = (get-item "./target/hsp3-debug-empty/$platform-$config/bin/$name").fullName
    $dest = "$root/$name"

    if ($(test-path $dest)) {
        remove-item $dest
    }

    new-item -itemType symbolicLink -path $dest -value $src
}

./scripts/backup
./scripts/build-all

install 'hsp3debug.dll' 'Win32' 'Debug'
install 'hsp3debug_u8.dll' 'Win32' 'DebugUtf8'
install 'hsp3debug_64.dll' 'x64' 'Debug'
