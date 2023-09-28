ビルド方法
==========

UWSCR
+++++

ソースからuwscr.exeをビルドする方法

Rust開発環境の準備
------------------

Windows 10 x64環境での手順

Visual C++ Build Tools のインストール
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

1. `Visual Studio 2019のツール <https://visualstudio.microsoft.com/ja/downloads/#vstool-2019-ja-family>`_ からBuild Tools for Visual Studio 2019のインストーラをダウンロード
2. インストーラからVisual C++ Build Toolsのインストールを行う

Rustのインストール
^^^^^^^^^^^^^^^^^^

1. `Rust をインストール - Rustプログラミング言語 <https://www.rust-lang.org/ja/tools/install>`_ から ``rustup-init.exe`` をダウンロード
2. PowerShellなどから ``rustup-init.exe`` を実行
3. プロンプトに従いインストールを完了する
4. ``rustup --version`` や ``cargo --version`` が正常に実行できればOK

    .. hint:: 実行できない場合は一旦PowerShellなどを再起動してみてください

5. ``rustup target install i686-pc-windows-msvc`` を実行してx86版もビルドできるようにする
6. ``rustup show`` を実行し以下のような出力になっていればOK

     .. code:: text

         Default host: x86_64-pc-windows-msvc
         rustup home:  C:\Users\(your name)\.rustup

         installed toolchains
         --------------------

         stable-i686-pc-windows-msvc
         stable-x86_64-pc-windows-msvc (default)

         installed targets for active toolchain
         --------------------------------------

         i686-pc-windows-msvc
         x86_64-pc-windows-msvc

         active toolchain
         ----------------

         stable-x86_64-pc-windows-msvc (default)
         rustc 1.62.0 (a8314ef7d 2022-06-27)


OpenCV
^^^^^^

| chkimgを含める場合は事前にOpenCVをインストール、または静的リンクライブラリをビルドしておく必要があります
| インストールした場合はuwscr.exe実行時にopencv_worldXXX.dllを参照できるようにする必要があります
| ライブラリをビルドした場合はdllは不要です
| chkimgを含めずビルドする場合はこの項目をスキップしてください

.. _install_opencv:

OpenCVのインストール
********************

1. `OpenCV <https://github.com/opencv/opencv/releases/latest>`_ で ``opencv-X.Y.Z-vc14_vc15.exe`` をダウンロード
2. ``opencv-X.Y.Z-vc14_vc15.exe`` を実行し、任意のフォルダに展開

.. _build_opencv:

OpenCVのビルド
**************

準備
~~~~

1. `OpenCV <https://github.com/opencv/opencv/releases/latest>`_ のソースコードをダウンロードして任意のフォルダに展開
2. `Cmake <https://cmake.org/download/>`_ をダウンロードしインストール

cmake
~~~~~

.. hint::

    | 以下ではOpenCVの展開先を ``C:\tools\opencv`` としています
    | 生成されるファイルの出力先を ``C:\tools\opencv64`` または ``C:\tools\opencv86`` としています
    | msvcビルドツールは ``Visual Studio 16 2019`` としています
    | いずれも環境に合わせて読み替えてください

1. スタートメニューからcmake-guiを起動
2. Where is the source code (ソース) に ``C:\tools\opencv`` を指定
3. Where to build the binaries (出力先) に ``C:\tools\opencv64`` を指定 (x86はopencv86)
4. ``Configure`` ボタンを押す (出力先フォルダが存在しない場合はダイアログで確認されるので作成してもらう)
5. ダイアログが表示されたら
    - generatorは ``Visual Studio 16 2019`` を選択
    - platformは ``x64`` を選択
        - x86なら ``Win32`` にする
    - toolsetは空欄
    - ``Finish`` ボタンを押してしばらく待つ
6. リストが表示されるので変更を加える
    - ``BUILD_SHARED_LIBS`` のチェックを外す
    - ``BUILD_opencv_*`` 系は以下のみチェックし、ほかは外す
        - ``BUILD_opencv_core``
        - ``BUILD_opencv_imgcodecs``
        - ``BUILD_opencv_imgprocs``
    - ``*_TESTS`` 系のチェック外す
    - ``BUILD_JAVA`` のチェック外す
    - ``WITH_ADE`` のチェック外す
    - ``WITH_QUIRC`` のチェック外す
    - ``WITH_OPENEXR`` のチェック外す
    - VC++ランタイムを静的リンクしない場合のみ
        - ``BUILD_WITH_STATIC_CRT`` のチェックを外す
