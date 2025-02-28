システム関数
============

システム情報
------------

.. function:: kindofos(データ種別=FALSE)

    OS種別、またはアーキテクチャを判定します

    :param 真偽値または定数 省略可 データ種別: 以下のいずれかを指定

        .. object:: IS_64BIT_OS, TRUE

            OSが64ビットかどうかを真偽値で返す

        .. object:: KIND_OF_OS, FALSE

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

.. function:: setenv(環境変数, 設定値)

    | プロセス環境変数を設定します

    .. hint:: この環境変数は実行中のuwscrプロセス及びその子プロセスに対してのみ有効です

    :param 文字列 環境変数: 環境変数名
    :param 文字列 設定値: 環境変数にセットする値

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            print env('NO_PROXY') // 空文字
            setenv('NO_PROXY', 'localhost')
            print env('NO_PROXY') // localhost


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

        .. admonition:: 戻り値NaNの廃止
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

    .. admonition:: 要管理者特権
        :class: important

        | UWSCRを **管理者として実行** する必要があります

    .. admonition:: ロックの強制解除
        :class: hint

        - ``Ctrl+Alt+Delete`` でロック状態を強制解除できます
        - ロックしたままでもUWSCRのプロセスが終了すればロックは解除されます

    :param 真偽値 省略可 フラグ: TRUEで入力禁止、FALSEで解除
    :rtype: 真偽値
    :return: 関数が成功した場合TRUE

.. function:: lockhardex([ID=EMPTY, モード=LOCK_ALL])

    | ウィンドウに対するマウス、キーボードの入力を禁止する

    .. admonition:: ロックの強制解除
        :class: hint

        - ``Ctrl+Alt+Delete`` でロック状態を強制解除できます
        - ロックしたままでもUWSCRのプロセスが終了すればロックは解除されます

    .. admonition:: ロック対象について
        :class: important

        | ロック可能な対象は常に一つです
        | ロック中に別のウィンドウに対してロックを行った場合元のウィンドウは開放されます

    :param 数値 省略可 ID: 入力を禁止するウィンドウのID、0の場合はデスクトップ全体、EMPTYならロックを解除
    :param 定数 省略可 モード: 禁止内容を指定

        .. object:: LOCK_ALL (0)

            | マウス、キーボードの入力を禁止

        .. object:: LOCK_KEYBOARD

            | キーボードの入力のみ禁止

        .. object:: LOCK_MOUSE

            | マウスの入力のみ禁止

    :rtype: 真偽値
    :return: 関数が成功した場合TRUE

音声出力
--------

.. .. function:: sound([名前=EMPTY, 同期フラグ=FALSE, 再生デバイス=0])
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

キー入力
--------

.. function:: getkeystate(キーコード, [ID=0])

    | マウスやキーボードがクリックされたかどうか、または特定のキーのトグル状態を得る

    :param 定数 キーコード: VK定数(クリック判定)またはTGL定数(トグル判定)

        .. object:: VK定数

            | :ref:`virtualkeys` を参照

        .. object:: TGL_NUMLOCK

            | Num Lock

        .. object:: TGL_CAPSLOCK

            | Caps Lock

        .. object:: TGL_SCROLLLOCK

            | Scroll Lock

        .. object:: TGL_KANALOCK

            | カタカナ入力 (要ID指定)

        .. object:: TGL_IME

            | IME (要ID指定)

    :param 数値 省略可 ID:

        | ``TGL_KANALOCK``, ``TGL_IME`` にて入力方式を確認したいウィンドウのID
        | 0ならアクティブウィンドウ

    :rtype: 真偽値
    :return: クリックされていたらTRUE、またはトグル状態がオンならTRUE

.. function:: sethotkey(キーコード, [修飾子キー=0, 関数=EMPTY])

    | 関数をホットキーに登録

    :param 定数 キーコード: 登録するキーコードをVK定数で指定 (VK定数は :ref:`virtualkeys` を参照)
    :param 定数 省略可 修飾子キー: 同時に押す修飾子キーを指定、OR連結で複数指定、0なら修飾子キーなし

        .. object:: MOD_ALT

            | Altキー

        .. object:: MOD_CONTROL

            | Controlキー

        .. object:: MOD_SHIFT

            | Shiftキー

        .. object:: MOD_WIN

            | Winキー

    :param 文字列またはユーザー定義関数 省略可 関数:

        | ホットキー入力時に実行するユーザー定義関数、またはその名前を文字列で指定
        | 省略時、またはEMPTYや空文字が入力された場合はホットキーを解除する

        .. admonition:: 指定関数の注意点
            :class: caution

            | 引数を受ける関数の場合、引数は無視されます (引数0の関数として扱われる)
            | 関数内で引数へのアクセスを行う場合はエラーになります
            | 関数内でエラーが発生した場合はスクリプトが強制終了されます

        .. admonition:: HOTKEY特殊変数
            :class: hint

            | ホットキーで呼ばれる関数内では以下の変数が使えます

            .. object:: HOTKEY_VK

                | ホットキーのキーコード

            .. object:: HOTKEY_MOD

                | ホットキーの修飾子キー

    :return: なし


