# bootstrap.hsp を使って ginger (CLI) の実行ファイルを作成する。
# 使い方: bootstrap/bootstrap "path/to/hsp"

$ErrorActionPreference = 'Stop'

$hsp3Home = $args[0]

if (!$hsp3Home) {
    $hsp3Home = $env:HSP3_HOME
}

if (!$hsp3Home) {
    write-error '引数または環境変数 HSP3_HOME に HSP のインストールディレクトリを指定してください。'
    exit 1
}

$baseDir = (get-item .).fullName

try {
    chdir 'bootstrap'

    # 実行ファイル生成用のスクリプトをコンパイルする。
    & "$hsp3Home/hspcmp.exe" "--compath=$hsp3Home/common/" "$baseDir/bootstrap/bootstrap.hsp"
    if (!$?) {
        write-error 'bootstrap.hsp のコンパイルに失敗しました。'
        exit 1
    }

    # AXファイルが生成されたことを確認する。
    $bootstrapAx = './bootstrap.ax'
    if (!(test-path $bootstrapAx)) {
        write-error "$bootstrapAx が見つかりません。"
        exit 1
    }

    # ginger の実行ファイルを生成する。
    & "$hsp3Home/hsp3cl.exe" $bootstrapAx
    if (!$?) {
        write-error 'ginger の実行ファイル生成に失敗しました。'
        exit 1
    }

    # & "$baseDir/src/ginger.exe" build --hsp $hsp3Home "$baseDir/src/ginger_main_gui.hsp"
    # if (!$?) {
    #     write-error 'ginger_gui の実行ファイル生成に失敗しました。'
    #     exit 1
    # }

    move-item -force "$baseDir/src/ginger.exe" "$baseDir/bin/ginger.exe"

    echo 'OK'
} finally {
    cd $baseDir
}
