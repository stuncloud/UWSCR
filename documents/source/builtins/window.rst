ウィンドウ操作関数
==================

ID取得
------

.. function:: getid(タイトル, [クラス名=EMPTY, 待ち時間=1])

    | ウィンドウを検索し、該当するウィンドウを示すIDを返します
    | 見つからない場合やタイムアウトした場合-1を返します

    :param 文字列 タイトル: 検索するウィンドウのタイトル (部分一致)
    :param 文字列 省略可 クラス名: 検索するウィンドウのクラス名 (部分一致)
    :param 数値 省略可 待ち時間: ウィンドウが見つからない場合のタイムアウト時間
    :rtype: 数値
    :return: ウィンドウID、失敗時は ``-1``

.. function:: getid(定数)
    :noindex:

    :param 定数 定数: 以下の定数を指定

        .. object:: GET_ACTIVE_WIN

            | アクティブウィンドウ

        .. object:: GET_FROMPOINT_WIN

            | マウスカーソル下のウィンドウ

        .. object:: GET_FROMPOINT_OBJ

            | マウスカーソル下の子ウィンドウ

        .. object:: GET_LOGPRINT_WIN

            | Printウィンドウ

        .. object:: GET_BALLOON_WIN
        .. object:: GET_FUKIDASI_WIN

            | 吹き出し

        .. object:: GET_THISUWSC_WIN
        .. object:: GET_CONSOLE_WIN

            | UWSCRを実行しているコンソールウィンドウのIDを返します

        .. object:: GET_FORM_WIN
        .. object:: GET_FORM_WIN2

            | 未実装 (-1を返す)

    :rtype: 数値
    :return: ウィンドウID

.. function:: getallwin([ID=EMPTY])

    | すべてのウィンドウのIDを得ます
    | 特定のウィンドウIDを指定した場合、そのウィンドウの子要素を得ます

    :param 数値 省略可 ID: 子要素を取得したいウィンドウのID
    :return: ウィンドウIDの配列

    .. admonition:: 特殊変数の廃止
        :class: caution

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

    | 子ウィンドウ(ボタン等)のウィンドウハンドル値、またはメニューハンドルを得ます

    :param 数値 ID: ウィンドウID
    :param 文字列 アイテム名: 子ウィンドウのタイトルまたはクラス名 (部分一致)
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

                        | ``CLK_API`` でのみ使用可能です

            .. object:: CLK_TREEVIEW
            .. object:: CLK_TREEVEW

                | ツリービュー

                .. admonition:: 制限事項
                    :class: caution

                    | UWSCR x86版では ``CLK_TREEVIEW or CLK_API`` によるクリック操作に制限があり、
                    | x64のウィンドウに対するクリックが行えません
                    | ``CLK_API`` 以外の方式を指定してください

                .. hint::

                    | アイテム名は ``root\branch\leaf`` のように階層構造を表すパス形式も指定できます
                    | ``CLK_UIA`` で未展開のツリーを展開してクリックするためにはパス形式を指定する必要があります
                    | ``CLK_UIA`` で枝要素を指定した場合、枝が閉じていれば開き、開いていれば閉じます

            .. object:: CLK_LISTVIEW
            .. object:: CLK_LSTVEW

                | リストビュー、ヘッダ

                .. hint:: UWSCからの機能拡張

                    - リストビュー行の一番左だけでなく、どの列のアイテム名でも指定できるようになりました (``CLK_API/CLK_UIA``)
                    - ヘッダ名を指定することでヘッダをクリックできるようになりました (``CLK_API/CLK_ACC/CLK_UIA``)
                    - 複数行を選択できるようになりました (``CLK_UIA``)

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
        | 3状態チェックボックスの場合、 ``2`` を指定することでグレー状態にできます
        | それ以外のアイテムの場合FALSEだとクリック動作を行いません (対象が存在していればTRUEを返す)

        .. admonition:: 3状態チェックボックスのサポート
            :class: caution

            | CLK_APIとCLK_UIAのみ
            | CLK_ACCは3状態チェックボックスをサポートしません

        .. admonition:: CLK_UIA指定時の2の動作
            :class: note

            | 2状態チェックボックスに対してCLK_UIAで2を指定した場合は、クリック操作が複数回行われますが元々の状態に戻ります


    :param 数値 省略可 n番目: 同名アイテムの場合何番目をクリックするか

        .. admonition:: UWSCとは順序が異なる場合があります
            :class: caution

            | 実装の違いによりUWSCとは別の番号を指定しなければならない可能性があります
            | ご注意ください

    :return: 成功時TRUE、 ``CLK_HWND`` 指定時は対象のウィンドウハンドル値を返す

    .. admonition:: アイテム名の一致について
        :class: note

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
    :param 定数または文字列 キー: :ref:`virtual_keys` のいずれかまたはアルファベット一文字、35個まで

        .. admonition:: 修飾子キー指定について
            :class: note

            | ``VK_SHIFT``, ``VK_CTRL``, ``VK_ALT``, ``VK_WIN`` は押し下げられた状態になります (Rも含む)
            | これらのキーはすべてのキー入力が終了したあとにキーアップ状態に戻ります

    :return: なし