.. _virtualkeys:

仮想キーコード一覧
^^^^^^^^^^^^^^^^^^

.. list-table::

    * - ``VK_A``
      - ``VK_B``
      - ``VK_C``
      - ``VK_D``
      - ``VK_E``
    * - ``VK_F``
      - ``VK_G``
      - ``VK_H``
      - ``VK_I``
      - ``VK_J``
    * - ``VK_K``
      - ``VK_L``
      - ``VK_M``
      - ``VK_N``
      - ``VK_O``
    * - ``VK_P``
      - ``VK_Q``
      - ``VK_R``
      - ``VK_S``
      - ``VK_T``
    * - ``VK_U``
      - ``VK_V``
      - ``VK_W``
      - ``VK_X``
      - ``VK_Y``
    * - ``VK_Z``
      - ``VK_0``
      - ``VK_1``
      - ``VK_2``
      - ``VK_3``
    * - ``VK_4``
      - ``VK_5``
      - ``VK_6``
      - ``VK_7``
      - ``VK_8``
    * - ``VK_9``
      - ``VK_BACK``
      - ``VK_TAB``
      - ``VK_CLEAR``
      - ``VK_ESCAPE``
    * - ``VK_ESC``
      - ``VK_ENTER``
      - ``VK_RETURN``
      - ``VK_RRETURN``
      - ``VK_SHIFT``
    * - ``VK_RSHIFT``
      - ``VK_WIN``
      - ``VK_RWIN``
      - ``VK_START``
      - ``VK_MENU``
    * - ``VK_ALT``
      - ``VK_RALT``
      - ``VK_CONTROL``
      - ``VK_CTRL``
      - ``VK_RCTRL``
    * - ``VK_PAUSE``
      - ``VK_CAPITAL``
      - ``VK_KANA``
      - ``VK_FINAL``
      - ``VK_KANJI``
    * - ``VK_CONVERT``
      - ``VK_NONCONVERT``
      - ``VK_ACCEPT``
      - ``VK_MODECHANGE``
      - ``VK_SPACE``
    * - ``VK_PRIOR``
      - ``VK_NEXT``
      - ``VK_END``
      - ``VK_HOME``
      - ``VK_LEFT``
    * - ``VK_UP``
      - ``VK_RIGHT``
      - ``VK_DOWN``
      - ``VK_SELECT``
      - ``VK_PRINT``
    * - ``VK_EXECUTE``
      - ``VK_SNAPSHOT``
      - ``VK_INSERT``
      - ``VK_DELETE``
      - ``VK_HELP``
    * - ``VK_APPS``
      - ``VK_MULTIPLY``
      - ``VK_ADD``
      - ``VK_SEPARATOR``
      - ``VK_SUBTRACT``
    * - ``VK_DECIMAL``
      - ``VK_DIVIDE``
      - ``VK_NUMPAD0``
      - ``VK_NUMPAD1``
      - ``VK_NUMPAD2``
    * - ``VK_NUMPAD3``
      - ``VK_NUMPAD4``
      - ``VK_NUMPAD5``
      - ``VK_NUMPAD6``
      - ``VK_NUMPAD7``
    * - ``VK_NUMPAD8``
      - ``VK_NUMPAD9``
      - ``VK_F1``
      - ``VK_F2``
      - ``VK_F3``
    * - ``VK_F4``
      - ``VK_F5``
      - ``VK_F6``
      - ``VK_F7``
      - ``VK_F8``
    * - ``VK_F9``
      - ``VK_F10``
      - ``VK_F11``
      - ``VK_F12``
      - ``VK_NUMLOCK``
    * - ``VK_SCROLL``
      - ``VK_PLAY``
      - ``VK_ZOOM``
      - ``VK_SLEEP``
      - ``VK_BROWSER_BACK``
    * - ``VK_BROWSER_FORWARD``
      - ``VK_BROWSER_REFRESH``
      - ``VK_BROWSER_STOP``
      - ``VK_BROWSER_SEARCH``
      - ``VK_BROWSER_FAVORITES``
    * - ``VK_BROWSER_HOME``
      - ``VK_VOLUME_MUTE``
      - ``VK_VOLUME_DOWN``
      - ``VK_VOLUME_UP``
      - ``VK_MEDIA_NEXT_TRACK``
    * - ``VK_MEDIA_PREV_TRACK``
      - ``VK_MEDIA_STOP``
      - ``VK_MEDIA_PLAY_PAUSE``
      - ``VK_LAUNCH_MEDIA_SELECT``
      - ``VK_LAUNCH_MAIL``
    * - ``VK_LAUNCH_APP1``
      - ``VK_LAUNCH_APP2``
      - ``VK_OEM_PLUS``
      - ``VK_OEM_COMMA``
      - ``VK_OEM_MINUS``
    * - ``VK_OEM_PERIOD``
      - ``VK_OEM_1``
      - ``VK_OEM_2``
      - ``VK_OEM_3``
      - ``VK_OEM_4``
    * - ``VK_OEM_5``
      - ``VK_OEM_6``
      - ``VK_OEM_7``
      - ``VK_OEM_8``
      - ``VK_OEM_RESET``
    * - ``VK_OEM_JUMP``
      - ``VK_OEM_PA1``
      - ``VK_OEM_PA2``
      - ``VK_OEM_PA3``
      - ``VK_LBUTTON``
    * - ``VK_RBUTTON``
      - ``VK_MBUTTON``
      -
      -
      -

