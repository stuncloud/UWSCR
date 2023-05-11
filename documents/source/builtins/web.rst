ウェブ関連
==========

ブラウザ操作
------------

.. admonition:: 破壊的変更が行われました
    :class: warning

    | バージョン `0.11.0` 以降のブラウザ操作機能はバージョン `0.10.2` 以前とは互換性がありません

.. function:: BrowserControl(ブラウザ定数, [ポート=9222])

    | Devtools Protocolを利用したブラウザ操作を行うための :ref:`browser_object` を返します
    | デバッグポートを開いたブラウザを起動します
    | 対応ブラウザは以下

        - Google Chrome
        - Microsoft Edge

    :param 定数 ブラウザ定数: 以下のいずれかを指定

        .. object:: BC_CHROME

            Google Chromeを操作します

        .. object:: BC_MSEDGE

            Microsoft Edgeを操作します

    :param 数値 省略可 ポート: デバッグポートを指定する
    :rtype: :ref:`browser_object`
    :return: 対象ブラウザの :ref:`browser_object`

    .. admonition:: ブラウザへの再接続について
        :class: hint

        | 対象ブラウザが同じデバッグポートを開けて起動している場合はそのブラウザに再接続できます
        | 異なるポートを開いている、またはポートが開かれていない場合は再接続できずエラーになります

    .. admonition:: 起動中のブラウザとは別に自動操作用のブラウザを起ち上げるには
        :class: hint

        | 起動中のブラウザとは異なるプロファイルで新たなブラウザを起動する必要があります
        | このような場合はBrowserControl関数ではなく :any:`Browserbuilder` 関数 を使用してください
        | :any:`Browserbuilder` 関数が返す :ref:`builder_object` でプロファイルフォルダを指定します

.. function:: Browserbuilder(ブラウザ定数)

    | :ref:`builder_object` を返します
    | 最低限の設定でブラウザを起動する :any:`BrowserControl` 関数とは異なり :ref:`builder_object` を介して様々な設定が行なえます

    :param 定数 ブラウザ定数: 以下のいずれかを指定

        .. object:: BC_CHROME

            Google Chromeを操作します

        .. object:: BC_MSEDGE

            Microsoft Edgeを操作します

    :rtype: :ref:`builder_object`
    :return: 対象ブラウザの :ref:`builder_object`


    .. admonition:: ブラウザの起動方法
        :class: hint

        | :ref:`builder_object` の ``start()`` メソッドでブラウザを起動、または再接続します

        .. sourcecode:: uwscr

            // BrowserBuilderオブジェクトを作成し、startメソッドを呼ぶ
            builder = BrowserBuilder(BC_CHROME)
            chrome = builder.start()

            // 以下のようにも書ける
            chrome = BrowserBuilder(BC_CHROME).start()

            // ポートの変更
            chrome = BrowserBuilder(BC_CHROME)_
                .port(9999)_
                .start()

            // ヘッドレス起動
            chrome = BrowserBuilder(BC_CHROME)_
                .headless(TRUE)_
                .start()

            // プロファイルフォルダの変更
            chrome = BrowserBuilder(BC_CHROME)_
                .profile("C:\uwscr\chrome\profile1")_
                .start()

            // 複合設定
            chrome = BrowserBuilder(BC_CHROME)_
                .port(12345)_
                .headless(TRUE)_
                .start()

    .. admonition:: 対象ブラウザが指定ポートを開いていなかった場合の動作
        :class: hint

        - 対象ブラウザのプロセスがすでに存在している
        - そのプロセスが指定ポートを開いていない

        | の2点を満たす場合、再接続が行えないためエラーになります
        | ただし、起動中のブラウザとは異なるプロファイルフォルダを指定した場合は指定ポートで新たなブラウザプロセスを起動します
        | (同一プロファイルにつき一つのデバッグポート(またはポートなし)でしかブラウザを起動できないため)