.. function:: setslider(ID, 値, [n番目=1, スクロール=FALSE])

    | スライダー(スクロールバー、トラックバー)の値を設定します

    :param ウィンドウID ID: 対象ウィンドウのID
    :param 数値 値: スライダーに設定する値

        .. admonition:: 範囲外指定時の動作
            :class: hint

            | 最大値を上回る値だった場合は最大値に、最小値を下回る値だった場合は最小値に変更されます

    :param 数値 省略可 n番目: n番目のスライダーを設定する
    :param 真偽値 省略可 スクロール: TRUEならスクロールバーを少しずつ動かす
    :rtype: 真偽値
    :return: 成功時TRUE、失敗または操作不能時はFALSE

.. function:: sendstr(ID, 文字列, [n番目=0, 送信モード=FALSE, ACC指定=FALSE])

    | エディットボックスに文字列を送信します

    :param 数値 ID: 対象ウィンドウのID

        | 0ならクリップボードに送信 (その場合n番目、送信モード、ACC指定は無視されます)

    :param 文字列 文字列: 送信する文字列
    :param 数値 n番目: n番目のエディットボックスに送信

        | 0ならフォーカスされたエディットボックス (対象ウィンドウは必ずアクティブになる)

        .. admonition:: UWSCとは順序が異なる場合があります
            :class: caution

            | 実装の違いによりUWSCとは別の番号を指定しなければならない可能性があります
            | ご注意ください

    :param 真偽値または数値 送信モード:

        .. object:: FALSE または 0

            | 追記

        .. object:: TRUE または 1

            | 置き換え

        .. object:: 2

            | 一文字ずつ送信
            | ACC時は無視されます (TRUE扱い)

    :param 真偽値または定数 ACC指定:

        .. object:: FALSE または 0

            | APIまたはUIAを使用

            .. hint::

                | APIで検索を行い該当するものがなかった場合はUIAでの検索を試みます
                | UIA使用時は送信モードは無視され、常に置き換えられます

        .. object:: TRUE または 1

            | ACCを使用

        .. object:: STR_ACC_CELL (5)

            | DataGridView内のCell値の変更 (ACCを使用)

        .. object:: STR_UIA (6)

            | UIAを使用
            | 送信モードは無視され、常に置き換えられます

        .. admonition:: UWSCとの違い
            :class: note

            | TRUEでも対象ウィンドウをアクティブにしないため、2は廃止されました

    :return: なし

