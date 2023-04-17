# bootstrap.hsp を使って ginger (CLI) の実行ファイルを作成する。
# 使い方: bootstrap/bootstrap "path/to/hsp"

$hsp3Home = $args[0]

if (!$hsp3Home) {
    $hsp3Home = $env:HSP3_HOME
}

if (!$hsp3Home) {
    write-error '引数または環境変数 HSP3_HOME に HSP のインストールディレクトリを指定してください。'
    exit 1
}

$workDir = (get-item .).fullName

# 実行ファイル生成用のスクリプトをコンパイルする。
& "$hsp3Home/hspcmp.exe" "--compath=$hsp3Home/common/" "$workDir/bootstrap/bootstrap.hsp"
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
    cd $hsp3Home

    & "$hsp3Home/hsp3cl.exe" $bootstrapAx "$workDir/src/ginger_main_cli.hsp"
    if (!$?) {
        write-error 'ginger の実行ファイル生成に失敗しました。'
        exit 1
    }

    & "$hsp3Home/ginger.exe" build "$workDir/src/ginger_main_gui.hsp"
    if (!$?) {
        write-error 'ginger_gui の実行ファイル生成に失敗しました。'
        exit 1
    }

    cp "$hsp3Home/ginger.exe" "$workDir/bin/ginger.exe"
} finally {
    cd $workDir
}
