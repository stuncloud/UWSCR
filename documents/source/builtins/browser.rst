ブラウザ操作
============

ブラウザ操作
------------

.. function:: BrowserControl(ブラウザ定数, [プロファイル=EMPTY, ポート=9222, ヘッドレス=FALSE])

    | Devtools Protocolを利用したブラウザ操作を行うためのBrowserオブジェクトを返します
    | デバッグポート9222 (デフォルト、変更化) でブラウザを起動します
    | 対応ブラウザは

        - Google Chrome
        - Microsoft Edge

    | 関数実行時にブラウザを起動します
    | 指定ポートが開かれているブラウザが既に起動している場合は再接続します
    | 指定ポートが開かれていないブラウザが既に起動している場合はエラーになります (**ブラウザを閉じて再実行してください**)

    :param 定数 ブラウザ定数: 以下のいずれかを指定

        .. object:: BC_CHROME

            Google Chromeを操作します

        .. object:: BC_MSEDGE

            Microsoft Edgeを操作します

    :param 文字列 省略可 プロファイル: プロファイル保存先フォルダを指定、省略時はデフォルトプロファイルを使用します

        .. admonition:: ブラウザ起動中に自動化用のブラウザを別途起ち上げる
            :class: hint

            | プロファイルフォルダを指定することで現在実行中のブラウザとは別のブラウザを起動できます
            | 自動化用のプロファイルを保存するフォルダを別途指定してください
            | フォルダが存在しない場合は自動で作成されます
            | 次回以降も同一フォルダを指定することで自動化用プロファイルとして利用できます

    :param 数値 省略可 ポート: デバッグポートを変更します
    :param 真偽値 省略可 ヘッドレス: TRUEにした場合はブラウザを非表示(ヘッドレス)で起動します、再接続時は無視されます

    :rtype: :ref:`browser_object`
    :return: 対象ブラウザの :ref:`browser_object`

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

.. function:: ConvertFromRemoteObject(remote)

    | リモートオブジェクトがプリミティブな値の場合に適切な値型に変換します
    | 変換できないものはそのまま返ります

    :param RemoteObject remote: 値型に変換したい :ref:`remote_object`
    :return: 変換された値、変換できない場合は :ref:`remote_object`

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