.. function:: mouseorg(ID, [基準=MORG_WINDOW, 画面取得=MORG_FORE, HWND=FALSE])

    | 以下の関数にて座標の始点(0, 0)を特定のウィンドウ基準とする

    - :any:`mmv`
    - :any:`btn`
    - :any:`chkimg` (指定座標及び戻り値の座標)
    - :any:`chkclr` (指定座標及び戻り値の座標)
    - :any:`peekcolor`

    | `MORG_DIRECT` を指定した場合は以下も対象となる

    - :any:`kbd`

    :param 数値 ID: ウィンドウID または HWND

        | 該当するIDが存在しない場合は失敗となるが、基準に ``MORG_DIRECT`` が指定されている場合はこの値をHWNDとして扱う
        | IDまたはHWNDに該当する有効なウィンドウが存在しない場合は失敗となる
        | ``0`` が指定された場合はスクリーン座標基準に戻す (この場合以下の引数は無視される)

    :param 定数 省略可 基準: 座標の始点を指定する

        .. object:: MORG_WINDOW (0)

            | 対象ウィンドウのウィンドウ領域左上を基準とする

        .. object:: MORG_CLIENT

            | 対象ウィンドウのクライアント領域左上を基準とする

        .. object:: MORG_DIRECT

            | 対象ウィンドウのクライアント領域左上を基準とする
            | また :any:`mmv`, :any:`btn` 及び :any:`kbd` 関数のマウス・キー操作をウィンドウに直接送信(``SendMessage``)する
            | 送信するメッセージは以下 (対象ウィンドウがこれらのメッセージを処理しない場合操作は無効となる)

            .. list-table::
                :header-rows: 1
                :align: left

                * - 関数
                  - 操作
                  - メッセージ
                * - mmv
                  - カーソル移動
                  - ``WM_MOUSEMOVE``
                * - btn
                  - 左ボタン下げ
                  - ``WM_LBUTTONDOWN``
                * - btn
                  - 左ボタン上げ
                  - ``WM_LBUTTONUP``
                * - btn
                  - 右ボタン下げ
                  - ``WM_RBUTTONDOWN``
                * - btn
                  - 右ボタン上げ
                  - ``WM_RBUTTONUP``
                * - btn
                  - 中央ボタン下げ
                  - ``WM_MBUTTONDOWN``
                * - btn
                  - 中央ボタン上げ
                  - ``WM_MBUTTONUP``
                * - btn
                  - マウスホイール回転(縦)
                  - ``WM_MOUSEWHEEL``
                * - btn
                  - マウスホイール回転(横)
                  - ``WM_MOUSEHWHEEL``
                * - kbd
                  - キー下げ
                  - ``WM_KEYDOWN``
                * - kbd
                  - キー上げ
                  - ``WM_KEYUP``
                * - kbd
                  - 文字送信(1文字ずつ)
                  - ``WM_CHAR``

            .. admonition:: TOUCH非対応
                :class: caution

                | btn関数でTOUCH指定時のMORG_DIRECTは無視されMORG_CLIENTとして動作します

    :param 定数 省略可 画面取得: 画面取得方法を指定する

        .. object:: MORG_FORE

            | スクリーン上から画像を取得する (:any:`chkimg`)、または色を得る (:any:`peekcolor`)

        .. object:: MORG_BACK

            | 対象ウィンドウから直接画像の取得 (:any:`chkimg`)、または色の取得 (:any:`peekcolor`) を試みる
            | 他のウィンドウに隠れている場合でも使用可能

            .. admonition:: 動作しない場合
                :class: caution

                | 対象ウィンドウによっては正常に動作しない可能性があります
                | 例: saveimgのIMG_BACKで画像が保存できないウィンドウ

            .. admonition:: CHKIMG_USE_WGCAPI指定時
                :class: hint

                | chkimgでGraphicsCaptureAPI利用時にこれらのオプションは影響しません
                | ウィンドウの位置を問わずウィンドウ画像を取得します

    :param 真偽値またはEMPTY 省略可 HWND: ``MORG_DIRECT`` 指定時の第一引数の振る舞いを限定します (``MORG_DIRECT`` 以外の場合無視される)

        .. object:: FALSE

            | 第一引数をIDとしますが、有効なIDが登録されていない場合はその値をHWNDとして扱います

            .. admonition:: 例
                :class: hint

                | 30000 を指定

                - ID30000が登録済み→該当ウィンドウを対象とする
                - ID30000が未登録→HWNDが30000のウィンドウを対象とする


        .. object:: TRUE

            | 第一引数をHWNDとして扱います

    :rtype: 真偽値
    :return: 成功した場合TRUE、失敗時はFALSE

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            // MORG_DIRECTのHWND指定
            id = getid(hoge)
            hnd = getctlhnd(id, class_name)
            // このとき hnd の値がいずれかの登録済みIDと一致してしまった場合は予期せぬ動作となる
            mouseorg(hnd, MORG_DIRECT)

            // MORG_DIRECTかつ第四引数をTRUEにした場合hndはHWNDとして扱われる
            mouseorg(hnd, MORG_DIRECT, , TRUE)

.. function:: chkmorg()

    | mouseorgで基準点となっているスクリーン座標を得る

    :rtype: 数値配列またはEMPTY
    :return: 基準点が変更されている場合は [x, y]、変更されていない場合はEMPTY

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            mouseorg(id)
            print chkmorg() // [x, y]
            mouseorg(0)
            print chkmorg() // EMPTY


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

            ウィンドウが表示されているモニタ番号 (:any:`monitor` 関数に対応) (数値)

        .. object:: ST_WX

            ウィンドウの補正なしX座標

        .. object:: ST_WY

            ウィンドウの補正なしY座標

        .. object:: ST_WWIDTH

            ウィンドウの補正なし幅

        .. object:: ST_WHEIGHT

            ウィンドウの補正なし高さ

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