7. 再度 ``Configure`` ボタンを押ししばらく待つ
    - ``BUILD_FAT_JAVA_LIB`` が赤くなるけど無視
8. リストが赤くなっていればなくなるまで ``Configure`` ボタンを押す
9. ``Generate`` ボタンを押す

.. tip:: スクリプトによる実行方法

    | UWSCRリポジトリにある ``CmakeOpencv.ps1`` で上記と同等のことができます

    .. code-block:: powershell

       .\CmakeOpencv.ps1 -Source C:\tools\opencv\ -OutDir C:\tools\opencv64\ -Architecture x64 -WithStaticCrt


msbuild
~~~~~~~

.. hint::

    | msvcビルドツールは ``Visual Studio 16 2019`` がインストールされているものとします
    | また、cmakeの出力先が ``C:\tools\opencv64`` または ``C:\tools\opencv86`` であるものとします
    | 環境に合わせて適宜読み替えてください


1. スタートメニューから ``x64 Native Tools Command Prompt for VS 2019`` を起動
2. 以下を実行

   - x64

       .. code:: bat

           cd /d c:\tools\opencv64
           chcp 65001
           msbuild -p:Configuration=Release;Platform=x64;CodePage=65001 INSTALL.vcxproj

   - x86

       .. code:: bat

           cd /d c:\tools\opencv86
           chcp 65001
           msbuild -p:Configuration=Release;Platform=Win32;CodePage=65001 INSTALL.vcxproj

3. ``C:\tools\opencv64\install`` (x86なら ``C:\tools\opencv86\install``) に出力される

ビルド
------

.. important:: Rustのバージョンについて

    | UWSCR0.8.1よりCargo.tomlで ``rust-version`` が指定されています
    | このバージョン未満のRustではビルドができません

.. important:: VC++ランタイムライブラリについて

    | 以下のコマンドでそのままビルドした場合は実行時にVC++ランタイムライブラリが必要になります
    | exe単体で動作させる(ライブラリを静的リンクする)ためには事前に以下の環境変数をセットしてください

     .. code-block:: powershell

         $env:RUSTFLAGS='-C target-feature=+crt-static'


1. UWSCRを ``git clone`` し、PowerShellでそのディレクトリへ移動
2. 以下のコマンドを実行

デバッグビルド
^^^^^^^^^^^^^^

.. code:: powershell

    # x64
    cargo build

.. note:: ``.\target\debug\`` に出力されます

.. code:: powershell

    # x86
    cargo build --target=i686-pc-windows-msvc

.. note:: ``.\target\i686-pc-windows-msvc\debug\`` に出力されます

リリースビルド
^^^^^^^^^^^^^^

.. code:: powershell

    # x64
    cargo build --release

.. note:: ``.\target\release\`` に出力されます

.. code:: powershell

    # x86
    cargo build --target=i686-pc-windows-msvc --release

.. note:: ``.\target\i686-pc-windows-msvc\release\`` に出力されます

chkimgを含める場合
^^^^^^^^^^^^^^^^^^

準備
****

1. `LLVM <https://github.com/llvm/llvm-project/releases/latest>`_ で ``LLVM-X.Y.Z-win64.exe`` をダウンロードしてインストール

| 以下の環境変数を設定する必要があります
| 具体的な設定値については後述します

.. object:: OPENCV_INCLUDE_PATHS

        includeフォルダのパス

.. object:: OPENCV_LINK_PATHS

        libファイルのあるパス

.. object:: OPENCV_LINK_LIBS

        読み込むlibファイル

ビルド
******

``cargo`` 実行時に ``--features chkimg`` を指定しchkimgが含まれるようにします

OpenCVをインストールした場合
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. hint::

   | :ref:`install_opencv` を実行している必要があります
   | opencvの展開先は ``C:\tools\opencv\`` としています、環境に合わせて適宜読み替えてください

.. caution::

    | この方法ではx86版はビルドできません

以下は環境変数を設定しつつcargoによるビルドを行うPowerShellスクリプトのサンプルです

.. code:: powershell

    # includeフォルダ
    $env:OPENCV_INCLUDE_PATHS = 'C:\tools\opencv\build\include\'
    # libファイルのあるフォルダ
    $env:OPENCV_LINK_PATHS = 'C:\tools\opencv\build\x64\vc15\lib'
    # 読み込むlibファイル
    $env:OPENCV_LINK_LIBS = 'opencv_worldXXX'
    # XXXの部分はopencvのバージョンにより変わります (バージョン4.6.0→460)

    cargo build --features chkimg

