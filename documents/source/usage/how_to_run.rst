実行方法
========

| UWSCRはコンソールアプリケーションです
| コマンドプロンプトやPowerShell上で実行してください
| Explorer等から実行した場合はコンソールウィンドウが表示されます

コマンドラインオプション
------------------------

.. program:: uwscr

スクリプトの実行
^^^^^^^^^^^^^^^^

.. option:: スクリプトパス [PARAM_STR...]

    スクリプトを実行します

    .. object:: スクリプトパス

        実行するスクリプトファイルのパス

    .. object:: PARAM_STR

        | スクリプトに渡すパラメータ
        | 半角スペース区切りで複数のパラメータを指定可能
        | 渡されたパラメータは ``PARAM_STR`` 特殊変数に格納されています

        .. admonition:: 実行例

            .. code:: shell

                uwscr hoge.uws foo bar baz

            .. code:: uwscr

                print PARAM_STR // [foo, bar, baz]

    .. admonition:: ハイフンから始まる文字を渡す場合
        :class: hint

        | ``-`` や ``--`` から始まる文字列はコマンドラインオプションと見なされます
        | PARAM_STRの文字列としてそれらを渡す場合は ``--`` の後に記述してください

        .. sourcecode:: powershell

            uwscr 123 456 -78    # -78 がオプションだと見なされエラーになる
            uwscr 123 456 "-78"  # ""で括っても同様
            # -- の後に書けばOK
            uwscr -- 123 456 -78 # ["123", "456", "-78"] として渡る


.. option:: -w, --window

    | コンソールから実行された場合にwindowモードでの起動を強制します
    | スクリプトパスが指定されていない場合使えません

.. option:: -a, --ast

    | スクリプトの構文木を出力します
    | スクリプトパスが指定されていない場合使えません

.. option:: --continue

    | 構文木の出力後にスクリプトを実行する場合に指定
    | ``--ast`` が指定されていない場合使えません

.. option::  -p, --prettify

    | 出力される構文木を見やすくします
    | ``--ast`` が指定されていない場合使えません

REPLモード
^^^^^^^^^^

.. option:: モジュールパス [PARAM_STR...]

    | REPL起動前に読み込ませるモジュールファイルのパス
    | PARAM_STRを渡すこともできる

    .. sourcecode:: shell

        PS> uwscr hoge.uws foo bar baz --repl
        uwscr> PARAM_STR
        ["foo", "bar", "baz"]

.. option:: -r, --repl

    | Replを起動します

.. admonition:: Replの使い方
    :class: hint

    | プロンプトに式や文を入力しEnterキーを押すと実行されます
    | 変数への代入などは次の入力にも引き継がれます
    | スクリプトを読み込ませることで事前に定義した関数等も使用できます
    | Tabキーで以下の補完が行なえます、いずれも小文字のみにマッチします

    - ビルトイン関数
    - ビルトイン定数
    - キーワードの一部

    | Alt+Enterで改行します
    | ブロック構文の入力や複数行の一括実行が行なえます


.. hint:: コマンドライン引数がない場合もREPLモードで起動します

UWSCRライブラリ(uwsl)ファイル出力
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
.. option:: スクリプトパス

    | uwslの変換するスクリプトのパス

.. option:: -l, --lib

    | スクリプトのあるディレクトリに ``スクリプト名.uwsl`` ファイルを出力します

コード実行
^^^^^^^^^^

.. option:: -c, --code <CODE>

    | 渡された文字列を評価して実行します

    .. object:: CODE

        | UWSCRで評価可能な式または文を示す文字列
        | 半角スペースを含む場合は ``""`` で括ってください

    .. admonition:: 実行例

        .. code:: shell

            uwscr -c "msgbox('hello world!')"

操作記録
^^^^^^^^

.. option:: --record [<FILE>]

    | 実行した操作の低レベル記録を行います。
    | FILEには記録した操作を保存するパスを指定します
    | FILEを省略した場合はクリップボードに保存します

設定ファイル
^^^^^^^^^^^^

.. option:: -s, --settings [<OPTION>]

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

.. option:: --schema [<DIR>]

    設定ファイル用のjson schemaファイル(``uwscr-settings-schema.json``)を出力します

    .. object:: DIR

        | 出力先ディレクトリのパスを指定
        | 省略した場合はuwscr.exeと同じディレクトリに出力されます

オンラインヘルプ
^^^^^^^^^^^^^^^^

.. option:: -o, --online-help

    オンラインヘルプをブラウザで表示します

.. option:: --license

    サードパーティライセンスをブラウザで表示します

情報表示
^^^^^^^^

.. option:: -h, --help

    ヘルプを表示します

.. option:: -v, --version

    UWSCRのバージョンを表示します

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
