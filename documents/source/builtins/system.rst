システム関数
============

システム情報
------------

.. function:: kindofos(データ種別=FALSE)

    OS種別、またはアーキテクチャを判定します

    :param 真偽値または定数 省略可 データ種別: 以下のいずれかを指定

        .. object:: TRUE

            OSが64ビットかどうかを真偽値で返す

        .. object:: FALSE

            OS種別をOS定数で返す

            .. object:: OS_WIN2000 (12)
            .. object:: OS_WINXP (13)
            .. object:: OS_WINSRV2003 (14)
            .. object:: OS_WINSRV2003R2 (15)
            .. object:: OS_WINVISTA (20)
            .. object:: OS_WINSRV2008 (21)
            .. object:: OS_WIN7 (22)
            .. object:: OS_WINSRV2008R2 (27)
            .. object:: OS_WIN8 (23)
            .. object:: OS_WINSRV2012 (24)
            .. object:: OS_WIN81 (25)
            .. object:: OS_WINSRV2012R2 (26)
            .. object:: OS_WIN10 (30)
            .. object:: OS_WINSRV2016 (31)
            .. object:: OS_WIN11 (32)

        .. object:: OSVER_MAJOR

            OSのメジャーバージョンを数値で返す

        .. object:: OSVER_MINOR

            OSのマイナーバージョンを数値で返す

        .. object:: OSVER_BUILD

            OSのビルド番号を数値で返す

        .. object:: OSVER_PLATFORM

            OSのプラットフォームIDを数値で返す

    :return: データ種別による


.. function:: env(環境変数)

    環境変数を展開します

    :param 文字列 環境変数: 環境変数を示す文字列
    :return:  展開された環境変数(``文字列``)

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            print env('programfiles') // C:\Program Files


.. function:: wmi(WQL, 名前空間="root/cimv2")

    | WQLを発行しWMIから情報を得ます

    :param 文字列 WQL: WMIに対するクエリ文
    :param 文字列 省略可 名前空間: 名前空間のパス
    :return: クエリ結果(``UObject配列``)

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            res = wmi('select name, processid, commandline from Win32_Process where name = "uwscr.exe"')
            for obj in res
                print obj.name
                print obj.processid
                print obj.commandline
            next

.. function:: cpuuserate()

    | システム全体での1秒間のCPU使用率を得る

    :rtype: 数値
    :return: CPU使用率

プロセス実行
------------

.. function:: exec(ファイル名, 同期フラグ=FALSE, x=EMPTY, y=EMPTY, width=EMPTY, height=EMPTY)

    | プロセスを起動します
    | IDの取得に成功した場合はID0を更新します

    :param 文字列 ファイル名: 実行するexeのパス
    :param 真偽値 省略可 同期フラグ:

        - TRUE: プロセス終了までブロックする
        - FALSE: プロセス終了を待たずに続行する

    :param 数値 省略可 x: ウィンドウ表示位置(X座標)、省略時はウィンドウのデフォルト
    :param 数値 省略可 y: ウィンドウ表示位置(Y座標)、省略時はウィンドウのデフォルト
    :param 数値 省略可 width: ウィンドウの幅、省略時はウィンドウのデフォルト
    :param 数値 省略可 height: ウィンドウの高さ、省略時はウィンドウのデフォルト
    :return:

        - 同期フラグTRUE: プロセスの終了コード(``数値``)
        - 同期フラグFALSE: ``ウィンドウID`` (取得できなければ-1)
        - 失敗時: -1

.. function:: shexec(ファイル, パラメータ=EMPTY)

    | 対象ファイルに対してシェルにより指定された動作で実行させます
    | (「ファイル名を指定して実行」とほぼ同じ)

    :param 文字列 ファイル: 実行するファイルのパス
    :param 文字列 省略可 パラメータ: 実行時に付与するパラメータ
    :戻り値: ``真偽値`` 正常に実行されれば ``TRUE``

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            shexec("cmd", "/k ipconfig")

CUIシェル
---------

.. function:: doscmd(コマンド, 非同期=FALSE, 画面表示=FALSE, Unicode=FALSE)

    | コマンドプロンプトを実行します

    :param 文字列 コマンド: 実行するコマンド
    :param 真偽値 省略可 非同期: FALSEなら終了するまで待つ
    :param 真偽値 省略可 画面表示: TRUEならコマンドプロンプトを表示する
    :param 真偽値 省略可 Unicode: TRUEならUnicode出力
    :return: *非同期* と *画面表示* がいずれもFALSEであれば標準出力または標準エラー(``文字列``)を返す、それ以外は ``EMPTY``

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            // Unicode出力で文字化けを解消する
            cmd = "echo 森鷗外𠮟る 🐶"
            print doscmd(cmd, FALSE, FALSE, FALSE) // 森?外??る ??
            print doscmd(cmd, FALSE, FALSE, TRUE)  // 森鷗外𠮟る 🐶

.. function:: powershell(コマンド, 非同期=FALSE, 画面表示=FALSE, プロファイル無視=FALSE)

    | Windows PowerShell (バージョン6未満)を実行します

.. function:: pwsh(コマンド, 非同期=FALSE, 画面表示=FALSE, プロファイル無視=FALSE)

    | PowerShell (バージョン6以降)を実行します

    :param 文字列 必須 コマンド: 実行するコマンド
    :param 真偽値 省略可 非同期: FALSEなら終了するまで待つ
    :param 真偽値または2 省略可 画面表示: TRUEならPowerShellを表示する、2なら表示して最小化
    :param 真偽値 省略可 プロファイル無視: TRUEなら$PROFILEを読み込まない
    :return: 非同期と画面表示がいずれもFALSEであれば標準出力(``文字列``)を返す、それ以外は ``EMPTY``

