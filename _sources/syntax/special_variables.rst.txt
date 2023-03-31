特殊変数
========

環境や実行方法により変化する特殊な変数です

.. object:: GET_UWSC_PRO

    ``EMPTY``

    .. tip:: 実行環境の判定方法

        .. sourcecode:: uwscr

            select GET_UWSC_PRO
                case EMPTY
                    print "UWSCRです"
                case TRUE
                    print "UWSC Pro版です"
                case FALSE
                    print "UWSC Free版です"
            selend

.. object:: GET_UWSC_VER
.. object:: GET_UWSCR_VER

    UWSCRのバージョンを返す

.. object:: PARAM_STR

    起動時パラメータを格納した配列

.. object:: GET_UWSC_DIR
.. object:: GET_UWSCR_DIR

    uwscr.exeのあるフォルダ

.. object:: GET_UWSC_NAME
.. object:: GET_UWSCR_NAME

    スクリプトファイルの名前

.. object:: GET_FUNC_NAME

    | ユーザー定義関数の名前
    | 関数内でのみ有効(関数外では未定義)
    | 無名関数の場合はEMPTY

.. object:: GET_WIN_DIR

    windowsフォルダのパス

.. object:: GET_SYS_DIR

    systemフォルダのパス

.. object:: GET_APPDATA_DIR

    appdataのパス

.. object:: GET_CUR_DIR

    カレントディレクトリ

.. object:: G_MOUSE_X

    マウスポインタのX座標

.. object:: G_MOUSE_Y

    マウスポインタのY座標

.. object:: G_SCREEN_W

    画面全体の幅

.. object:: G_SCREEN_H

    画面全体の高さ

.. object:: G_SCREEN_C

    色数(１ピクセルのビット数)