.. function:: ConvertFromRemoteObject(remote)

    | リモートオブジェクトがプリミティブな値の場合に適切な値型に変換します
    | 変換できないものはそのまま返ります

    :param RemoteObject remote: 値型に変換したい :ref:`remote_object`
    :return: 変換された値、変換できない場合は :ref:`remote_object`

    .. admonition:: ブラウザパスの指定方法
        :class: tip

        | 通常はレジストリ等からブラウザの実行ファイルのパスを取得しそれを実行します (パスの自動取得)
        | 自動取得を行わずに任意のパスで実行させるには設定ファイルにパスを記述します

        .. code:: json

            {
                "browser": {
                    "chrome": "C:\\path\\to\\chrome.exe",
                    "msedge": "C:\\path\\to\\msedge.exe"
                },
            }

        | 自動取得に戻す場合は ``null`` にします

        .. code:: json

            {
                "browser": {
                    "chrome": null,
                    "msedge": null
                },
            }

        | パスは必ずchrome.exeおよびmsedge.exeのものにしてください
        | それ以外は動作保証外です

.. function:: RemoteObjectType(remote)

    | :ref:`remote_object` の型を返します
    | 型名の他に可能であれば以下を含みます

    - 型の詳細
    - クラス名

    :param RemoteObject remote: 型情報を得たい :ref:`remote_object`
    :rtype: 文字列
    :return: 型の情報を示す文字列

.. _builder_object:

BrowserBuilderオブジェクト
~~~~~~~~~~~~~~~~~~~~~~~~~~

| ブラウザの起動、再接続、起動時設定を行うオブジェクト

.. class:: BrowserBuilder

    .. method:: port(port)

        | ブラウザのデバッグポートを変更します、デフォルトは ``9222``

        :param 数値 port: 変更するデバッグポート
        :rtype: BrowserBuilder
        :return: 更新されたBrowserBuilder

    .. method:: headless(有効=TRUE)

        | ブラウザをヘッドレスで起動するかどうかを設定します
        | この設定は再接続時には無視されます

        :param 真偽値 有効: TRUEの場合ブラウザをヘッドレスで起動
        :rtype: BrowserBuilder
        :return: 更新されたBrowserBuilder

    .. method:: private(有効=TRUE)

        | ブラウザをプライベートモードで起動するかどうかを設定します
        | この設定は再接続時には無視されます

        :param 真偽値 有効: TRUEの場合ブラウザをプライベートモードで起動
        :rtype: BrowserBuilder
        :return: 更新されたBrowserBuilder

    .. method:: profile(プロファイルパス)

        | プロファイルを保存するパスを指定します
        | この設定は再接続時には無視されます

        :param 文字列 プロファイルパス: プロファイルを保存するパス
        :rtype: BrowserBuilder
        :return: 更新されたBrowserBuilder

    .. method:: start()

        | ブラウザを起動し :ref:`browser_object` を返します

        :rtype: :ref:`browser_object`
        :return: 対象ブラウザの :ref:`browser_object`

.. _browser_object:

Browserオブジェクト
~~~~~~~~~~~~~~~~~~~

| 操作対象となるタブを示すオブジェクト

.. class:: Browser

    .. property:: count

        ブラウザ上の操作可能なタブの数を返します

    .. property:: tabs[i]

        インデックスを指定し :ref:`tabwindow_object` を返します

        .. admonition:: 配列表記対応
            :class: hint

            | Browserオブジェクトに直接インデックス指定することもできます

            .. sourcecode:: uwscr

                chrome = BrowserControl(BC_CHROME)

                // タブの取得
                tab = chrome.tabs[0]

                // 以下のようにも書ける
                tab = chrome[0]

    .. method:: close()

        | ブラウザを閉じます

        :return: なし

    .. method:: new(url)

        | 指定したURLを新しいタブを開きます

        :param 文字列 url: 開きたいサイトのURL
        :rtype: :ref:`tabwindow_object`
        :return: 新しく開いたタブの :ref:`tabwindow_object`

    .. method:: id()

        | ブラウザのウィンドウIDを返します

        :rtype: 数値
        :return: ウィンドウID

.. admonition:: タブ一覧取得が遅い場合がある
    :class: caution

    | countやtabsの結果を得るまでに数秒かかる場合があります
    | これは、使用しているDevtools ProtocolのAPI実行速度によるものです

.. _tabwindow_object:

TabWindowオブジェクト
~~~~~~~~~~~~~~~~~~~~~

| タブごとのWindowオブジェクトを示すオブジェクト

