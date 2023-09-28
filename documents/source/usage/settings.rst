設定ファイル
============

ファイルの所在
--------------

.. code:: powershell

    uwscr --settings

を実行することで ``%APPDATA%\UWSCR\settings.json`` に出力されます

設定ファイルに関するコマンド
----------------------------

.. code:: powershell

    # 現在の設定ファイルを標準のエディタで開く
    uwscr --settings

    # 設定ファイルを初期化する
    uwscr --settings init

    # 旧バージョンの設定を保持しつつ、新しい設定項目を追加する
    uwscr --settings merge

    # いずれの場合も設定ファイルが存在しない場合は新規に作成します

設定の優先順位
--------------

スクリプト実行時の設定の優先順位が以下の通りです (上にあるほど優先される)

1. OPTION設定
2. 設定ファイル(settings.json)
3. デフォルト設定

設定ファイルの内容
------------------

.. hint:: 設定ファイルのデータ型について

    .. object:: bool

        ``true`` または ``false`` を指定

    .. object:: string (文字列)

        文字列、設定が不要な場合は ``null`` を指定する

    .. object:: number (数値)

        数値

.. code:: json

    {
        "options": {
            // bool  : fially部を必ず実行するかどうか
            "opt_finally": false,
            // bool  : 変数初期化にdim宣言を必須とするかどうか
            "explicit": false,
            // string: (未対応) ダイアログのタイトル
            "dlg_title": null,
            // number: ログファイルの出力方法
            "log_file": 1,
            // number: (未対応) ログファイルの行数
            "log_lines": 400,
            // string: ログ出力フォルダ
            "log_path": null,
            // (未対応) メインGUIの座標
            "position": {
                // number: ウィンドウ左上のx座標
                "left": 0,
                // number: ウィンドウ左上のy座標
                "top": 0
            },
            // ダイアログなどのフォント
            "default_font": {
            // string: フォント名
            "name": "Yu Gothic UI",
            // number: フォントサイズ
            "size": 15
            },
            // bool  : (未対応) 仮想デスクトップにも吹き出しを表示するかどうか
            "fix_balloon": false,
            // bool  : (未対応) 停止ホットキーを無効にするかどうか
            "no_stop_hot_key": false,
            // bool  : (未対応) 短絡評価を行うかどうか
            "short_circuit": true,
            // bool  : publicの重複定義を禁止するかどうか
            "opt_public": false,
            // bool  : 文字列比較などで大文字小文字を区別するかどうか
            "same_str": false,
            // bool  : print文をGUIに出力するかどうか
             "gui_print": false
        },
        // BrowserControl設定
        "browser": {
            // string: 操作するGoogle Chromeのパス (nullなら自動取得)
            "chrome": null,
            // string: Microsoft Edgeのパス (nullなら自動取得)
            "msedge": null
        },
        // chkimg設定
        "chkimg": {
            // bool  : chkimg実行時のスクリーン画像を保存する(chkimg_ss.png)
            "save_ss": false
        },
        // print窓のフォント
        "logfont": {
            // string: フォント名
            "name": "MS Gothic",
            // number: フォントサイズ
            "size": 15
        },
        // json schemaのurl: x.x.xはリリースバージョン
        "$schema": "https://github.com/stuncloud/UWSCR/releases/download/x.x.x/uwscr-settings-schema.json"
    }

設定ファイルを読み取れない場合
------------------------------

| 書式が不正な場合は設定ファイルの内容は読み取られません
| その場合はデフォルト設定が適用されます
| また、エラー(読み取れなかった理由)がコンソールに出力されます

json schemaについて
-------------------

| 設定ファイルの ``$schema`` は設定ファイルに対応したjson schemaのURLです
| Visual Studio Code等でjsonファイルを編集する際に補完機能が使えるようになります

json schemaのオフライン利用
^^^^^^^^^^^^^^^^^^^^^^^^^^^

| schemaファイルをローカルに出力することでオフライン環境でもjson schemaが利用できます
| 以下のコマンドを実行すると指定パスに ``uwscr-settings-schema.json`` が出力されます

.. code:: powershell

    uwscr --schema [パス]

| このファイルのパスをurlに変換し設定ファイルの ``$schema`` に指定します

.. hint:: ファイルパス→URL変換方法

    | ``C:\\uwscr\\uwscr-settings-schema.json`` であれば ``file:///C:/uwscr/uwscr-settings-schema.json`` のように変換する必要があります
    | PowerShellで簡単に変換できます

    .. code:: powershell

        PS> ([uri] 'C:\\uwscr\\uwscr-settings-schema.json').AbsoluteUri

        file:///C://uwscr//uwscr-settings-schema.json