システム制御
------------

.. function:: poff(コマンド, [スクリプト再実行=TRUE])

    | 電源等の制御

    :param 定数 コマンド: 制御方法を示す定数

        .. object:: P_POWEROFF

            | PCの電源オフ

        .. object:: P_SHUTDOWN

            | PCの電源を切れる状態までOSをシャットダウンする

        .. object:: P_LOGOFF または P_SIGNOUT

            | 現在のユーザーをサインアウトする

        .. object:: P_REBOOT

            | PCを再起動する

        .. object:: P_SUSPEND または P_HIBERNATE

            | PCを休止状態にする
            | システムが休止をサポートしている必要があります

        .. object:: P_SUSPEND2 または P_SLEEP

            | PCをスリープ状態にする

        .. object:: P_MONIPOWER または P_MONITOR_POWERSAVE

            | モニタを省電力モードにする
            | モニタが省電力機能をサポートしている必要があります

        .. object:: P_MONIPOWER2 または P_MONITOR_OFF

            | モニタの電源を切る
            | モニタが省電力機能をサポートしている必要があります

        .. object:: P_MONIPOWER3 または P_MONITOR_ON

            | モニタの電源を入れる
            | モニタが省電力機能をサポートしている必要があります

        .. object:: P_SCREENSAVE

            | スクリーンセーバーを起動

        .. object:: P_UWSC_REEXEC

            | UWSCRの再起動
            | 第二引数がTRUEならスクリプトを再実行する

            .. admonition:: 無限ループに注意
                :class: caution

                | スクリプト再実行を行う場合はpoffの実行条件に注意してください
                | 繰り返しスクリプトの再実行が行われるおそれがあります

            .. admonition:: コンソールモード中の場合
                :class: important

                | ウィンドウモードで再実行されます

        .. object:: P_FORCE

            | アプリケーションの終了を待たずにサインアウトしたい場合や、シャットダウンを強制したい場合に指定
            | ``P_POWEROFF``, ``P_SHUTDOWN``, ``P_LOGOFF``, ``P_REBOOT`` のいずれかに ``OR`` で連結指定する
            | それ以外の場合は無視される

            .. sourcecode:: uwscr

                poff(P_POWEROFF or P_FORCE) // 強制電源断

    :param 真偽値 省略可 スクリプト再実行: TRUEなら ``P_UWSC_REEXEC`` 指定時にスクリプトを再実行する

        .. admonition:: UWSCとの違い
            :class: note

            | デフォルト値がTRUEになりました

    :return: なし

    .. admonition:: OPTFINALLY指定時の動作
        :class: hint

        | 自身のプロセス終了を伴う以下のコマンドが ``try`` 節で実行された場合
        | OPTFINALLY指定時に限り ``finally`` 節が実行されます

        - 自身を終了する前にfinallyを実行
            - ``P_UWSC_REEXEC``
        - コマンド呼び出し前にfinallyを実行 (finally節が終了するまでこれらの処理は行われない)
            - ``P_POWEROFF``
            - ``P_SHUTDOWN``
            - ``P_LOGOFF``
            - ``P_REBOOT``

        .. sourcecode:: uwscr

            OPTION OPTFINALLY
            try
                poff(P_UWSC_REEXEC)
                msgbox("poff以降は実行されない")
            finally
                msgbox("finallyが実行される")
            endtry

        .. sourcecode:: uwscr

            // OPTFINALLYが無い場合
            try
                poff(P_UWSC_REEXEC)
            finally
                msgbox("OPTFINALLYがないので実行されない")
            endtry

        .. sourcecode:: uwscr

            OPTION OPTFINALLY
            // poffがtryの外にある場合
            poff(P_UWSC_REEXEC)
            try
            finally
                msgbox("OPTFINALLYがあってもtry外だと実行されない")
            endtry

    .. admonition:: シャットダウンの理由
        :class: note

        | poffによるシャットダウンは以下の理由で行われます

        - ``SHTDN_REASON_MAJOR_OTHER``
        - ``SHTDN_REASON_MINOR_OTHER``
        - ``SHTDN_REASON_FLAG_PLANNED``