.. class:: TabWindow

    .. property:: document

        ``window.document`` に相当する :ref:`remote_object` を返します

        .. admonition:: ブラウザ操作の基本はdocument取得から
            :class: hint

            | :ref:`remote_object` はブラウザ上のJavaScriptオブジェクトです
            | ``document`` を起点に ``querySelector`` 等でエレメントにアクセスできます
            | :ref:`remote_object` のプロパティやメソッドの実行結果は :ref:`remote_object` として返ります
            | そのためブラウザ上でJavaScriptを実行するかのようにブラウザ操作を行うことが可能です
            | 詳しくは :ref:`browser_sample` を参照してください

    .. method:: navigate(uri)

        | 指定URLを開きます
        | ページの読み込み完了まで待機します (最大10秒)

        .. admonition:: 読み込み時間が長い場合
            :class: hint

            | 読み込みに10秒以上かかるページに対しては navigate実行後に :any:`wait` メソッドを呼んでください

        :param 文字列 uri: 開きたいサイトのURL
        :rtype: 真偽値
        :return: タイムアウトした場合FALSE

    .. method:: reload([キャッシュ無視=FALSE])

        | ページをリロードします
        | ページの読み込み完了まで待機します (最大10秒)

        .. admonition:: 読み込み時間が長い場合
            :class: hint

            | 読み込みに10秒以上かかるページに対しては navigate実行後に :any:`wait` メソッドを呼んでください

        :param 真偽値 キャッシュ無視: TRUEならキャッシュを無視してリロード (`Shift+refresh` と同等)
        :rtype: 真偽値
        :return: タイムアウトした場合FALSE

    .. method:: wait([タイムアウト秒=10])

        | ページの読み込みが完了するのを待ちます
        | リンクをクリックした後などに使用します

        :param 数値 省略可 タイムアウト秒: 読み込み完了まで待機する最大時間 (秒)
        :rtype: 真偽値
        :return: タイムアウトした場合はFALSE

    .. method:: activate()

        | タブをアクティブにします

        :return: なし

    .. method:: close()

        | タブを閉じます

        :return: なし

    .. method:: dialog([許可=TRUE, プロンプト=EMPTY])

        | JavaScriptダイアログ(alert, confirm, prompt)を処理します

        :param 真偽値 省略可 許可: ダイアログを閉じる方法を指定、TRUEでOK、FALSEでキャンセル
        :param 文字列 省略可 プロンプト: promptに入力する文字列
        :return: なし

    .. method:: leftClick(x, y)
    .. method:: rightClick(x, y)
    .. method:: middleClick(x, y)

        | マウスクリックイベントを発生させます
        | それぞれ左クリック、右クリック、中央クリックを行います

        :param 数値 x: ブラウザのビューポート上のX座標 (CSSピクセル単位、左上から)
        :param 数値 y: ブラウザのビューポート上のY座標 (CSSピクセル単位、左上から)
        :return: なし

        .. admonition:: サンプルコード

            .. sourcecode:: uwscr

                // エレメントの取得
                element = browser[0].document.querySelector(selector)
                // getBoundingClientRectメソッドでエレメントの座標等の情報を得る
                rect = element.getBoundingClientRect()
                // 座標を指定し右クリックする
                tab.rightClick(rect.x + 10, rect.y + 10)



.. _remote_object:

RemoteObject
~~~~~~~~~~~~

| ブラウザ上に存在するJavaScriptオブジェクトを示すオブジェクト

メソッドの実行
^^^^^^^^^^^^^^

| ``RemoteObject.メソッド名(引数)`` でメソッドを実行し、戻り値を :ref:`remote_object` として取得します
| メソッド名は大文字小文字を区別します

.. sourcecode:: uwscr

    chrome = BrowserControl(BC_CHROME)
    foo = chrome[0].document.querySelector("#foo")

プロパティの取得
^^^^^^^^^^^^^^^^

| ``RemoteObject.プロパティ名`` とすることでプロパティ値を :ref:`remote_object` として取得します
| 配列要素であればインデックスを指定します ``RemoteObject.プロパティ名[i]``
| プロパティ名は大文字小文字を区別します

.. sourcecode:: uwscr

    chrome = BrowserControl(BC_CHROME)
    url = chrome[0].document.URL

プロパティの変更
^^^^^^^^^^^^^^^^