.. important::

    | この方法でビルドしたuwscr.exeは ``opencv_worldXXX.dll`` が参照できないと実行できません
    | 以下のいずれかの方法でdllを参照できるようにしてください

    - ``C:\tools\opencv\build\x64\vc15\bin`` にPATHを通す
    - ``C:\tools\opencv\build\x64\vc15\bin\opencv_worldXXX.dll`` をuwscr.exeと同じフォルダにコピーする

OpenCVをビルドした場合
~~~~~~~~~~~~~~~~~~~~~~

.. hint::

   | :ref:`build_opencv` を実行している必要があります
   | msbuildの出力先は ``C:\tools\opencv64\install\`` (``C:\tools\opencv86\install\``) としています、環境に合わせて適宜読み替えてください

.. important:: BUILD_WITH_STATIC_CRTについて

   | VC++ランタイムライブラリを静的リンクしてビルドする場合はopencvビルド時に ``BUILD_WITH_STATIC_CRT`` をオンにします
   | VC++ランタイムライブラリを静的リンクしない場合はopencvビルド時に ``BUILD_WITH_STATIC_CRT`` をオフにします

以下は環境変数を設定しつつcargoによるビルドを行うPowerShellスクリプトのサンプルです

- x64

   .. code:: powershell

       # includeフォルダ
       $env:OPENCV_INCLUDE_PATHS = 'C:\tools\opencv64\install\include'
       # libファイルのあるフォルダ
       $env:OPENCV_LINK_PATHS = 'C:\tools\opencv64\install\x64\vc16\staticlib'
       # 読み込むlibファイル
       # 複数ある場合は , で連結する
       $env:OPENCV_LINK_LIBS = @(
           'opencv_coreXXX'
           'opencv_imgcodecsXXX'
           'opencv_imgprocXXX'
           'ippiw'
           'ittnotify'
           'ippicvmt'
           'liblibjpeg-turbo'
           'liblibopenjp2'
           'liblibpng'
           'liblibtiff'
           'liblibwebp'
           'zlib'
       ) -join ','
       # XXXの部分はopencvのバージョンにより変わります (バージョン4.6.0→460)
       # libから始まるファイルは先頭にlibを追加する必要があります (libpng→liblibpng)

       cargo build --features chkimg

- x86

   .. code:: powershell

       $env:OPENCV_INCLUDE_PATHS = 'C:\tools\opencv86\install\include'
       $env:OPENCV_LINK_PATHS = 'C:\tools\opencv86\install\x86\vc16\staticlib'
       $env:OPENCV_LINK_LIBS = @(
           'opencv_coreXXX'
           'opencv_imgcodecsXXX'
           'opencv_imgprocXXX'
           'ippiw'
           'ittnotify'
           'ippicvmt'
           'liblibjpeg-turbo'
           'liblibopenjp2'
           'liblibpng'
           'liblibtiff'
           'liblibwebp'
           'zlib'
       ) -join ','

       cargo build --features chkimg --target=i686-pc-windows-msvc

GUIアプリケーションとしてビルド
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

| ``gui`` featureフラグを加えてビルドすることでUWSCRはGUIアプリケーションとして振る舞います

.. code-block:: powershell

    cargo build --features gui
    cargo build --features gui --release
    # chkimgも加える
    cargo build --features gui,chkimg

.. admonition:: このfeatureによるビルドは動作保証外です
    :class: caution

    | ``gui`` featureは十分なテストが行われていません
    | このfeatureによるビルドを行った場合の動作については保証されません

cargoによるテスト実行
---------------------

| cargoを使ったuwscrのテスト実行方法
| 都度ビルド→実行を行います

.. code:: powershell

    # スクリプトの実行
    cargo run -- C:\uwscr\test.uws
    # x86
    cargo run --target=i686-pc-windows-msvc -- C:\uwscr\test.uws
    # リリース版で実行
    cargo run --release -- C:\uwscr\test.uws
    # repl
    cargo run
    cargo run -- --repl
    # 設定ファイルを開く
    cargo run -- --settings merge
    # schemaファイルを出力
    cargo run -- --schema .\schema

ドキュメント
++++++++++++

.. important:: Python実行環境が必要です

準備
----

``pip`` 等で以下をインストール

- ``Sphinx`` (ドキュメントのビルド)
- ``furo`` (ドキュメントのテーマ)
- ``pygments`` (サンプル構文のシンタックスハイライト)
- ``sphinx-favicon`` (faviconの設定)

ビルド
------

1. ``.\documents\make.bat html`` を実行

.. hint:: ``.\documents\build\html\`` に出力されます
