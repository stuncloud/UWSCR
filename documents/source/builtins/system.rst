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

.. function:: sensor(種別)

    | 各種センサーから情報を得る (Sensor APIを使用)

    :param 定数 種別: センサー種別を指定する定数

        .. object:: SNSR_Biometric_HumanPresense

            | 人が存在した場合に True

        .. object:: SNSR_Biometric_HumanProximity

            | 人との距離(メートル)

        .. object:: SNSR_Electrical_Capacitance

            | 静電容量(ファラド)

        .. object:: SNSR_Electrical_Resistance

            | 電気抵抗(オーム)

        .. object:: SNSR_Electrical_Inductance

            | 誘導係数(ヘンリー)

        .. object:: SNSR_Electrical_Current

            | 電流(アンペア)

        .. object:: SNSR_Electrical_Voltage

            | 電圧(ボルト)

        .. object:: SNSR_Electrical_Power

            | 電力(ワット)

        .. object:: SNSR_Environmental_Temperature

            | 気温(セ氏)

        .. object:: SNSR_Environmental_Pressure

            | 気圧(バール)

        .. object:: SNSR_Environmental_Humidity

            | 湿度(パーセンテージ)

        .. object:: SNSR_Environmental_WindDirection

            | 風向(度数)

        .. object:: SNSR_Environmental_WindSpeed

            | 風速(メートル毎秒)

        .. object:: SNSR_Light_Lux

            | 照度(ルクス)

        .. object:: SNSR_Light_Temperature

            | 光色温度(ケルビン)

        .. object:: SNSR_Mechanical_Force

            | 力(ニュートン)

        .. object:: SNSR_Mechanical_AbsPressure

            | 絶対圧(パスカル)

        .. object:: SNSR_Mechanical_GaugePressure

            | ゲージ圧(パスカル)

        .. object:: SNSR_Mechanical_Weight

            | 重量(キログラム)

        .. object:: SNSR_Motion_AccelerationX
        .. object:: SNSR_Motion_AccelerationY
        .. object:: SNSR_Motion_AccelerationZ

            | X/Y/Z軸 加速度(ガル)

        .. object:: SNSR_Motion_AngleAccelX
        .. object:: SNSR_Motion_AngleAccelY
        .. object:: SNSR_Motion_AngleAccelZ

            | X/Y/Z軸 角加速度(度毎秒毎秒)

        .. object:: SNSR_Motion_Speed

            | 速度(メートル毎秒)

        .. object:: SNSR_Scanner_RFIDTag

            | RFIDタグの40ビット値

        .. object:: SNSR_Scanner_BarcodeData

            | バーコードデータを表す文字列

            .. caution:: UWSCRではサポートされません (必ずEMPTYを返します)


        .. object:: SNSR_Orientation_TiltX
        .. object:: SNSR_Orientation_TiltY
        .. object:: SNSR_Orientation_TiltZ

            | X/Y/Z 軸角(度)

        .. object:: SNSR_Orientation_DistanceX
        .. object:: SNSR_Orientation_DistanceY
        .. object:: SNSR_Orientation_DistanceZ

            | X/Y/Z 距離(メートル)

        .. object:: SNSR_Orientation_MagHeading

            | 磁北基準未補正コンパス方位

        .. object:: SNSR_Orientation_TrueHeading

            | 真北基準未補正コンパス方位

        .. object:: SNSR_Orientation_CompMagHeading

            | 磁北基準補正済みコンパス方位

        .. object:: SNSR_Orientation_CompTrueHeading

            | 真北基準補正済みコンパス方位

        .. object:: SNSR_Location_Altitude

            | 海抜(メートル)

        .. object:: SNSR_Location_Latitude

            | 緯度(度数)

        .. object:: SNSR_Location_Longitude

            | 経度(度数)

        .. object:: SNSR_Location_Speed

            | スピード(ノット)

    :rtype: 真偽値、数値、文字列
    :return: 種別に応じた値、値が取得できない場合はEMPTY

        .. admonition:: UWSCとの違い
            :class: note

            | 一部のエラーで値が取得できない場合にUWSCはNaNを返していましたが、UWSCRではEMPTYが返ります


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


入力制御
--------

.. function:: lockhard(フラグ)

    | マウス、キーボードの入力を禁止する

    .. important:: 実行には管理者特権が必要です

        | UWSCRを **管理者として実行** する必要があります

    .. hint::

        - ``Ctrl+Alt+Delete`` でロック状態を強制解除できます
        - ロックしたままでもUWSCRのプロセスが終了すればロックは解除されます

    :param 真偽値 省略可 フラグ: TRUEで入力禁止、FALSEで解除
    :rtype: 真偽値
    :return: 関数が成功した場合TRUE

.. function:: lockhardex(ID, [モード=LOCK_ALL])

    | ウィンドウに対するマウス、キーボードの入力を禁止する

    .. hint::

        - 管理者特権は不要です
        - ``Ctrl+Alt+Delete`` でロック状態を強制解除できます
        - ロックしたままでもUWSCRのプロセスが終了すればロックは解除されます

    .. important::

        | ロック可能な対象は常に一つです
        | ロック中に別のウィンドウに対してロックを行った場合元のウィンドウは開放されます

    :param 数値 ID: ウィンドウID、0の場合は全体
    :param 定数 省略可 モード: 禁止内容を指定

        .. object:: LOCK_ALL (0)

            | マウス、キーボードの入力を禁止

        .. object:: LOCK_KEYBOARD

            | キーボードの入力のみ禁止

        .. object:: LOCK_MOUSE

            | マウスの入力のみ禁止

    :rtype: 戻り値の型
    :return: 戻り値の説明

音声出力
--------

function:: sound([名前=EMPTY, 同期フラグ=FALSE, 再生デバイス=0])
.. function:: sound([名前=EMPTY, 同期フラグ=FALSE])

    | ファイル名、またはサウンドイベント名を指定しそれを再生する

    .. admonition:: UWSCとの違い
        :class: hint

        | "BEEP" 指定のビープ音再生は廃止されました
        | 代わりに :any:`beep` 関数を使用してください

    .. caution:: wavの再生デバイス選択には対応していません


    :param 文字列 省略可 名前:

        .. object:: ファイル名

            | 再生したいwavファイルのパスを指定

        .. object:: サウンドイベント名

            システム上で定義されているサウンドイベント名を指定

            .. hint:: サウンドイベント名について

                | 環境により登録されているイベント名が異なる可能性があります
                | 以下はWin32のドキュメントに記載されていたイベント名です

                - SystemAsterisk
                - SystemExclamation
                - SystemExit
                - SystemHand
                - SystemQuestion
                - SystemStar

        .. object:: EMPTY

            | 再生を停止します

    :param 真偽値 省略可 同期フラグ: TRUEなら再生終了を待つ
    .. :param 数値 省略可 再生デバイス: wavファイルの出力先デバイスを番号で指定 (0から)
    :return: なし

.. function:: beep([長さ=300, 周波数=2000, 繰り返し=1])

    | ビープ音を再生します

    :param 数値 省略可 長さ: ビープ音を再生する長さをミリ秒で指定
    :param 数値 省略可 周波数: ビープ音の周波数(ヘルツ)を37-32767で指定
    :param 数値 省略可 繰り返し: 同じ長さと周波数のビープ音を繰り返し再生する回数
    :return: なし
