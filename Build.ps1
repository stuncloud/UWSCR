[CmdletBinding()]
param(
    [Parameter(Mandatory=$false)]
    [string] $Version,
    [switch] $Release
)

# build x64 exe
$cmd = 'cargo build {0}' -f $(if ($Release) {'--release'})
Invoke-Expression -Command $cmd
# build x86 exe
$cmd = 'cargo build --target=i686-pc-windows-msvc {0}' -f $(if ($Release) {'--release'})
Invoke-Expression -Command $cmd

if ($Release) {
    $exe64 = '.\target\release\uwscr.exe'
    $exe86 = '.\target\i686-pc-windows-msvc\release\uwscr.exe'
    if (! (Test-Path $exe64)) {
        Write-Error 'x64版uwscr.exeがない'
        break
    }
    if (! (Test-Path $exe86)) {
        Write-Error 'x86版uwscr.exeがない'
        break
    }
    $outdir = '.\target\github-release\'
    if (! $Version) {
        if (('{0} --version' -f $exe64 | Invoke-Expression) -match '\d+\.\d+\.\d+') {
            $Version = $Matches[0]
        } else {
            Write-Error "uwscrのバージョンが不明"
            break
        }
    }
    if (! (Test-Path($outdir))) {
        mkdir $outdir | Out-Null
    }
    $verpath = Join-Path -Path $outdir -ChildPath $Version
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
    Get-ChildItem $verpath | Compress-Archive -DestinationPath $outzip -PassThru -Force
}