.. function:: monitor(モニタ番号, [MON定数=MON_ALL])

    | モニタの情報を得ます

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

        .. object:: MON_DPI

            画面のDPI

        .. object:: MON_SCALING

            スケーリング倍率 (%)

        .. object:: MON_ALL

            上記すべて (連想配列、キーはMON定数)

    :return:

        - 定数指定 (``MON_ALL`` 以外): 得られた値
        - ``MON_ALL`` 指定: 連想配列 (キーはMON定数)
        - 該当モニタなし: ``FALSE``

.. function:: monitor()
    :noindex:

    | (引数なし) モニタの数を得ます

    :return: モニタの数

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

.. function:: getslider(ID, [n番目=1, パラメータ=SLD_POS])

    | スライダー(スクロールバー、トラックバー)の値を取得します

    :param ウィンドウID ID: 対象ウィンドウのID
    :param 数値 省略可 n番目: n番目のスライダーから値を得る
    :param 定数 省略可 パラメータ: 取得する値の種類を示す定数

        .. object:: SLD_POS

            | 現在値

        .. object:: SLD_MIN

            | 最小値

        .. object:: SLD_MAX

            | 最大値

        .. object:: SLD_PAGE

            | 1ページ移動量

        .. object:: SLD_BAR

            | 表示方向 (横なら0、縦なら1を返す)

        .. object:: SLD_X

            | クライアントX座標

        .. object:: SLD_Y

            | クライアントY座標

    :rtype: 数値
    :return: 取得した値、該当するスライダーがない場合は ``-999999``

.. function:: chkbtn(ID, アイテム名, [n番目=1, ACC=FALSE])

    | ボタン(チェックボックス、ラジオボタン)やメニューのチェック状態を得る

    :param 数値 ID: 対象ウィンドウのID
    :param 文字列 アイテム名: ボタン名 (部分一致)
    :param 数値 省略可 n番目: n番目に該当するボタンの状態を得る

        .. admonition:: UWSCとは順序が異なる場合があります
            :class: caution

            | 実装の違いによりUWSCとは別の番号を指定しなければならない可能性があります
            | ご注意ください

    :param 真偽値 省略可 ACC:

        .. object:: FALSE

            | APIまたはUIAを使用

        .. object:: TRUE

            | ACCを使用

        .. admonition:: UWSCとの違い
            :class: note

            | TRUEでも対象ウィンドウをアクティブにしないため、2は廃止されました

    :rtype: 数値またはFALSE
    :return:

        - -1: 存在しない、または無効
        - 0: チェックされていない
        - 1: チェックされている
        - 2: チェックボックスが灰色 (ACCでは判定不可)
        - FALSE: ウィンドウが存在しない

.. function:: getstr(ID, [n番目=1, 種別=STR_EDIT, マウス移動=FALSE])

    | ウィンドウ上の文字列を取得します

    :param 数値 ID: 対象ウィンドウのID

        | 0の場合クリップボードから取得します (その場合以降の引数は無視されます)

        .. admonition:: クリップボードへのアクセスができない場合
            :class: caution

            | クリップボードアクセス時に何かしらのエラーが発生した場合はEMPTYを返します

    :param 数値 省略可 n番目: n番目に該当するアイテム種別の文字列を得る

        .. admonition:: UWSCとは順序が異なる場合があります
            :class: caution

            | 実装の違いによりUWSCとは別の番号を指定しなければならない可能性があります
            | ご注意ください

    :param 定数 省略可 種別: 文字列を取得するアイテム種別

        .. object:: STR_EDIT

            | エディットコントロール

        .. object:: STR_STATIC

            | スタティックコントロール

        .. object:: STR_STATUS

            | ステータスバー

        .. object:: STR_ACC_EDIT

            | エディットコントロール等 (ACCで取得)

        .. object:: STR_ACC_STATIC

            | スタティックコントロール (ACCで取得)

        .. object:: STR_ACC_CELL

            | DataGridView内のセルの値

    :param 真偽値 省略可 マウス移動: TRUEなら該当アイテムまでマウス移動
    :rtype: 文字列またはEMPTY
    :return: 取得した文字列、対象がない場合はEMPTY

