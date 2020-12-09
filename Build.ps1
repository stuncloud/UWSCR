[CmdletBinding()]
param(
    [Parameter(Mandatory=$false)]
    [string] $Version,
    [switch] $Release,
    [string] $OutDir = '.\.release'
)

# リリースビルドの場合vcのライブラリをスタティックリンクする
if ($Release) {
    $env:RUSTFLAGS='-C target-feature=+crt-static'
} else {
    $env:RUSTFLAGS=''
}
# build x64 exe
$cmd = 'cargo build {0}' -f $(if ($Release) {'--release'})
Invoke-Expression -Command $cmd
# build x86 exe
$cmd = 'cargo build --target=i686-pc-windows-msvc {0}' -f $(if ($Release) {'--release'})
Invoke-Expression -Command $cmd

if ($Release) {
    $env:RUSTFLAGS=''

    $exe64 = '.\target\release\uwscr.exe'
    $exe86 = '.\target\i686-pc-windows-msvc\release\uwscr.exe'
    $exe64, $exe86 | % {
        if (! (Test-Path $_)) {
            Write-Error "$($_) が見つかりません"
            break
        }
    }
    if (! $Version) {
        if (('{0} --version' -f $exe64 | Invoke-Expression) -match '\d+\.\d+\.\d+') {
            $Version = $Matches[0]
        } else {
            Write-Error "uwscrのバージョンが不明"
            break
        }
    }
    if (! (Test-Path($OutDir))) {
        mkdir $OutDir | Out-Null
    }
    $verpath = Join-Path -Path $OutDir -ChildPath $Version
    $x64path = Join-Path -Path $verpath -ChildPath 'x64'
    $x86path = Join-Path -Path $verpath -ChildPath 'x86'
    if (! (Test-Path $verpath)) {
        mkdir $verpath | ForEach-Object {
            mkdir $x64path | Out-Null
            mkdir $x86path | Out-Null
        }
    }
    $exe64 | Copy-Item -Destination $x64path
    $exe86 | Copy-Item -Destination $x86path
    $outzip = Join-Path -Path $verpath -ChildPath uwscr.zip
    Get-ChildItem $verpath -Directory | Compress-Archive -DestinationPath $outzip -PassThru -Force
}