日時
----

.. function:: gettime([補正値=0, 基準日時=EMPTY, 補正値オプション=G_OFFSET_DAYS, ミリ秒=FALSE])

    | 指定日時の2000年1月1日からの経過時間を得る
    | またその時間に該当する日時情報を ``G_TIME_*`` 特殊変数に格納する

    :param 数値 省略可 補正値: 基準日時を起点として指定値分ずらした日時を得る
    :param 文字列 省略可 基準日時:

        | 基準となる日時を指定、EMPTYで現在時刻
        | 以下の形式で指定

        - "YYYYMMDD"
        - "YYYY/MM/DD"
        - "YYYY-MM-DD"
        - "YYYYMMDDHHNNSS"
        - "YYYY/MM/DD HH:NN:SS"
        - "YYYY-MM-DD HH:NN:SS"
        - RFC 3339 形式
            - タイムゾーン情報を含めた場合ローカル時間に変換されます


    :param 定数 省略可 補正値オプション: 補正値の指定方法を指定

        .. object:: G_OFFSET_DAYS

            | 補正値を日数として扱う

        .. object:: G_OFFSET_HOURS

            | 補正値を時間として扱う

        .. object:: G_OFFSET_MINUTES

            | 補正値を分として扱う

        .. object:: G_OFFSET_SECONDS

            | 補正値を秒として扱う

        .. object:: G_OFFSET_MILLIS

            | 補正値をミリ秒として扱う


    :param 真偽値 省略可 ミリ秒: 戻り値を秒ではなくミリ秒で返す
    :rtype: 数値
    :return: 2000年1月1日からの秒数

    .. admonition:: 実行後に変更される特殊変数
        :class: note

        | 以下の変数がgettime関数の結果に応じて変更されます
        | 文字列型の場合は桁数分左0埋めされます
        | これらの変数の変更が適用されるのはgettimeを呼び出したスコープ内に限定されます

        .. list-table::
            :header-rows: 1
            :align: left

            * - 変数
              - 型
              - 内容
            * - ``G_TIME_YY``
              - 数値
              - 年
            * - ``G_TIME_MM``
              - 数値
              - 月
            * - ``G_TIME_DD``
              - 数値
              - 日
            * - ``G_TIME_HH``
              - 数値
              - 時
            * - ``G_TIME_NN``
              - 数値
              - 分
            * - ``G_TIME_SS``
              - 数値
              - 秒
            * - ``G_TIME_ZZ``
              - 数値
              - ミリ秒
            * - ``G_TIME_WW``
              - 数値
              - 曜日 (0:日,1:月,2:火,3:水,4:木,5:金,6:土)
            * - ``G_TIME_YY2``
              - 文字列
              - 年 (下2桁)
            * - ``G_TIME_MM2``
              - 文字列
              - 月 (2桁)
            * - ``G_TIME_DD2``
              - 文字列
              - 日 (2桁)
            * - ``G_TIME_HH2``
              - 文字列
              - 時 (2桁)
            * - ``G_TIME_NN2``
              - 文字列
              - 分 (2桁)
            * - ``G_TIME_SS2``
              - 文字列
              - 秒 (2桁)
            * - ``G_TIME_ZZ2``
              - 文字列
              - ミリ秒 (3桁)
            * - ``G_TIME_YY4``
              - 文字列
              - 年 (4桁)

        | 以下の定数で ``G_TIME_WW`` と比較ができます

        - ``G_WEEKDAY_SUN`` (0)
        - ``G_WEEKDAY_MON`` (1)
        - ``G_WEEKDAY_TUE`` (2)
        - ``G_WEEKDAY_WED`` (3)
        - ``G_WEEKDAY_THU`` (4)
        - ``G_WEEKDAY_FRI`` (5)
        - ``G_WEEKDAY_SAT`` (6)

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            dt = "2023/04/01 10:10:10"

            // 戻り値
            print gettime(, dt)        // 733659010
            // ミリ秒で返す
            print gettime(, dt,, TRUE) // 733659010000

            // 第一引数の単位変更
            // 従来の書き方で6時間ずらす
            ts1 = gettime(0.25, dt)
            // 第三引数指定で時間扱いになる
            ts2 = gettime(6, dt, G_OFFSET_HOURS)

            assert_equal(ts1, ts2)

            // format関数による日時フォーマット
            ts = gettime(, dt)
            print format(ts, "%c") // 2023年04月01日 10時10分10秒

            // RFC3339形式
            ts = gettime(, "2023-10-10T00:00:00+0000")
            print format(ts, "%c") // 2023年10月10日 09時00分00秒

