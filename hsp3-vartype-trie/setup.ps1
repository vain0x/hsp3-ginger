#!/bin/pwsh
# FIXME: 動作しない。シンボリックリンクは DLL としてロードできない？

# 管理者権限を取得する。
function escalate($command) {
    if (!$(test-path '.escalate-lock')) {
        new-item '.escalate-lock'
        start-process pwsh -verb 'runas' -argumentList @('/c', $command)
        exit 0
    }

    remove-item '.escalate-lock'
}

# シンボリックリンクを作る。(ln -sbf $referant $name と同様。管理者権限が必要。)
function linkFile($referant, $name) {
    $backup = $name + ".bak"

    if ($(test-path $name)) {
        if ($(test-path $backup)) {
            remove-item -force $backup
        }

        move-item -path $name -destination $backup
    }

    new-item -itemType symbolicLink -path $name -value $referant
}

# ~/bin にパスを通す。
function ensureHomeBin() {
    $homeBin = [System.IO.Path]::Combine($env:UserProfile, "bin")

    if (!$(test-path $homeBin)) {
        mkdir -force $homeBin
    }

    if (!$env:PATH.contains($homeBin)) {
        $env:PATH = "$homeBin;$env:PATH"
        [System.Environment]::SetEnvironmentVariable('PATH', $env:PATH, [EnvironmentVariableTarget]::User)
    }
}

# デバッグ版の DLL にシンボリックリンクを張る。
$targetBinDll = "$PWD/target/hsp3-vartype-trie/Win32-Debug/bin/hsp3_vartype_trie.dll"
$distDll = "$PWD/dist/hsp3_vartype_trie.dll"

if (!$(test-path $distDll)) {
    escalate './setup.ps1'
    linkFile $targetBinDll $distDll
}

echo 'OK'
