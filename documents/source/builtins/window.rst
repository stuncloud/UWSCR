ウィンドウ操作関数
==================

ID取得
------

.. function:: getid(タイトル, [クラス名=EMPTY, 待ち時間=1])
.. function:: getid(定数)
    :noindex:

    | ウィンドウを検索し、該当するウィンドウを示すIDを返します

    :param 文字列 タイトル: 検索するウィンドウのタイトル (部分一致)
    :param 文字列 省略可 クラス名: 検索するウィンドウのクラス名 (部分一致)
    :param 数値 省略可 待ち時間: ウィンドウが見つからない場合のタイムアウト時間
    :param 定数 定数: 以下の定数を指定

        .. object:: GET_ACTIVE_WIN

            アクティブウィンドウ

        .. object:: GET_FROMPOINT_WIN

            マウスカーソル下のウィンドウ

        .. object:: GET_FROMPOINT_OBJ

            マウスカーソル下の子ウィンドウ


            対象なし(-1を返す)

        .. object:: GET_LOGPRINT_WIN

            Printウィンドウ

        .. object:: GET_BALLOON_WIN
        .. object:: GET_FUKIDASI_WIN

            吹き出し

        .. object:: GET_THISUWSC_WIN
        .. object:: GET_FORM_WIN
        .. object:: GET_FORM_WIN2

            未実装 (-1を返す)

    :return: ウィンドウID、タイムアウトした場合 ``-1``

.. function:: getallwin([ID=EMPTY])

    | すべてのウィンドウのIDを得ます
    | 特定のウィンドウIDを指定した場合、そのウィンドウの子要素を得ます

    :param 数値 省略可 ID: 子要素を取得したいウィンドウのID
    :return: ウィンドウIDの配列

    .. caution::

        | UWSCとは異なり見つかったウィンドウの個数ではなくウィンドウIDの配列が返るようになりました
        | それに伴い特殊変数 ``ALL_WIN_ID`` は廃止されました

.. function:: idtohnd(ID)

    | ウィンドウIDからウィンドウハンドル値を得ます

    :param 数値 ID: ウィンドウID
    :return: ウィンドウハンドル値、該当ウィンドウがない場合は ``0``

.. function:: hndtoid(hwnd)

    | ウィンドウハンドル値からウィンドウIDを得ます

    :param 数値 hwnd: ウィンドウハンドル値
    :return: ウィンドウID、該当ウィンドウがない場合 ``-1``

.. function:: getctlhnd(ID, アイテム名, [n番目=1])
.. function:: getctlhnd(ID, メニュー定数)
    :noindex:

    | 小ウィンドウ(ボタン等)のウィンドウハンドル値、またはメニューハンドルを得ます

    :param 数値 ID: ウィンドウID
    :param 文字列 アイテム名: 小ウィンドウのタイトルまたはクラス名 (部分一致)
    :param 定数 メニュー定数: 以下のいずれかを指定

        .. object:: GET_MENU_HND

            メニューハンドルを返す

        .. object:: GET_SYSMENU_HND

            システムメニューハンドルを返す

    :param 数値 省略可 n番目: n番目に該当するアイテムを探す
    :return: ハンドル値

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            id = getid("ファイル名を指定して実行")
            h1 = getctlhnd(id, "実行するプログラム名、") // タイトルを部分一致
            h2 = getctlhnd(id, "static", 2)              // クラス名指定、2番目
            assert_equal(h1, h2) // 一致

.. _about_id0:

ID0について
^^^^^^^^^^^

| ウィンドウIDを使う一部の関数が実行されると、その関数の対象となったウィンドウが `ID0` に記憶されます
| 次に同様の関数が実行されると `ID0` は上書きされます

.. admonition:: サンプルコード

    .. sourcecode:: uwscr

        ctrlwin(getid("TEST"), HIDE)
        // getid("TEST")のウィンドウがID0に記憶される

        ctrlwin(0, SHOW) // 同じウィンドウに対して実行される

ウィンドウ操作
--------------