.. function:: getitem(ID, 種別, [n番目=1, 列=1, ディセーブル無視=FALSE, ACC最大取得数=0])

    | ウィンドウ上の文字情報をアイテム種類別に取得する

    :param 数値 ID: 対象ウィンドウのID
    :param 定数 種別: 種類を示す定数、OR連結で複数指定可

        .. object:: ITM_BTN

            ボタン、チェックボックス、ラジオボタン

        .. object:: ITM_LIST

            リストボックス、コンボボックス

        .. object:: ITM_TAB

            タブコントロール

        .. object:: ITM_MENU

            メニュー

        .. object:: ITM_TREEVIEW (ITM_TREEVEW)

            ツリービュー

        .. object:: ITM_LISTVIEW (ITM_LSTVEW)

            リストビュー

        .. object:: ITM_EDIT

            エディットボックス

        .. object:: ITM_STATIC

            スタティックコントロール

        .. object:: ITM_STATUSBAR

            ステータスバー

        .. object:: ITM_TOOLBAR

            ツールバー

        .. object:: ITM_LINK

            リンク

        .. object:: ITM_ACCCLK

            ACCによりクリック可能なもの

        .. object:: ITM_ACCCLK2

            ACCによりクリック可能なもの、選択可能テキスト

        .. object:: ITM_ACCTXT

            ACCスタティックテキスト

        .. object:: ITM_ACCEDIT

            ACCエディット可能テキスト

        .. object:: ITM_FROMLAST

            ACCで検索順序を逆にする (最後のアイテムから取得)

        .. admonition:: UWSCとの違い
            :class: caution

            | ACCでもウィンドウをアクティブにしないため、ITM_BACKは廃止されました


    :param 数値 省略可 n番目: ITM_LIST、ITM_TREEVIEW、ITM_LISTVIEW指定時かつ対象が複数あった場合にいずれを取得するか指定、-1ならすべて取得

        .. admonition:: 複数種別同時指定時の処理について
            :class: hint

            | ITM_LIST、ITM_TREEVIEW、ITM_LISTVIEWのうち複数を同時に指定した場合、それぞれのn番目を検索します

            .. sourcecode:: uwscr

                // この場合リストまたはコンボボックスの2番目、及びツリービューの2番目をそれぞれ取得します
                getitem(id, ITM_LIST or ITM_TREEVIEW, 2)

        .. admonition:: UWSCとは順序が異なる場合があります
            :class: caution

            | 実装の違いによりUWSCとは別の番号を指定しなければならない可能性があります
            | ご注意ください

    :param 数値 省略可 列: ITM_LISTVIEW指定時にどの列から取得するかを指定(1から)、0ならすべての列、-1ならカラム名を取得
    :param 真偽値 省略可 ディセーブル無視: FALSEならディセーブル状態でも取得する、TRUEなら取得しない
    :param 数値 省略可 ACC最大取得数: ACC指定時に取得するアイテム数の上限を指定、0なら無制限、マイナス指定時は逆順(ITM_FROMLASTと同じ)
    :rtype: 文字列の配列
    :return: 取得されたアイテム名の配列

        .. admonition:: UWSCとの違い
            :class: caution

            | 戻り値が配列になったため ``ALL_ITEM_LIST`` は廃止されました

            .. sourcecode:: uwscr

                items = getitem(id, ITM_BTN)
                // 個数を得る
                print length(items)
                // アイテム名の表示
                for item in items
                    print item
                next

            | また、空の文字列は結果に含まれなくなりました

            .. sourcecode:: uwscr

                // UWSCでは空文字を1つ目のアイテムとして出力していましたが、UWSCRでは空文字はスキップされます
                i = 0
                for item in getitem(getid('ファイル名を指定して実行'), ITM_STATIC)
                    i += 1
                    print "<#i>: <#item>"
                next
                // 結果
                // 1: 実行するプログラム名、または開くフォルダーやドキュメント名、インターネット リソース名を入力してください。
                // 2: 名前(&O):

.. function:: getslctlst(ID, [n番目=1, 列=1])

    | 表示されているコンボボックス、リストボックス、ツリービュー、リストビューから選択されている項目を取得

    :param ID 数値: 対象ウィンドウのID
    :param 数値 省略可 n番目: n番目の該当コントロールから値を得る (1から)
    :param 数値 省略可 列: リストビューの場合取得する列を指定 (1から)
    :rtype: 文字列、または文字列の配列
    :return: 選択項目、複数選択されている場合は配列で返る

        .. admonition:: UWSCとの違い
            :class: caution

            | リストやリストビューが複数選択されていた場合にタブ連結された文字列ではなく、
            | それぞれの要素を持つ配列として返すようになりました

