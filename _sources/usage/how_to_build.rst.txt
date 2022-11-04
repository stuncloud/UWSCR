ビルド方法
==========

UWSCR通常版
+++++++++++

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

     .. code:: shell

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

ビルド
------

.. important:: Rustのバージョンについて

    | UWSCR0.8.1よりCargo.tomlで ``rust-version`` が指定されています
    | このバージョン未満のRustではビルドができません


1. UWSCRを ``git clone`` し、PowerShellでそのディレクトリへ移動
2. 以下のコマンドを実行

    - x64デバッグビルド

        .. code:: powershell

            cargo build

        .. note:: ``.\target\debug\`` に出力されます

    - x64リリースビルド

        .. code:: powershell

            cargo build --release

        .. note:: ``.\target\release\`` に出力されます


    - x86デバッグビルド

        .. code:: powershell

            cargo build --target=i686-pc-windows-msvc

        .. note:: ``.\target\i686-pc-windows-msvc\debug\`` に出力されます


    - x86リリースビルド

        .. code:: powershell

            cargo build --target=i686-pc-windows-msvc --release

        .. note:: ``.\target\i686-pc-windows-msvc\release\`` に出力されます

cargoによるテスト実行
^^^^^^^^^^^^^^^^^^^^^

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

chkimg版
++++++++

| chkimgを含むUWSCRをビルドする場合は別途opencvのインストールが必要です
| opencvはバージョン4.5.4をインストールしてください

opencvのインストール
--------------------

1. `Release LLVM 13.0.0 · llvm/llvm-project <https://github.com/llvm/llvm-project/releases/tag/llvmorg-13.0.0>`_ で ``LLVM-13.0.0-win**.exe`` をダウンロードしてインストール
2. `Release OpenCV 4.5.4 · opencv/opencv <https://github.com/opencv/opencv/releases/tag/4.5.4>`_ で ``opencv-4.5.4-vc14_vc15.exe`` をダウンロードしてインストール
3. ビルド環境で以下の環境変数を読み取れるようにしてください

    | opencv4.5.4を ``C:\tools`` に展開した場合

    .. object:: OPENCV_LINK_PATHS

        ``C:\tools\opencv\build\x64\vc15\lib``

    .. object:: OPENCV_LINK_LIBS

        ``opencv_world454``

    .. object:: OPENCV_INCLUDE_PATHS

        ``C:\tools\opencv\build\include\``

    .. object:: PATH

        cargoでテスト実行する際にdllを参照するため

        ``%PATH%;C:\tools\opencv\build\x64\vc15\bin``

ビルド
------

``chkimg`` フィーチャーを指定してビルドします

.. code:: powershell

    # x64デバッグ版
    cargo build --features chkimg
    # x64リリース版
    cargo build --features chkimg --release

.. warning:: chkimg版はx86でのビルドはできません