.. function:: clkitem(ID, アイテム名, [CLK定数=0, チェック指定=TRUE, n番目=1])

    | ボタン等をクリックします

    :param 数値 ID: 対象のウィンドウID
    :param 文字列 アイテム名: クリックしたいボタンや項目の名前
    :param 定数 省略可 CLK定数: クリックしたいアイテムの種類やクリックの方法を指定します

        これらの定数は ``OR`` で連結することにより複数指定が可能

        - アイテム種別

            | アイテム種別が未指定の場合はすべての種別を検索します
            | (``CLK_BTN or CLK_LIST or CLK_TAB or CLK_MENU or CLK_TREEVIEW or CLK_LISTVIEW or CLK_TOOLBAR or CLK_LINK`` と同等)
            | 複数指定時の検索順は以下の通り

            1. ``CLK_BTN``
            2. ``CLK_LIST``
            3. ``CLK_TAB``
            4. ``CLK_MENU``
            5. ``CLK_TREEVIEW``
            6. ``CLK_LISTVIEW``
            7. ``CLK_TOOLBAR``
            8. ``CLK_LINK``

            .. object:: CLK_BTN

                | ボタン、チェックボックス、ラジオボタン、その他

            .. object:: CLK_LIST

                | リストボックス、コンボボックス

                .. hint:: 複数選択可能なリストボックスでの複数項目指定

                    | アイテム名をタブ文字 (``<#TAB>``) で区切るか、配列指定で複数選択できます

                    .. sourcecode:: uwscr

                        // foo, bar, bazを選択状態にする
                        clkitem(id, "foo<#TAB>bar<#TAB>baz", CLK_LIST) // タブ文字区切り
                        clkitem(id, ["foo", "bar", "baz"], CLK_LIST)   // タブ文字区切り

            .. object:: CLK_TAB

                | タブ

            .. object:: CLK_MENU

                | メニュー

                .. hint:: アイテム名のパス指定

                    | ``ファイル\保存`` のように階層構造をパス表記することもできます

                    .. caution::

                        | ``CLK_ACC`` によるメニューのクリックは失敗する可能性があります
                        | その場合は ``CLK_API`` を指定してください

            .. object:: CLK_TREEVIEW
            .. object:: CLK_TREEVEW

                | ツリービュー

                .. admonition:: 制限事項
                    :class: caution

                    | UWSCR x86版では ``CLK_TREEVIEW or CLK_API`` によるクリック操作に制限があり、
                    | x64のウィンドウに対するクリックが行えません
                    | ``CLK_API`` 以外の方式を指定してください

                .. hint:: アイテム名のパス指定

                    ``root\branch\leaf`` のように階層構造をパス表記することもできます

            .. object:: CLK_LISTVIEW
            .. object:: CLK_LSTVEW

                | リストビュー、ヘッダ

                .. hint:: UWSCからの機能拡張

                    | リストビュー行の一番左だけでなく、どの列のアイテム名でも指定できるようになりました (``CLK_API``)
                    | また、ヘッダ名を指定することでヘッダをクリックできるようになりました (``CLK_API``, ``CLK_ACC``)

            .. object:: CLK_TOOLBAR

                | ツールバー

            .. object:: CLK_LINK

                | リンク

                .. caution::

                    | CLK_APIによるリンククリックは未対応です
                    | CLK_ACCをご利用ください

        - マウスボタン指定

            | マウスボタン指定があった場合はクリック方式に関わらずメッセージ送信(PostMessage)による疑似クリック処理が行われます
            | 未指定の場合はクリック方式別の処理を行います

            .. object:: CLK_RIGHTCLK

                右クリック

            .. object:: CLK_LEFTCLK

                左クリック (CLK_RIGHTCLKと同時指定ならこちらが優先)

            .. object:: CLK_DBLCLK

                ダブルクリック (CLK_LEFTCLKと同時指定で2回目のクリック)

        - クリック方式(API)

            | クリック方式が未指定の場合はすべての方式で検索を行います
            | (``CLK_API`` or ``CLK_UIA`` or ``CLK_ACC`` と同等)
            | クリック方式が複数指定された場合の適用順は以下の通り

            1. ``CLK_API``
            2. ``CLK_UIA``
            3. ``CLK_ACC``

            .. object:: CLK_API

                | Win32 APIによる検索およびクリック
                | クリックは対象アイテムに応じたメッセージ処理を行います

            .. object:: CLK_ACC

                | アクセシビリティコントロールによる検索およびクリック
                | クリックはACCオブジェクトのデフォルトアクションを実行、または選択を行います

            .. object:: CLK_UIA

                | UI Automationによる検索およびクリック

                .. caution:: 未実装です


        - オプション

            .. object:: CLK_BACK

                バックグラウンド処理 (ウィンドウをアクティブにしない)

            .. object:: CLK_MOUSEMOVE
            .. object:: CLK_MUSMOVE

                クリック位置にマウスを移動

            .. object:: CLK_SHORT

                | アイテム名の部分一致
                | 未指定の場合は完全一致する必要があります

            .. object:: CLK_FROMLAST

                逆順サーチ (CLK_ACC指定時のみ有効)

            .. object:: CLK_HWND

                戻り値を対象アイテムのHWNDにする (0は対象不明)


    :param 真偽値 省略可 チェック指定:

        | チェックボックスやメニューの場合、チェックのオンオフを指定 (TRUEならチェックを入れる、FALSEならはずす)
        | 3状態チェックボックスの場合、2を指定することでグレー状態にできます
        | それ以外のアイテムの場合FALSEだとクリック動作を行いません (対象が存在していればTRUEを返す)

        .. caution:: CLK_ACCは3状態チェックボックスをサポートしません

    :param 数値 省略可 n番目: 同名アイテムの場合何番目をクリックするか
    :return: 成功時TRUE、 ``CLK_HWND`` 指定時は対象のウィンドウハンドル値を返す

    .. note:: アイテム名の一致について

        ``CLK_SHORT`` を指定しない場合アイテム名は完全一致する必要がありますが、ニーモニックがある場合はそれを無視することができます

        - ``&`` の有無は問わない
        - ``(&A)`` のように括弧で括られたニーモニックは括弧ごと無視できる
        - 括弧以降にある文字も無視できる

        .. sourcecode:: uwscr

                // &Button
                clkitem(id, "&Button")    // ok, "&"を含めても一致する
                clkitem(id, "Button")     // ok, "&"がなくても一致
                // ボタン(&B)
                clkitem(id, "ボタン(&B)") // ok
                clkitem(id, "ボタン(B)")  // ok, "&"は無視できる
                clkitem(id, "ボタン")     // ok, 括弧ごと無視できる
                // ボタン (&B)
                clkitem(id, "ボタン")     // ok, 括弧の前に半角スペースがあった場合それも無視できる
                // 選択 (&S)...
                clkitem(id, "選択")       // ok, 括弧以降も無視できる

.. function:: ctrlwin(ID, コマンド定数)

    | 対象ウィンドウに命令コマンドを送信します
    | :ref:`ID0 <about_id0>` を更新します

    :param 数値 ID: 対象ウィンドウ
    :param 定数 コマンド定数: 実行したいコマンドを示す定数

        .. object:: CLOSE

            ウィンドウを閉じる

        .. object:: CLOSE2

            ウィンドウを強制的に閉じる

        .. object:: ACTIVATE

            ウィンドウをアクティブにする

        .. object:: HIDE

            ウィンドウを非表示にする

        .. object:: SHOW

            ウィンドウの非表示を解除する

        .. object:: MIN

            ウィンドウを最小化する

        .. object:: MAX

            ウィンドウを最大化する

        .. object:: NORMAL

            ウィンドウを通常サイズに戻す

        .. object:: TOPMOST

            ウィンドウを最前面に固定する

        .. object:: NOTOPMOST

            ウィンドウの最前面固定を解除

        .. object:: TOPNOACTV

            ウィンドウを最前面に移動するがアクティブにはしない


    :return: なし

.. function:: sckey(ID, キー, [キー, ...])

    | ショートカットキーを送信します

    :param ウィンドウID ID: アクティブにするウィンドウのID、0指定でどのウィンドウもアクティブにしない
    :param 定数または文字列 キー: :ref:`virtual_keys` のいずれか、またはアルファベット一文字

        .. note::

            | キーは35個まで指定可能
            | ``VK_SHIFT``, ``VK_CTRL``, ``VK_ALT``, ``VK_WIN`` は押し下げられた状態になります (Rも含む)
            | これらのキーはすべてのキー入力が終了したあとにキーアップ状態に戻ります

    :return: なし

ウィンドウ情報取得
------------------

.. function:: status(ID, ST定数, [ST定数...])

    | 対象ウィンドウの各種状態を取得します

    :param 数値 ID: ウィンドウID
    :param 定数 ST定数: 取得したい状態を示す定数を指定

        | 定数は最大21個指定できます

        .. object:: ST_TITLE

            ウィンドウタイトル (文字列)

        .. object:: ST_CLASS

            ウィンドウクラス名 (文字列)

        .. object:: ST_X

            ウィンドウ左上のX座標 (数値)

        .. object:: ST_Y

            ウィンドウ左上のY座標 (数値)

        .. object:: ST_WIDTH

            ウィンドウの幅 (数値)

        .. object:: ST_HEIGHT

            ウィンドウの高さ (数値)

        .. object:: ST_CLX

            ウィンドウのクライアント領域左上のX座標 (数値)

        .. object:: ST_CLY

            ウィンドウのクライアント領域左上のY座標 (数値)

        .. object:: ST_CLWIDTH

            ウィンドウのクライアント領域の幅 (数値)

        .. object:: ST_CLHEIGHT

            ウィンドウのクライアント領域の高さ (数値)

        .. object:: ST_PARENT

            親ウィンドウのID (数値)

        .. object:: ST_ICON

            最小化してればTRUE (真偽値)

        .. object:: ST_MAXIMIZED

            最大化してればTRUE (真偽値)

        .. object:: ST_VISIBLE

            ウィンドウが可視ならTRUE (真偽値)

        .. object:: ST_ACTIVE

            ウィンドウがアクティブならTRUE (真偽値)

        .. object:: ST_BUSY

            ウィンドウが応答なしならTRUE (真偽値)

        .. object:: ST_ISID

            ウィンドウが有効ならTRUE (真偽値)

        .. object:: ST_WIN64

            プロセスが64ビットかどうか (真偽値)

        .. object:: ST_PATH

            プロセスの実行ファイルのパス (文字列)

        .. object:: ST_PROCESS

            プロセスID (数値)

        .. object:: ST_MONITOR

            ウィンドウが表示されているモニタ番号 (monitor関数に対応) (数値)

        .. object:: ST_ALL

            | すべての状態を取得
            | この定数を指定する場合ほかの定数は指定できません

    :return: ST定数を一つだけ指定した場合は得られた値、複数指定時または ``ST_ALL`` 指定時は連想配列 (キーはST定数)

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            id = getid("uwsc", "HH") // uwscヘルプファイル
            stat = status(id, ST_TITLE, ST_CLASS, ST_HEIGHT, ST_WIDTH)

            print stat[ST_TITLE]  // uwsc
            print stat[ST_CLASS]  // HH Parent
            print stat[ST_HEIGHT] // 778
            print stat[ST_WIDTH]  // 1251

.. function:: monitor(モニタ番号, [MON定数])
.. function:: monitor()
    :noindex:

    | モニタの情報を得ます
    | 引数なしで実行した場合モニタの数を得ます

    :param 数値 省略可 モニタ番号: モニタを示す番号 (0から)
    :param 定数 省略可 MON定数: 取得したい情報を示す定数

        .. object:: MON_X

            モニタのX座標 (数値)

        .. object:: MON_Y

            モニタのY座標 (数値)

        .. object:: MON_WIDTH

            モニタの幅 (数値)

        .. object:: MON_HEIGHT

            モニタの高さ (数値)

        .. object:: MON_PRIMARY
        .. object:: MON_ISMAIN

            プライマリ(メイン)モニタならTRUE (真偽値)

        .. object:: MON_NAME

            モニタ名 (文字列)

        .. object:: MON_WORK_X

            作業エリアのX座標 (数値)

        .. object:: MON_WORK_Y

            作業エリアのY座標 (数値)

        .. object:: MON_WORK_WIDTH

            作業エリアの幅 (数値)

        .. object:: MON_WORK_HEIGHT

            作業エリアの高さ (数値)

        .. object:: MON_ALL

            上記すべて (デフォルト)


    :return:

        - 引数なしで実行: モニタの数
        - 定数指定 (``MON_ALL`` 以外): 得られた値
        - ``MON_ALL`` 指定: 連想配列 (キーはMON定数)
        - 該当モニタなし: ``FALSE``

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            // すべてのモニタのサイズを表示
            for i = 0 to monitor() - 1
                m = monitor(i, MON_ALL)
                print "モニタ" + i + ": " + m[MON_NAME]
                print m[MON_X] + ", " + m[MON_Y]
                print m[MON_WIDTH] + " x " + m[MON_HEIGHT]
            next

.. function:: posacc(ID, クライアントX座標, クライアントY座標, [種別=0])

    | 座標位置のアクセシビリティオブジェクトから情報を得ます

    :param ウィンドウID ID: 対象ウィンドウのID
    :param 数値 クライアントX座標: 対象ウィンドウのクライアント領域におけるX座標
    :param 数値 クライアントY座標: 対象ウィンドウのクライアント領域におけるY座標
    :param 定数 省略可 種別: 取得したい情報の種類を示す定数

        .. object:: 0

            | ``ACC_ACC`` を実行し、取得できなければ ``ACC_API`` を実行 (デフォルト)

        .. object:: ACC_ACC

            | 表示文字列の取得

        .. object:: ACC_API

            | DrawText, TextOut等のAPIで描画されたテキストを取得 (未実装)

        .. object:: ACC_NAME

            | オブジェクトの表示名

        .. object:: ACC_VALUE

            | オブジェクトの値 (エディットボックス等)

        .. object:: ACC_ROLE

            | オブジェクトの役割名

        .. object:: ACC_STATE

            | オブジェクトの状態

        .. object:: ACC_DESCRIPTION

            | オブジェクトの説明

        .. object:: ACC_LOCATION

            | オブジェクトの位置情報
            | [x, y, 幅, 高さ]

        .. object:: ACC_BACK (オプション)

            | 他の定数とOR連結で指定
            | 対象ウィンドウをアクティブにしない
    :rtype: 文字列または配列
    :return:

        | ``ACC_LOCATION`` 指定時は数値の配列を返します
        | ``ACC_STATE`` 指定時は文字列の配列を返します
        | それ以外は該当する値を文字列で返します
        | 失敗時はEMPTYを返します

.. function:: muscur()

    | マウスカーソルの種別を返します

    :rtype: 定数
    :return:

        .. object:: CUR_APPSTARTING (1)

            | 砂時計付き矢印

        .. object:: CUR_ARROW (2)

            | 標準矢印

        .. object:: CUR_CROSS (3)

            | 十字

        .. object:: CUR_HAND (4)

            | ハンド

        .. object:: CUR_HELP (5)

            | クエスチョンマーク付き矢印

        .. object:: CUR_IBEAM (6)

            | アイビーム (テキスト上のカーソル)

        .. object:: CUR_NO (8)

            | 禁止

        .. object:: CUR_SIZEALL (10)

            | ４方向矢印

        .. object:: CUR_SIZENESW (11)

            | 斜め左下がりの両方向矢印

        .. object:: CUR_SIZENS (12)

            | 上下両方向矢印

        .. object:: CUR_SIZENWSE (13)

            | 斜め右下がりの両方向矢印

        .. object:: CUR_SIZEWE (14)

            | 左右両方向矢印

        .. object:: CUR_UPARROW (15)

            | 垂直の矢印

        .. object:: CUR_WAIT (16)

            | 砂時計

        .. object:: 0

            | 上記以外

.. function:: peekcolor(x, y, [RGB指定=COL_BGR, クリップボード=FALSE])

    | 指定位置の色を得ます

    :param 数値 x: X座標
    :param 数値 y: Y座標
    :param 定数 省略可 RGB指定: 戻り値の指定

        .. object:: COL_BGR (0)

            | BGR値で返す
            | 青は$FF0000、緑は$00FF00、赤は$0000FF

        .. object:: COL_RGB

            | RGB値で返す
            | 赤は$FF0000、緑は$00FF00、青は$0000FF

        .. object:: COL_R

            | 赤の成分のみ

        .. object:: COL_G

            | 緑の成分のみ

        .. object:: COL_B

            | 青の成分のみ
    :param 真偽値 省略可 クリップボード:

        .. object:: FALSE

            | 画面の指定座標から

        .. object:: TRUE

            | クリップボード画像の指定座標から

    :rtype: 数値
    :return:

        | 指定座標の色を示す数値
        | 失敗時は ``-1`` (範囲外指定やクリップボード指定でクリップボード画像がない場合)


画像検索
--------

.. hint:: chkimg関数を使う場合chkimg版UWSCR(UWSCRx64_chkimg.zip)を導入してください

.. function:: chkimg(画像ファイルパス, [スコア=95, 最大検索数=5, left=EMPTY, top=EMPTY, right=EMPTY, bottom=EMPTY])

    | 指定画像をスクリーン上から探してその座標を返します

    .. caution:: UWSCとは引数や戻り値が異なります

        特殊変数 ``G_IMG_X``, ``G_IMG_Y``, ``ALL_IMG_X``, ``ALL_IMG_Y`` は廃止

    .. attention:: OpenCV 4.5.4が必要です

        インストール方法等は :ref:`opencv` を参照ください

    :param 文字列 画像ファイルパス: 検索する画像のパス (jpg, bmp, png)
    :param 数値 省略可 スコア: 画像に対する一致率を指定 (0-100)

        | 一致率が指定値以上であれば結果を返します
        | 100が完全一致

    :param 数値 省略可 最大検索数: 検索の試行回数を指定
    :param 数値 省略可 left: 検索範囲指定: 左上X座標、省略時は画面左上X座標
    :param 数値 省略可 top: 検索範囲指定: 左上Y座標、省略時は画面左上Y座標
    :param 数値 省略可 right: 検索範囲指定: 右下X座標、省略時は画面右下X座標
    :param 数値 省略可 bottom: 検索範囲指定: 右下X座標、省略時は画面右下Y座標

    :return: 該当する部分の座標とスコアを格納した二次元配列 ``[[X座標, Y座標, スコア], ...]``

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            for found in chkimg("hoge.png")
                print found // [x, y, スコア]
            next

.. _opencv:

OpenCV導入方法
^^^^^^^^^^^^^^

1. OpenCVのインストール
    1. `Release OpenCV 4.5.4 · opencv/opencv <https://github.com/opencv/opencv/releases/tag/4.5.4>`_ を開く
    2. Asssetsにある ``opencv-4.5.4-vc14_vc15.exe`` をダウンロード
    3. ``opencv-4.5.4-vc14_vc15.exe`` を実行し、任意のフォルダに展開する (例: `C:\\tools`)
2. UWSCRからdllを参照できるようにする (以下のいずれかの方法)
    - 方法1: dllをuwscr.exeと同じフォルダに置く
        1. `{展開先}\\opencv\\build\\x64\\vc15\\bin` の ``opencv_world454.dll`` をコピー
    - 方法2: 環境変数PATHに登録
        1. スタートメニューから **環境変数を編集** を実行
        2. ユーザー環境変数の `Path` をダブルクリック
        3. `{展開先}\\opencv\\build\\x64\\vc15\\bin` を追記 (`{展開先}` は実際のフォルダパスに変換してください 例: `C:\\tools`)
        4. 実行環境(PowerShellなど)を再起動


低レベル関数
------------

.. function:: mmv(x, y, [ms=0])

    | マウスカーソルを移動します

    :param 数値 x: 移動先のX座標
    :param 数値 y: 移動先のY座標
    :param 数値 省略可 ms: マウス移動を行うまでの待機時間 (ミリ秒)
    :return: なし

.. function:: btn(ボタン定数, [状態=CLICK, x=EMPTY, y=EMPTY, ms=0])

    | 指定座標にマウスボタン操作を送信します

    :param 定数 ボタン定数: 操作するマウスボタンを指定

        .. object:: LEFT

            左クリック

        .. object:: RIGHT

            右クリック

        .. object:: MIDDLE

            ホイルクリック

        .. object:: WHEEL

            ホイル回転 (上下方向)

        .. object:: WHEEL2

            ホイル回転 (左右方向)

        .. object:: TOUCH

            タッチ操作 (未実装)

    :param 定数 省略可 状態: マウスボタンに対してどのような操作を行うかを指定

        - ``LEFT``, ``RIGHT``, ``MIDDLE`` の場合以下のいずれかを指定

            .. object:: CLICK

                ボタンクリック (デフォルト)

            .. object:: DOWN

                ボタン押し下げ

            .. object:: UP

                ボタン開放

        - ``WHEEL``: 数値を指定、正の数なら下方向、負の数なら上方向にスクロール
        - ``WHEEL2``: 数値を指定、正の数なら下方向、負の数なら上方向にスクロール

    :param 数値 省略可 x: ボタン操作を行う位置のX座標、省略時は現在のマウスのX座標
    :param 数値 省略可 y: ボタン操作を行う位置のY座標、省略時は現在のマウスのY座標
    :param 数値 省略可 ms: ボタン操作を行うまでの待機時間 (ミリ秒)

    :return: なし

.. function:: kbd(仮想キー, [状態=CLICK, ms=0])
.. function:: kbd(送信文字列, [状態=CLICK, ms=0])
    :noindex:

    | キーボード入力を送信します

    :param 定数 仮想キー: :ref:`virtual_keys` のいずれか
    :param 文字列 送信文字列: キー入力として送信される文字列
    :param 定数 省略可 状態: キーの入力状態を指定、文字列送信時は無視される

        .. object:: CLICK

            キークリック (デフォルト)

        .. object:: DOWN

            キー押し下げ

        .. object:: UP

            キー開放

    :param 数値 省略可 ms: キーボード入力を行うまでの待機時間 (ミリ秒)

    :return: なし

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            // a が入力される
            kbd(VK_A)

            // A が入力される
            kbd(VK_SHIFT, DOWN)
            kbd(VK_A, CLICK, 100)
            kbd(VK_SHIFT, UP, 100)

            // A が入力される
            kbd("A")

            // あ が入力される
            kbd("あ")

            // abcde が入力される
            kbd("abcde")

.. function:: acw(ID, [x=EMPTY, y=EMPTY, h=EMPTY, w=EMPTY, ms=0])

    | ウィンドウの位置やサイズを変更します
    | :ref:`ID0 <about_id0>` を更新します

    :param 数値 ID: ウィンドウID
    :param 数値 省略可 x: 移動先のX座標、省略時は対象ウィンドウの現在のX座標
    :param 数値 省略可 y: 移動先のY座標、省略時は対象ウィンドウの現在のY座標
    :param 数値 省略可 h: 変更するウィンドウの高さ、省略時は対象ウィンドウの現在の高さ
    :param 数値 省略可 w: 変更するウィンドウの幅、省略時は対象ウィンドウの現在の幅
    :param 数値 省略可 ms: ウィンドウに変更を加えるまでの待機時間 (ミリ秒)
    :return: なし

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            acw(getid(GET_ACTIVE_WIN), 100, 100) // ID0を更新
            sleep(1)
            acw(0, 200, 200)

.. _virtual_keys:

仮想キーコード一覧
^^^^^^^^^^^^^^^^^^

.. object:: VK_A
.. object:: VK_B
.. object:: VK_C
.. object:: VK_D
.. object:: VK_E
.. object:: VK_F
.. object:: VK_G
.. object:: VK_H
.. object:: VK_I
.. object:: VK_J
.. object:: VK_K
.. object:: VK_L
.. object:: VK_M
.. object:: VK_N
.. object:: VK_O
.. object:: VK_P
.. object:: VK_Q
.. object:: VK_R
.. object:: VK_S
.. object:: VK_T
.. object:: VK_U
.. object:: VK_V
.. object:: VK_W
.. object:: VK_X
.. object:: VK_Y
.. object:: VK_Z
.. object:: VK_0
.. object:: VK_1
.. object:: VK_2
.. object:: VK_3
.. object:: VK_4
.. object:: VK_5
.. object:: VK_6
.. object:: VK_7
.. object:: VK_8
.. object:: VK_9
.. object:: VK_START
.. object:: VK_BACK
.. object:: VK_TAB
.. object:: VK_CLEAR
.. object:: VK_ESC
.. object:: VK_ESCAPE
.. object:: VK_RETURN
.. object:: VK_ENTER
.. object:: VK_RRETURN
.. object:: VK_SHIFT
.. object:: VK_RSHIFT
.. object:: VK_WIN
.. object:: VK_RWIN
.. object:: VK_ALT
.. object:: VK_MENU
.. object:: VK_RALT
.. object:: VK_CTRL
.. object:: VK_CONTROL
.. object:: VK_RCTRL
.. object:: VK_PAUSE
.. object:: VK_CAPITAL
.. object:: VK_KANA
.. object:: VK_FINAL
.. object:: VK_KANJI
.. object:: VK_CONVERT
.. object:: VK_NONCONVERT
.. object:: VK_ACCEPT
.. object:: VK_MODECHANGE
.. object:: VK_SPACE
.. object:: VK_PRIOR
.. object:: VK_NEXT
.. object:: VK_END
.. object:: VK_HOME
.. object:: VK_LEFT
.. object:: VK_UP
.. object:: VK_RIGHT
.. object:: VK_DOWN
.. object:: VK_SELECT
.. object:: VK_PRINT
.. object:: VK_EXECUTE
.. object:: VK_SNAPSHOT
.. object:: VK_INSERT
.. object:: VK_DELETE
.. object:: VK_HELP
.. object:: VK_APPS
.. object:: VK_MULTIPLY
.. object:: VK_ADD
.. object:: VK_SEPARATOR
.. object:: VK_SUBTRACT
.. object:: VK_DECIMAL
.. object:: VK_DIVIDE
.. object:: VK_NUMPAD0
.. object:: VK_NUMPAD1
.. object:: VK_NUMPAD2
.. object:: VK_NUMPAD3
.. object:: VK_NUMPAD4
.. object:: VK_NUMPAD5
.. object:: VK_NUMPAD6
.. object:: VK_NUMPAD7
.. object:: VK_NUMPAD8
.. object:: VK_NUMPAD9
.. object:: VK_F1
.. object:: VK_F2
.. object:: VK_F3
.. object:: VK_F4
.. object:: VK_F5
.. object:: VK_F6
.. object:: VK_F7
.. object:: VK_F8
.. object:: VK_F9
.. object:: VK_F10
.. object:: VK_F11
.. object:: VK_F12
.. object:: VK_NUMLOCK
.. object:: VK_SCROLL
.. object:: VK_PLAY
.. object:: VK_ZOOM
.. object:: VK_SLEEP
.. object:: VK_BROWSER_BACK
.. object:: VK_BROWSER_FORWARD
.. object:: VK_BROWSER_REFRESH
.. object:: VK_BROWSER_STOP
.. object:: VK_BROWSER_SEARCH
.. object:: VK_BROWSER_FAVORITES
.. object:: VK_BROWSER_HOME
.. object:: VK_VOLUME_MUTE
.. object:: VK_VOLUME_DOWN
.. object:: VK_VOLUME_UP
.. object:: VK_MEDIA_NEXT_TRACK
.. object:: VK_MEDIA_PREV_TRACK
.. object:: VK_MEDIA_STOP
.. object:: VK_MEDIA_PLAY_PAUSE
.. object:: VK_LAUNCH_MEDIA_SELECT
.. object:: VK_LAUNCH_MAIL
.. object:: VK_LAUNCH_APP1
.. object:: VK_LAUNCH_APP2
.. object:: VK_OEM_PLUS
.. object:: VK_OEM_COMMA
.. object:: VK_OEM_MINUS
.. object:: VK_OEM_PERIOD
.. object:: VK_OEM_1
.. object:: VK_OEM_2
.. object:: VK_OEM_3
.. object:: VK_OEM_4
.. object:: VK_OEM_5
.. object:: VK_OEM_6
.. object:: VK_OEM_7
.. object:: VK_OEM_8
.. object:: VK_OEM_RESET
.. object:: VK_OEM_JUMP
.. object:: VK_OEM_PA1
.. object:: VK_OEM_PA2
.. object:: VK_OEM_PA3