.. function:: chkclr(探索色, [閾値=0, 範囲=[], モニタ番号=0])

    | 範囲内に探索色があればその位置を返します
    | :any:`mouseorg` が実行されている場合は探索対象がそのウィンドウとなります

    :param 数値または配列 探索色: 探す色を指定します

        - 数値: BGR値
        - 配列: [B値, G値, R値]

    :param 数値または配列 省略可 閾値: 探索する色の幅を指定します

        - 数値: BGRそれぞれに対する閾値
        - 配列: 個別指定 [対B, 対G, 対R]

        .. admonition:: 閾値指定による色の幅について
            :class: hint

            | 探索色のB値が30でBに対する閾値が5の場合25～35であればヒットする
            | 255 ($FF) を指定すると元の値に関わらずその色要素に対して必ずヒットします

            .. sourcecode:: uwscr

                chkclr([0, 100, 0], [255, 5, 255])
                // 下限: [  0,  95, 255]
                // 上限: [255, 105, 255]
                // が探索色となるB要素とR要素はすべてを対象とし、Gのみ95-105を対象とする

    :param 配列 省略可 範囲: 探索範囲を [左上x, 左上y, 右下x, 右下y] で指定、省略時はモニタまたはウィンドウに準拠

        .. admonition:: 部分的な省略について
            :class: hint

            | 配列サイズが4より小さい場合、不足分は省略扱いとなります
            | ``null`` を記述することで省略であることを明示できます

            - [100] 左上xのみ指定、残りは省略
            - [100, 100] 左上xyを指定、右下xyは省略
            - [100, null, 100] 左上xと右下xを指定、左上yと右下yは省略

    :param 数値 省略可 モニタ番号: mouseorgを使わない場合に探索対象とするモニタ番号を0から指定
    :rtype: 二次元配列
    :return: 該当色のある座標および見つかった色([x, y, [b, g, r]])の配列

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            function bgr_array_to_int(arr: array)
                result = arr[0] * $10000 + arr[1] * $100 + arr[2]
            fend

            mouseorg(id)
            offset_x = status(id, ST_X)
            offset_y = status(id, ST_Y)
            color = [0, 100, 0]
            bgcolor = bgr_array_to_int(color)
            threshold = [0, 5, 0]
            // [0, 95, 0] から [0, 105, 0] を探索範囲とする
            for found in chkclr(color, threshold)
                x = found[0] + offset_x
                y = found[1] + offset_y
                color = found[2]
                msg = "座標: <#x>, <#y> 色: <#color>"
                balloon(msg, x, y, FUKI_DOWN or FUKI_POINT, , , 0, bgcolor)
                if msgbox("次へ") = BTN_CANCEL then
                    break
                endif
            next

    .. admonition:: 「Explorerが停止しているかもしれません、その場合Explorerを再起動してください」エラーについて
        :class: caution

        | Explorerが停止状態のため画面のキャプチャに失敗している可能性があります
        | このエラーが出力された場合はタスクマネージャでExplorerの状態を確認してください
        | もし停止していたら再起動してください

        1. ``Ctrl`` + ``Shift`` + ``Esc`` キーを同時に押し、タスクマネージャを起動します
        2. **エクスプローラー** を右クリックします
        3. メニューの ``再起動`` を押します

画像検索
--------

