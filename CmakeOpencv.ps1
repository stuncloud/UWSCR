#requires -Version 7.0

[CmdletBinding()]
param(
    # ソースファイルのあるディレクトリ
    [Parameter(Mandatory)]
    [Alias("S")]
    [string] $Source,
    # 出力先
    [Parameter(Mandatory)]
    [Alias("O")]
    [string] $OutDir,
    # ジェネレータを指定します
    [Parameter(Mandatory=$false)]
    [Alias("G")]
    [string] $Generator="Visual Studio 16 2019",
    # ジェネレータのアーキテクチャを指定します
    [Parameter(Mandatory=$false)]
    [Alias("Arc")]
    [ValidateSet("x64", "Win32")]
    [string] $Architecture="x64",
    # BUILD_SHARED_LIBSを有効にします
    [Parameter(Mandatory=$false)]
    [switch] $WithStaticCrt
)

if (Get-Command cmake) {
    $crt = $WithStaticCrt ? 'BUILD_WITH_STATIC_CRT=ON' : 'BUILD_WITH_STATIC_CRT=OFF'
    cmake -S $Source -B $OutDir -G $Generator -A $Architecture `
    -D CMAKE_BUILD_TYPE=Release `
    -D BUILD_SHARED_LIBS=OFF `
    -D $crt `
    -D BUILD_opencv_apps=OFF `
    -D BUILD_opencv_calib3d=OFF `
    -D BUILD_opencv_dnn=OFF `
    -D BUILD_opencv_features2d=OFF `
    -D BUILD_opencv_flann=OFF `
    -D BUILD_opencv_gapi=OFF `
    -D BUILD_opencv_highgui=OFF `
    -D BUILD_opencv_java_bindings_generator=OFF `
    -D BUILD_opencv_js_bindings_generator=OFF `
    -D BUILD_opencv_ml=OFF `
    -D BUILD_opencv_objc_bindings_generator=OFF `
    -D BUILD_opencv_objdetect=OFF `
    -D BUILD_opencv_photo=OFF `
    -D BUILD_opencv_python_bindings_generator=OFF `
    -D BUILD_opencv_python_tests=OFF `
    -D BUILD_opencv_stitching=OFF `
    -D BUILD_opencv_ts=OFF `
    -D BUILD_opencv_video=OFF `
    -D BUILD_opencv_videoio=OFF `
    -D BUILD_JAVA=OFF `
    -D BUILD_PERF_TESTS=OFF `
    -D BUILD_TESTS=OFF `
    -D WITH_ADE=OFF `
    -D WITH_OPENEXR=OFF `
    -D WITH_QUIRC=OFF
}

<#
.SYNOPSIS
opencvのソースをcmakeします
.DESCRIPTION
cmakeコマンドを実行し、Visual Studioビルドツールでビルドを行うためのファイルを生成します
cmakeコマンドにPATHが通っている必要があります
.EXAMPLE
.\CmakeOpencv.ps1 -S C:\tools\opencv-4.6.0 -O C:\tools\opencv64 -Arc x64 -WithStaticCrt
.EXAMPLE
.\CmakeOpencv.ps1 -S C:\tools\opencv-4.6.0 -O C:\tools\opencv86 -Arc Win32
#>