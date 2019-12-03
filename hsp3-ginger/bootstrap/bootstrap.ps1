# bootstrap.hsp を使って ginger (CLI) の実行ファイルを作成する。
# 使い方: bootstrap/bootstrap "path/to/hsp"

$hsp3Root = $args[0]

if (!$hsp3Root) {
    $hsp3Root = $env:HSP3_ROOT
}

# 作者環境用
if (!$hsp3Root) {
    $hsp3Root = $env:KNOWBUG_CLIENT_HSP3_ROOT
}

if (!$hsp3Root) {
    write-error '引数または環境変数 HSP3_ROOT に HSP のインストールディレクトリを指定してください。'
    exit 1
}

$workDir = (get-item .).fullName

# 実行ファイル生成用のスクリプトをコンパイルする。
& "$hsp3Root/hspcmp.exe" "--compath=$hsp3Root/common/" "$workDir/bootstrap/bootstrap.hsp"
if (!$?) {
    write-error 'bootstrap.hsp のコンパイルに失敗しました。'
    exit 1
}

# AXファイルが生成されたことを確認する。
$bootstrapAx = "$workDir/bootstrap/bootstrap.ax"
if (!(test-path $bootstrapAx)) {
    write-error "$bootstrapAx が見つかりません。"
    exit 1
}

# ginger の実行ファイルを生成する。
try {
    cd $hsp3Root

    & "$hsp3Root/hsp3cl.exe" $bootstrapAx "$workDir/src/ginger_main_cli.hsp"
    if (!$?) {
        write-error 'ginger の実行ファイル生成に失敗しました。'
        exit 1
    }

    & "$hsp3Root/ginger.exe" build "$workDir/src/ginger_main_gui.hsp"
    if (!$?) {
        write-error 'ginger_gui の実行ファイル生成に失敗しました。'
        exit 1
    }

    cp "$hsp3Root/ginger.exe" "$workDir/bin/ginger.exe"
} finally {
    cd $workDir
}