.. function:: chkimg(画像ファイルパス, [スコア=95, 最大検索数=5, left=EMPTY, top=EMPTY, right=EMPTY, bottom=EMPTY, オプション=0, モニタ番号=0])

    | 指定画像をスクリーン上から探してその座標を返します

    .. admonition:: UWSCとは互換性がありません
        :class: caution

        - 特殊変数 ``G_IMG_X``, ``G_IMG_Y``, ``ALL_IMG_X``, ``ALL_IMG_Y`` は廃止
        - 戻り値が変更されています

    :param 文字列 画像ファイルパス: 検索する画像のパス (jpg, bmp, png)
    :param 数値 省略可 スコア: 画像に対する一致率を指定 (80.0-100.0)

        | 一致率が指定値以上であれば結果を返します
        | 小数も有効です (例: 99.75)

        .. admonition:: 生スコア値指定
            :class: note

            | スコアは実際の処理では0.0から1.0の範囲の値として扱われます
            | 例: 95 → 0.95
            | スコア値を0.0から1.0の範囲で指定した場合はそのままの値が使われます
            | この場合はスコアの下限がないため80未満を指定することも可能です
            | 例: 0.75 (スコア75相当)

    :param 数値 省略可 最大検索数: 検索の試行回数を指定
    :param 数値 省略可 left: 検索範囲指定: 左上X座標、省略時は画面左上X座標
    :param 数値 省略可 top: 検索範囲指定: 左上Y座標、省略時は画面左上Y座標
    :param 数値 省略可 right: 検索範囲指定: 右下X座標、省略時は画面右下X座標
    :param 数値 省略可 bottom: 検索範囲指定: 右下X座標、省略時は画面右下Y座標
    :param 定数 省略可 オプション: 実行時オプションを指定、OR連結可

        .. object:: CHKIMG_NO_GRAY

            | 画像をグレースケール化せず探索を行う

        .. object:: CHKIMG_USE_WGCAPI

            | デスクトップまたはウィンドウの画像取得にGraphicsCaptureAPIを使う
            | デスクトップの場合は対象とするモニタを次の引数で指定
            | mouseorgを利用している場合はウィンドウを対象とする
            | このオプションを指定した場合mouseorgの ``MOUSE_FORE`` および ``MOUSE_BACK`` は無視されます (指定に関わらずフォア・バックをキャプチャ可能)

            .. hint:: このオプションにより通常ではキャプチャできないウィンドウがキャプチャできる可能性があります

            .. admonition:: キャプチャできないウィンドウ状態について
                :class: note

                | 対象ウィンドウが最小化されている、または非表示になっている場合はキャプチャを行わず関数を終了します
                | このオプションでウィンドウをキャプチャする場合は対象ウィンドウが表示状態になっていることを確認してください

        .. object:: CHKIMG_METHOD_SQDIFF

            | 類似度の計算にTM_SQDIFFを使用する、他の計算方法と併用不可

        .. object:: CHKIMG_METHOD_SQDIFF_NORMED

            | 類似度の計算にTM_SQDIFF_NORMEDを使用する、他の計算方法と併用不可

        .. object:: CHKIMG_METHOD_CCORR

            | 類似度の計算にTM_CCORRを使用する、他の計算方法と併用不可

        .. object:: CHKIMG_METHOD_CCORR_NORMED

            | 類似度の計算にTM_CCORR_NORMEDを使用する、他の計算方法と併用不可

        .. object:: CHKIMG_METHOD_CCOEFF

            | 類似度の計算にTM_CCOEFFを使用する、他の計算方法と併用不可

        .. object:: CHKIMG_METHOD_CCOEFF_NORMED

            | 類似度の計算にTM_CCOEFF_NORMEDを使用する、他の計算方法と併用不可
            | 計算方法未指定時はこれが適用される


    :param 定数 省略可 モニタ番号:

        | ``CHKIMG_USE_WGCAPI`` 時に検索するモニタ番号を0から指定、デフォルトは0 (プライマリモニタ)
        | mousemorg使用時はウィンドウを対象とするためこの引数指定は不要

    :rtype: 二次元配列
    :return: 該当する部分の座標とスコアを格納した二次元配列 ``[[X座標, Y座標, スコア], ...]``

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            for found in chkimg("hoge.png")
                print found // [x, y, スコア]
            next

.. function:: chkimg(画像ファイルパス, [スコア=95, 最大検索数=5, 範囲, オプション=0])
    :noindex:

    | 配列による範囲指定

    :param 配列 省略可 範囲: ``[left, top, right, bottom]`` で指定

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            found = chkimg("hoge.png", 95, 1, [100, 100, 400, 400])

