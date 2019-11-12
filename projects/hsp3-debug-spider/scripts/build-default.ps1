# 既定の構成でデバッガーをビルドします。

if (!$(which MSBuild.exe)) {
    write-error 'MSBuild.exe にパスを通してください。'
    exit 1
}

MSBuild.exe './hsp3-debug-spider.sln' '-t:Build' '-p:Configuration=DebugUtf8;Platform=x86'
