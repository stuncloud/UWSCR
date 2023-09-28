実行方法
========

| UWSCRはコンソールアプリケーションです
| コマンドプロンプトやPowerShell上で実行してください
| Explorer等から実行した場合はコンソールウィンドウが表示されます

コマンドラインオプション
------------------------

.. program:: uwscr

.. option:: SCRIPT [params...]

    スクリプトを実行します

    .. object:: SCRIPT

        実行するスクリプトファイルのパス

    .. object:: params

        | スクリプトに渡すパラメータ
        | 半角スペース区切りで複数のパラメータを指定可能
        | 渡されたパラメータには ``PARAM_STR`` 特殊変数に格納されています

        .. admonition:: 実行例

            .. code:: shell

                uwscr hoge.uws foo bar baz

            .. code:: uwscr

                print PARAM_STR // [foo, bar, baz]

.. option:: --window SCRIPT [params]
.. option:: -w       SCRIPT [params]

    コンソールから実行された場合にwindowモードでの起動を強制します

.. option:: --repl [SCRIPT]
.. option:: -r     [SCRIPT]

    Replを起動します
    スクリプトを渡すとRepl起動前に読み込まれます

.. option:: --ast SCRIPT
.. option:: -a    SCRIPT

    スクリプトの構文木を出力します

.. option:: --ast-force SCRIPT

    構文解析エラーが発生した場合でも解析が完了した部分の構文木を出力します

.. option:: --lib SCRIPT
.. option:: -l    SCRIPT

    スクリプトからUWSCRライブラリファイル(.uwsl)を生成します

.. option:: --code CODE
.. option:: -c     CODE

    渡された文字列を評価して実行します

    .. object:: CODE

        UWSCRで評価可能な式または文

    .. admonition:: 実行例

        .. code:: shell

            uwscr -c "msgbox('hello world!')"

.. option:: --settings [OPTION]
.. option:: -s         [OPTION]

    | 設定ファイル(``settings.json``)を開きます
    | 設定ファイルは ``%APPDATA%\UWSCR\settings.json`` に出力されます

    .. object:: OPTION

        | 設定ファイルがすでに存在する場合にどのように開くかのオプションを指定します
        | 設定ファイルが存在しない場合これらのオプションは無視され、設定ファイルが新規に作成されます

        .. object:: 省略時

            設定ファイルが存在していればそれを開きます

        .. object:: init

            設定ファイルが存在する場合はそれを破棄し、新たな設定ファイルを出力します

        .. object:: merge

            古いバージョンの設定ファイルの内容を可能な限りマージした新しいバージョンの設定ファイルを出力します

.. option:: --schema [DIR]

    設定ファイル用のjson schemaファイル(``uwscr-settings-schema.json``)を出力します

    .. object:: DIR

        | 出力先ディレクトリのパスを指定
        | 省略した場合はuwscr.exeと同じディレクトリに出力されます

.. option:: --help
.. option:: -h
.. option:: -?
.. option:: /?

    ヘルプを表示します

.. option:: --version
.. option:: -v

    UWSCRのバージョンを表示します

.. option:: --online-help
.. option:: -o

    オンラインヘルプをブラウザで表示します

スクリプトファイルのエンコーディング
------------------------------------

以下に対応しています

- UTF-8
- UTF-16 (BE, LE)
- Shift-JIS

注意
----

ANSIコードポイントについて
^^^^^^^^^^^^^^^^^^^^^^^^^^

| UWSCRではOSのANSIコードポイントが932であることを想定しています
| 65001(UTF8)等に変更している場合の動作保証はありません