| ``RemoteObject.プロパティ名 = 値`` とすることでプロパティ値を変更します
| 配列要素であればインデックスを指定します ``RemoteObject.プロパティ名[i] = 値``
| プロパティ名は大文字小文字を区別します

.. sourcecode:: uwscr

    chrome = BrowserControl(BC_CHROME)
    foo = chrome[0].document.querySelector("#foo")
    foo.value = "ほげほげ"

関数として実行
^^^^^^^^^^^^^^

| :ref:`remote_object` 自身が関数である場合は ``RemoteObject(引数)`` として実行できます
| この場合も戻り値を :ref:`remote_object` として取得します

非同期関数とPromise
^^^^^^^^^^^^^^^^^^^

| :ref:`remote_object` 自身、またはそのメソッドが非同期関数であった場合 :ref:`await` 構文でその終了を待ちます
| :ref:`remote_object` がPromiseであった場合は :any:`WaitTask` 関数でその終了を待ちます
| いずれの場合も戻り値を :ref:`remote_object` として取得します

.. 他の値型との演算
.. ^^^^^^^^^^^^^^^^

.. | RemoteObjectがプリミティブな値であれば演算を行い、適した値型として値を返します

.. _browser_sample:

ブラウザ操作サンプル
~~~~~~~~~~~~~~~~~~~~

.. admonition:: documentへのアクセス

    .. sourcecode:: uwscr

        // ブラウザを開く
        chrome = BrowserControl(BC_CHROME)

        // ひとつめのタブを得る
        tab1 = chrome.tabs[0]
        // 以下のようにも書けます
        // tab1 = chrome[0]

        // 任意のサイトを開く
        tab1.navigate(url)

        // window.documentを得る
        document = tab1.document

        // URLを得る
        print document.URL

.. admonition:: タブごとのURLを列挙

    .. sourcecode:: uwscr

        // タブの数を得る
        print chrome.count

        // URLを列挙
        for tab in chrome.tabs
            print tab.document.URL
        next
        // 以下のようにも書けます
        // for tab in chrome
        //     print tab.document.URL
        // next

.. admonition:: 自動操作用ブラウザを別途開く

    .. sourcecode:: uwscr

        // デバッグポートを開いていないブラウザがすでに開かれている場合
        // 以下は再接続ができずエラーになる
        // chrome = BrowserControl(BC_CHROME)

        // プロファイルフォルダを指定して別のブラウザを起動する
        chrome = BrowserBuilder(BC_CHROME).profile("C:\chrome\profile1").start()

.. admonition:: Seleniumテストページの操作

    .. sourcecode:: uwscr

        // ブラウザを開く
        chrome = BrowserControl(BC_CHROME)
        // ブラウザをアクティブにする
        ctrlwin(chrome.id(), ACTIVATE)

        // 新しいタブでSeleniumのテストページを開く

        tab = chrome.new('http://example.selenium.jp/reserveApp_Renewal/')
        // ドキュメントを取得しておく
        document = tab.document

        // 宿泊日を入力

        // 3日後の日付を得る
        date = format(gettime(3, , G_OFFSET_DAYS), '%Y/%m/%d')

        document.querySelector('#datePick').value = date
        document.querySelector('#reserve_year').value = G_TIME_YY4
        document.querySelector('#reserve_month').value = G_TIME_MM2
        document.querySelector('#reserve_day').value = G_TIME_DD2

        // 宿泊日数を選択

        reserve_term = 2
        document.querySelector("#reserve_term option[value='<#reserve_term>']").selected = TRUE

        // 人数を選択

        headcount = 5
        document.querySelector("#headcount option[value='<#headcount>']").selected = TRUE

        // プラン選択

        // お得な観光プランをチェック
        document.querySelector('#plan_b').checked = TRUE


        // 名前入力

        document.querySelector('#guestname').value = "おなまえ"

        // 利用規約に同意して次へ をクリック

        document.querySelector('#agree_and_goto_next').click()

        // 読み込み完了を待つ

        tab.wait()
        // ページを移動したのでdocumentは取得しなおす
        document = tab.document

        // 合計金額を得る

        price = document.querySelector('#price').textContent
        // RemoteObjectを値に変換する
        price = ConvertFromRemoteObject(price)

        // 確定ボタンを押す

        document.querySelector('#commit').click()

        msgbox("宿泊費用は<#price>円でした")

        // タブを閉じる
        tab.close()