.. function:: saveimg([ファイル名=EMPTY, ID=0, x=EMPTY, y=EMPTY, 幅=EMPTY, 高さ=EMPTY, クライアント領域=FALSE, 圧縮率=EMPTY, 取得方法=IMG_AUTO, WGCAPI=false, モニタ番号=0])

    | ウィンドウの画像を保存します

    :param 文字列 省略可 ファイル名: 保存するファイル名 (対応する拡張子は ``jpg``, ``bmp``, ``png``)、EMPTYの場合はクリップボードにコピー

        .. admonition:: 拡張子が有効ではない場合
            :class: hint

            | pngファイルとして保存されます

            .. sourcecode:: uwscr

                saveimg("hoge") // hoge.pngが保存される


    :param 数値 省略可 ID: ウィンドウID、0の場合スクリーン全体
    :param 数値 省略可 x: 取得範囲の起点となるx座標、EMPTYの場合は左上
    :param 数値 省略可 y: 取得範囲の起点となるy座標、EMPTYの場合は左上
    :param 数値 省略可 幅: 取得範囲の幅、EMPTYの場合は ``ウィンドウ幅 - x``
    :param 数値 省略可 高さ: 取得範囲の高さ、EMPTYの場合は ``ウィンドウ高さ - y``
    :param 真偽値 省略可 クライアント領域: FALSEならウィンドウ全体、TRUEならクライアント領域のみ
    :param 数値 省略可 圧縮率:

        | 指定したファイル拡張子により指定値が異なります
        | ファイル名を省略した(クリップボードにコピーされる)場合この値は無視されます

        .. object:: jpg

            | JPEG画像の画質を0-100で指定します (高いほど高画質)
            | EMPTY指定時、または値が範囲外の場合は95になります

        .. object:: png

            | PNG画像の圧縮度合いを0-9で指定します (高いほどサイズが小さくなるが、遅くなる)
            | EMPTY指定時、または値が範囲外の場合は1になります

        .. object:: bmp

            この値は無視されます

        .. admonition:: UWSCとの違い
            :class: caution

            | UWSCでは1-100指定ならJPEG、0ならBMPで保存されていましたが、UWSCRではファイル名の拡張子で保存形式を指定します

    :param 定数 省略可 取得方法: 画面の取得方法

        .. object:: IMG_FORE

            スクリーン全体から対象ウィンドウの座標を元に画像を切り出す

        .. object:: IMG_BACK

            対象ウィンドウから画像を取得

            .. caution:: 他のウィンドウに隠れていても取得可能ですが、見た目が完全に一致しない場合があります

        .. object:: IMG_AUTO (0)

            ウィンドウ全体が可視かどうかで取得方法を自動的に切り替えます

            - ウィンドウが見えていれば ``IMG_FORE`` を使用する (アクティブかどうかは問わない)
            - 一部でも他のウィンドウに隠れていれば ``IMG_BACK`` を使用する
    :param 真偽値 省略可 WGCAPI: TRUEならGraphicsCaptureAPIにより画面またはウィンドウをキャプチャします
    :param 数値 省略可 モニタ番号: IDに0を指定して、かつWGCAPIをTRUEにした場合にキャプチャするモニタ番号を0から指定

        .. admonition:: xy座標は0から
            :class: note

            | xy座標はモニタごとの座標を0から指定してください
            | 0未満が指定された場合は0になります

    :return: なし


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

            | タッチ操作を行う
            | 状態をCLICKにした場合指定座標をタッチして離す
            | 状態をDOWNにした場合指定座標でタッチ
            | その後状態をUPで再実行した場合同一座標ならそのまま離し、座標が異なるならその座標までスワイプ操作を行う
            | msを指定した場合はスワイプ速度に影響する (移動区間の一区切り毎の移動速度を変更する)

            .. important:: タッチできるのは一点のみ (複数箇所タッチは不可)

    :param 定数 省略可 状態: マウスボタンに対してどのような操作を行うかを指定

        - ``LEFT``, ``RIGHT``, ``MIDDLE`` の場合以下のいずれかを指定

            .. object:: CLICK

                ボタンクリック (デフォルト)

            .. object:: DOWN

                ボタン押し下げ

            .. object:: UP

                ボタン開放

        - ``WHEEL``: ノッチ数を指定 (正なら上方向、負なら下方向に回転)
        - ``WHEEL2``: ノッチ数を指定 (正なら右方向、負なら左方向に回転)

    :param 数値 省略可 x: ボタン操作を行う位置のX座標、省略時は現在のマウスのX座標
    :param 数値 省略可 y: ボタン操作を行う位置のY座標、省略時は現在のマウスのY座標
    :param 数値 省略可 ms:

        | ボタン操作を行うまでの待機時間 (ミリ秒)
        | またはTOUCHのDOWN後のUPで別座標を指定した場合のスワイプ速度、0 (速)～10 (遅)

    :return: なし

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            btn(TOUCH, DOWN, 100, 100)
            btn(TOUCH, UP, 200, 200) // 別座標でUPした場合はスワイプ操作になる

            btn(TOUCH, DOWN, 150, 150)
            btn(TOUCH, UP, 250, 250, 0) // msが0なら最速

            btn(TOUCH, DOWN, 300, 300)
            btn(TOUCH, UP, 150, 150, 10) // 10ならとても遅い

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