音声
----

.. function:: speak(発声文字, [非同期=FALSE, 中断=FALSE])

    | 指定文字列を音声として再生する

    :param 文字列 発声文字: 発声させたい文字列
    :param 真偽値 省略可 非同期: TRUEなら非同期で発声、FALSEなら発声終了を待つ
    :param 真偽値 省略可 中断: 別の音声が発生中の場合にTRUEなら中断し、FALSEなら終了を待ってから発声させる

        .. admonition:: 終了待ちについて
            :class: note

            | 音声の終了待ちは、speak関数を非同期TRUEで事前に実行していた場合のみ有効です
            | また、speak関数は同一スレッド上で実行されている必要があります

    :return: なし

.. function:: recostate(開始フラグ, [登録単語...])

    | 任意の単語を登録し音声認識を開始、または終了する

    .. admonition:: 有効範囲はスレッド単位
        :class: note

        | 登録及び解除は同一スレッド上でのみ有効です

    :param 真偽値 開始フラグ: TRUEで音声認識を開始、FALSEで解除
    :param 文字列または文字列の配列 登録単語: 開始フラグがTRUEの場合に音声認識させたい言葉を指定、未指定の場合は認識エンジンの標準辞書を使用する
    :rtype: 文字列
    :return: 使用する認識エンジン名、登録失敗時はEMPTY


.. function:: dictate([拾得待ち=TRUE, タイムアウト=10000])

    | recostate関数で登録した単語が音声入力されたらその文字列を返す

    .. admonition:: 単語登録について
        :class: note

        | recostate関数が開始フラグTRUEで実行されていない場合は即座に終了しEMPTYを返します
        | また、recostate関数は同一スレッド上で実行されている必要があります

    :param 真偽値 省略可 拾得待ち: TRUEなら入力を待つ、FALSEなら直近の入力を返す(入力がなければEMPTYを返す)
    :param 数値 省略可 タイムアウト: 拾得待ちがTRUEだった場合に待機する時間(ミリ秒)、0なら無限に待つ、拾得待ちFALSEなら無視される
    :rtype: 文字列
    :return: 拾得した文字列、拾得待ちTRUEでタイムアウトまたは拾得待ちFALSEで入力がない場合はEMPTY

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            // 単語を登録する
            print recostate(TRUE, "りんご", "みかん", "いちご", "おわり")
            print "「りんご」「みかん」「いちご」に反応します"
            print "「おわり」で終了"

            while TRUE
                select word := dictate(TRUE)
                    case "おわり"
                        print "終了します"
                        break
                    case EMPTY
                        // デフォルトでは10000ミリ秒経過でタイムアウト
                        print "タイムアウトしました"
                        break
                    default
                        print word
                selend
            wend

            // 登録を解除
            recostate(false)