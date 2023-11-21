ウェブ関連
==========

ブラウザ操作
------------

.. admonition:: 破壊的変更が行われました
    :class: warning

    | バージョン `0.11.0` 以降のブラウザ操作機能はバージョン `0.10.2` 以前とは互換性がありません

.. admonition:: ブラウザパスの指定方法
    :class: hint

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

        .. sourcecode:: uwscr

            // 起動.uws

            chrome = BrowserControl(BC_CHROME, 9999) // ポート9999でChromeを起動
            chrome[0].navigate("https://example.com") // 0番目のタブで任意のサイトを開く

        .. sourcecode:: uwscr

            // 再接続.uws

            chrome = BrowserControl(BC_CHROME, 9999) // 9999ポートのChromeに再接続される
            url = chrome[0].document.URL // 0番目のタブのURLを取得
            print url // https://example.com

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

.. function:: RemoteObjectType(remote)

    | :ref:`remote_object` の型を返します
    | 型名の他に可能であれば以下を含みます

    - 型の詳細
    - クラス名

    :param RemoteObject remote: 型情報を得たい :ref:`remote_object`
    :rtype: 文字列
    :return: 型の情報を示す文字列

IE関数互換
~~~~~~~~~~

IEGETDATA互換
^^^^^^^^^^^^^

.. function:: BRGetData(タブ, name, [value=EMPTY, n番目=1])

    | エレメントのnameとvalue属性をもとに値を取得する

    :param TabWindowオブジェクト タブ: 値を取りたいページのタブを示す :ref:`tabwindow_object`
    :param 文字列 name: 値を取得するエレメントのname属性
    :param 文字列 省略可 value: nameが同一の場合にvalue属性の値を指定
    :param 数値 省略可 n番目: nameもvalueも一致する場合順番を1から指定
    :return: 取得された値、取得できない場合はEMPTY

.. function:: BRGetData(タブ, タグ指定, [n番目=1])
    :noindex:

    | エレメントのタグ名と順番を指定して値を取得する

    :param TabWindowオブジェクト タブ: 値を取りたいページのタブを示す :ref:`tabwindow_object`
    :param 文字列 タグ指定: "TAG=タグ名" でタグ指定モードになる
    :param 数値 省略可 n番目: 該当タグの順番を1から指定
    :return: 取得された値、取得できない場合はEMPTY

.. function:: BRGetData(タブ, タグ指定, プロパティ指定, [n番目=1])
    :noindex:

    | エレメントのタグ名とプロパティを指定して値を取得する

    :param TabWindowオブジェクト タブ: 値を取りたいページのタブを示す :ref:`tabwindow_object`
    :param 文字列 タグ指定: "TAG=タグ名" でタグ指定モードになる
    :param 文字列 省略可 プロパティ指定: "プロパティ名=値" を指定可("id=hoge" など)、プロパティ名のみ大文字小文字の一致が必須
    :param 数値 省略可 n番目: タグもプロパティも一致する場合順番を1から指定
    :return: 取得された値、取得できない場合はEMPTY

    .. admonition:: プロパティ指定について
        :class: note

        | UWSCとは異なりID, className, innerText, innerHTML以外のプロパティも指定できます
        | ただし、プロパティ名は大文字小文字が一致する必要があります(case sensitive)
        | プロパティの値は大文字小文字を無視しますが、完全一致する必要があります

.. function:: BRGetData(タブ, "TAG=TABLE", [n番目=1, 行=1, 列=1])
    :noindex:

    | テーブルエレメントの座標を指定して値を取得する

    :param TabWindowオブジェクト タブ: 値を取りたいページのタブを示す :ref:`tabwindow_object`
    :param 文字列 "TAG=TABLE": "TAG=TABLE" を指定(固定)
    :param 数値 省略可 n番目: テーブルの順番を1から指定
    :param 数値 省略可 行: テーブルの行番号を1から指定
    :param 数値 省略可 列: テーブルの列番号を1から指定
    :return: 取得された値、取得できない場合はEMPTY

IESETDATA互換
^^^^^^^^^^^^^

.. function:: BRSetData(タブ, 値, name, [value=EMPTY, n番目=1, 直接入力=FALSE])

    | テキストボックス等に文字列を入力する
    | キー入力をエミュレートします
    | ``input[type="file"]`` 要素に対してはファイルパスを設定します

    :param TabWindowオブジェクト タブ: 値を取りたいページのタブを示す :ref:`tabwindow_object`
    :param 文字列 値: 入力したい値、ファイルパス複数登録の場合は文字列配列も可
    :param 文字列 name: 値を変更するエレメントのname属性
    :param 文字列 省略可 value: 同一nameのエレメントがある場合にvalue値を指定
    :param 数値 省略可 n番目: nameとvalueが一致する場合に順番を1から指定
    :param 真偽値 省略可 直接入力: 直接valueプロパティを変更する場合はTRUE
    :rtype: 真偽値
    :return: 成功時TRUE

.. function:: BRSetData(RemoteObject, 値)

    | テキストボックス等に文字列を入力する
    | キー入力をエミュレートします
    | ``input[type="file"]`` 要素に対してはファイルパスを設定します

    :param RemoteObject タブ: 入力したいエレメントを示す :ref:`remote_object`
    :param 文字列 値: 入力したい値、ファイルパス複数登録の場合は文字列配列も可
    :rtype: 真偽値
    :return: 成功時TRUE

    .. sourcecode:: uwscr

        browser = BrowserControl(BC_CHROME)
        tab = browser[0]

        file = tab.querySelector("input[type=file]")
        files = [C:\test\hoge.txt, C:\test\fuga.txt]
        print BRSetData(file, files)

.. function:: BRSetData(タブ, TRUE, name, [value=EMPTY, n番目=1])
    :noindex:

    | nameにより指定したエレメントをクリックします

    :param TabWindowオブジェクト タブ: 値を取りたいページのタブを示す :ref:`tabwindow_object`
    :param 真偽値 TRUE: TRUEを指定 (固定)
    :param 文字列 name: クリックするエレメントのname属性
    :param 文字列 省略可 value: 同一nameのエレメントがある場合にvalue値を指定
    :param 数値 省略可 n番目: nameとvalueが一致する場合に順番を1から指定
    :rtype: 真偽値
    :return: 成功時TRUE

.. function:: BRSetData(タブ, TRUE, タグ指定, [n番目=1])
    :noindex:

    | タグ名と順番により指定したエレメントをクリックします

    :param TabWindowオブジェクト タブ: 値を取りたいページのタブを示す :ref:`tabwindow_object`
    :param 真偽値 TRUE: TRUEを指定 (固定)
    :param 文字列 タグ指定: "TAG=タグ名" でダグ指定モードになる
    :param 数値 省略可 n番目: タグ名が一致する場合に順番を1から指定
    :rtype: 真偽値
    :return: 成功時TRUE

.. function:: BRSetData(タブ, TRUE, タグ指定, プロパティ指定, [n番目=1])
    :noindex:

    | タグ名とプロパティにより指定したエレメントをクリックします

    :param TabWindowオブジェクト タブ: 値を取りたいページのタブを示す :ref:`tabwindow_object`
    :param 真偽値 TRUE: TRUEを指定 (固定)
    :param 文字列 タグ指定: "TAG=タグ名" でダグ指定モードになる
    :param 文字列 プロパティ指定: "プロパティ名=値" を指定
    :param 数値 省略可 n番目: タグ名とプロパティが一致する場合に順番を1から指定
    :rtype: 真偽値
    :return: 成功時TRUE

    .. admonition:: プロパティ指定について
        :class: note

        | UWSCとは異なりID, className, innerText, innerHTML以外のプロパティも指定できます
        | ただし、プロパティ名は大文字小文字が一致する必要があります(case sensitive)
        | プロパティの値は大文字小文字を無視しますが、完全一致する必要があります

.. function:: BRSetData(タブ, TRUE, "TAG=IMG", [src=EMPTY, n番目=1])
    :noindex:

    | IMGエレメントをクリックします

    :param TabWindowオブジェクト タブ: 値を取りたいページのタブを示す :ref:`tabwindow_object`
    :param 真偽値 TRUE: TRUEを指定 (固定)
    :param 文字列 "TAG=IMG": "TAG=IMG" を指定 (固定)
    :param 数値 省略可 src: 対象imgタグのsrcを指定
    :param 数値 省略可 n番目: srcが一致する場合に順番を1から指定
    :rtype: 真偽値
    :return: 成功時TRUE

IEGETSRC互換
^^^^^^^^^^^^

.. function:: BRGetSrc(タブ, タグ名, [n番目=1])

    | 指定タグのエレメントのouterHTMLを返します

    :param TabWindowオブジェクト タブ: 値を取りたいページのタブを示す :ref:`tabwindow_object`
    :param 文字列 タグ名: HTMLを取得したいタグ名
    :param 数値 省略可 n番目: タグの順番を1から指定
    :rtype: 文字列
    :return: 該当タグのHTMLソース、非該当ならEMPTY

IESETSRC互換
^^^^^^^^^^^^

.. admonition:: 非推奨関数
    :class: hint

    | ドキュメント全体の書き換えを非推奨としているため、互換関数は存在しません

IELINK互換
^^^^^^^^^^

.. function:: BRLink(タブ, リンク文字, [n番目=1, 完全一致=FALSE])

    | 指定リンクをクリックします

    :param TabWindowオブジェクト タブ: 値を取りたいページのタブを示す :ref:`tabwindow_object`
    :param 文字列 リンク文字: リンクに表示されている文字列(デフォルトは部分一致)
    :param 数値 省略可 n番目: リンク文字が同一の場合に順番を1から指定
    :param 真偽値 省略可 完全一致: TRUEの場合完全一致するリンク文字を検索する
    :rtype: 真偽値
    :return: 該当するリンクが存在しクリックを実行した場合TRUE

IEGETFRAME互換
^^^^^^^^^^^^^^

.. admonition:: 後日実装予定
    :class: note

    | TabWindowがフレーム対応し次第実装する予定です

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

    .. method:: argument(起動時オプション)

        | ブラウザの起動時オプションを追加します

        .. admonition:: 動作保証対象外の機能です
            :class: caution

            | これはブラウザ起動時のオプションを任意に追加できる機能です
            | この機能を利用した際の動作は保証されません
            | ブラウザ等への影響を理解している場合のみご利用ください
            | この機能を利用することにより生じた不具合はUWSCRのバグとしては扱われません

        :param 文字列 起動時オプション: 追加する起動時オプション
        :rtype: BrowserBuilder
        :return: 更新されたBrowserBuilder

        .. admonition:: サンプルコード

            .. sourcecode:: uwscr

                // ブラウザの拡張機能を無効にする
                builder = BrowserBuilder(BC_CHROME)
                builder.argument("--disable-extensions")
                chrome = builder.start()

    .. method:: start()

        | ブラウザを起動し :ref:`browser_object` を返します

        :rtype: :ref:`browser_object`
        :return: 対象ブラウザの :ref:`browser_object`

.. _browser_object:

Browserオブジェクト
~~~~~~~~~~~~~~~~~~~

| 操作対象となるタブを示すオブジェクト

.. admonition:: Browserオブジェクトの取得に時間がかかる場合がある
    :class: hint

    | Browserオブジェクト作成時に対象ブラウザに対してWebSocket接続を行います
    | WebSocket接続が確立されるまでにある程度の時間を要するのが原因です

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

.. _tabwindow_object:

TabWindowオブジェクト
~~~~~~~~~~~~~~~~~~~~~

| タブごとのWindowオブジェクトを示すオブジェクト

.. admonition:: 一度目のプロパティ取得やメソッド実行に時間がかかる場合がある
    :class: hint

    | タブ内のページ操作のためにWebSocketを使用していますが、初回のみWebSocketの接続処理が入ります
    | WebSocket接続が確立されるまでにある程度の時間を要するのが原因です

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

    .. method:: navigate(url)

        | 指定URLを開きます
        | ページの読み込み完了まで待機します (最大10秒)

        .. admonition:: 読み込み時間が長い場合
            :class: hint

            | 読み込みに10秒以上かかるページに対しては navigate実行後に :any:`wait` メソッドを呼んでください

        :param 文字列 url: 開きたいサイトのURL
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

    .. method:: eval(JavaScript式)

        | JavaScriptの式を評価し、オブジェクトの場合はRemoteObjectとして返します

        :param 文字列 JavaScript式: JavaScriptの式
        :rtype: :ref:`remote_object` またはいずれかの値型
        :return:

            | 評価結果がJavaScriptオブジェクトの場合は :ref:`remote_object` を返します
            | そうでない場合は該当するUWSCRの値型を返します

        .. admonition:: サンプルコード

            .. sourcecode:: uwscr

                chrome = BrowserControl(BC_CHROME)
                tab = chrome[0]
                tab.navigate(url)

                func = tab.eval("(a, b) => a + b") // アロー関数を評価
                print func(3, 5) // 8 (関数として実行できる)

                // コールバック用のJavaScript関数を作る
                callback = tab.eval("(event) => event.srcElement.style.backgroundColor = 'red'")
                slct = tab.document.querySelector("select")
                // イベントリスナをセット
                slct.addEventListener("change", callback)



.. _remote_object:

RemoteObject
~~~~~~~~~~~~

| ブラウザ上に存在するJavaScriptオブジェクトを示すオブジェクト

メソッドの実行
^^^^^^^^^^^^^^

| ``RemoteObject.メソッド名(引数)`` でメソッドを実行します
| メソッド名は大文字小文字を区別します

.. sourcecode:: uwscr

    chrome = BrowserControl(BC_CHROME)
    foo = chrome[0].document.querySelector("#foo")

プロパティの取得
^^^^^^^^^^^^^^^^

| ``RemoteObject.プロパティ名`` とすることでプロパティ値を取得します
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

インデックスによるアクセス
^^^^^^^^^^^^^^^^^^^^^^^^^^

| :ref:`remote_object` 自身が配列であった場合は ``RemoteObject[i]`` とすることで要素を得られます

.. sourcecode:: uwscr

    chrome = BrowserControl(BC_CHROME)
    links = chrome[0].document.querySelectorAll("a")
    print links[0].href

関数として実行
^^^^^^^^^^^^^^

| :ref:`remote_object` 自身が関数である場合は ``RemoteObject(引数)`` として実行できます

非同期関数とPromise
^^^^^^^^^^^^^^^^^^^

| :ref:`remote_object` 自身、またはそのメソッドが非同期関数であった場合 :ref:`await` 構文でその終了を待ちます
| :ref:`remote_object` がPromiseであった場合は :any:`WaitTask` 関数でその終了を待ちます
| いずれの場合も結果を返します

戻り値について
^^^^^^^^^^^^^^

:ref:`remote_object` のプロパティやメソッド、インデックスから得られる値の型は以下の通りです

.. list-table::
    :align: left
    :header-rows: 1

    * - JavaScript型
      - UWSCR型
    * - string
      - 文字列
    * - number
      - 数値
    * - bool
      - 真偽値
    * - null
      - NULL
    * - 上記以外のオブジェクト
      - :ref:`remote_object`
    * - オブジェクトでもプリミティブな値でもない場合 (undefinedなど)
      - EMPTY

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

ダウンロード先やその方法の制御について
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

| ダウンロードファイルの保存先フォルダの指定や、確認ダイアログの制御が現時点ではできません
| ブラウザ操作にて特定のフォルダへのダウンロードを確認なしで行いたい場合は事前に以下の操作を行ってください

1. :ref:`builder_object` で専用のプロファイルフォルダを指定し、ブラウザを起動する
2. 起動したブラウザの設定を手動で変更する

   - Chrome
        1. 設定画面の **ダウンロード** を開く
        2. **保存先** を任意のフォルダに変更する
        3. **ダウンロード前に各ファイルの保存場所を確認する** をオフにする
   - MSEdge
        1. 設定画面の **ダウンロード** を開く
        2. **場所** を任意のフォルダに変更する
        3. **ダウンロード時の動作を毎回確認する** をオフにする

3. 変更を施したプロファイルを指定して改めてブラウザ操作を行う

.. admonition:: ダウンロード開始と完了の検知
    :class: hint

    1. getdir関数で ``未確認*.crdownload`` ファイルの数を確認し、1個以上であればダウンロードが開始されていると判定
    2. | ダウンロードするファイルの名前がわかっている場合、F_EXISTSがTRUEならダウンロード完了
       | あるいはgetdir関数で ``未確認*.crdownload`` ファイルの数を確認し、0個であればダウンロード完了と判定

    .. sourcecode:: uwscr

        // ダウンロード開始検知
        repeat
            sleep(0.1)
            files = getdir(download_path, "未確認*.crdownload")
        until length(files) > 0

        if filename != EMPTY then
            // ファイル名が分かる場合
            repeat
                sleep(1)
            until fopen(filename, F_EXISTS)
        else
            // ファイル名が分からない場合
            repeat
                sleep(1)
                files = getdir(download_path, "未確認*.crdownload")
            until length(files) == 0
        endif

HTTPリクエスト
--------------

.. function:: Webrequest(url)

    | 指定URLに対してGETリクエストを送信します

    :param 文字列 url: リクエストを送るURL
    :rtype: :ref:`web_response`
    :return: レスポンスを示す :ref:`web_response`

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            res = WebRequest("http://example.com")
            print res.status
            print res.body

.. function:: WebRequestBuilder()

    | :ref:`web_request` を返します
    | :any:`WebRequest` とは異なり詳細な設定を行い任意のメソッドでリクエストを送信できます

    :rtype: :ref:`web_request`
    :return: リクエストを行うための :ref:`web_request`


.. _web_request:

WebRequestオブジェクト
~~~~~~~~~~~~~~~~~~~~~~

| HTTPリクエストを行うためのオブジェクト

.. class:: WebRequest

    .. method:: useragent(UA)

        | UserAgent文字列をUser-Agentヘッダに設定します
        | 未指定の場合設定されません

        :param 文字列 UA: UserAgent文字列
        :rtype: :ref:`web_request`
        :return: 更新された :ref:`web_request`

    .. method:: header(キー, 値)

        | リクエストヘッダを追加します

        :param 文字列 キー: ヘッダのキー
        :param 文字列 値: ヘッダの値
        :rtype: :ref:`web_request`
        :return: 更新された :ref:`web_request`

    .. method:: timeout(秒)

        | ヘッダを設定します
        | 未指定の場合タイムアウトしません

        :param 数値 秒: タイムアウト秒
        :rtype: :ref:`web_request`
        :return: 更新された :ref:`web_request`

    .. method:: body(本文)

        | リクエスト本文を設定します
        | 未指定の場合は何も送信しません

        :param 文字列またはUObject 本文: リクエスト本文、UObjectはjsonに変換されます
        :rtype: :ref:`web_request`
        :return: 更新された :ref:`web_request`

    .. method:: basic(ユーザー名, [パスワード=EMPTY])

        | Basic認証のユーザー名とパスワードを設定したAuthorizationヘッダを追加します
        | 未指定の場合は追加されません

        :param 文字列 ユーザー名: ユーザー名
        :param 文字列 省略可 パスワード: パスワード
        :rtype: :ref:`web_request`
        :return: 更新された :ref:`web_request`

    .. method:: bearer(トークン)

        | Bearer認証のトークンを設定したAuthorizationヘッダを追加します
        | 未指定の場合は追加されません

        :param 文字列 トークン: 認証トークン
        :rtype: :ref:`web_request`
        :return: 更新された :ref:`web_request`

    .. method:: get(url)

        | GETリクエストを送信します

        :param 文字列 url: リクエストを送るURL
        :rtype: :ref:`web_response`
        :return: :ref:`web_response`

    .. method:: put(url)

        | PUTリクエストを送信します

        :param 文字列 url: リクエストを送るURL
        :rtype: :ref:`web_response`
        :return: :ref:`web_response`

    .. method:: post(url)

        | POSTリクエストを送信します

        :param 文字列 url: リクエストを送るURL
        :rtype: :ref:`web_response`
        :return: :ref:`web_response`

    .. method:: delete(url)

        | DELETEリクエストを送信します

        :param 文字列 url: リクエストを送るURL
        :rtype: :ref:`web_response`
        :return: :ref:`web_response`

    .. method:: patch(url)

        | PATCHリクエストを送信します

        :param 文字列 url: リクエストを送るURL
        :rtype: :ref:`web_response`
        :return: :ref:`web_response`

    .. method:: head(url)

        | HEADリクエストを送信します

        :param 文字列 url: リクエストを送るURL
        :rtype: :ref:`web_response`
        :return: :ref:`web_response`

.. admonition:: サンプルコード

    .. sourcecode:: uwscr

        request = WebRequestBuilder()
        // ヘッダと認証情報を設定しておく
        request.bearer(MY_BEARER_TOKEN)_
            .header('Content-Type', 'application/json')

        // リクエストを送信
        res1 = request.body(json1).post(url1)
        res2 = request.body(json2).put(url2)

.. _web_response:

WebResponseオブジェクト
~~~~~~~~~~~~~~~~~~~~~~~

| HTTPレスポンスを示すオブジェクト

.. class:: WebResponse

    .. property:: status

        | レスポンスのステータスを数値で返します

    .. property:: statusText

        | レスポンスのステータスを示す文字列を返します

    .. property:: succeed

        | リクエストの成否を真偽値で返します

    .. property:: header

        | レスポンスヘッダを連想配列で返します

    .. property:: body

        | レスポンスボディを文字列で返します、返せない場合はEMPTY

    .. property:: json

        | レスポンスボディがjsonの場合UObjectを返します、返せない場合はEMPTY

HTTPパーサー
-------------

.. function:: ParseHTML(html)

    | HTMLをパースし :ref:`node_object` を返します

    :param 文字列またはWebResponse html: HTMLドキュメントまたはその一部を示す文字列、またはHTMLドキュメントとして受けた :ref:`web_response`
    :rtype: :ref:`node_object`
    :return: パースされたHTMLドキュメントを示す :ref:`node_object`

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            res = WebRequest(url)
            // WebResponseオブジェクトからHtmlNodeオブジェクトを得る
            doc = ParseHTML(res)

            // ラジオボタンのvalue値を列挙
            for radio in doc.find('input[type="radio"]')
                print radio.attr('value')
            next

            // 最初のselect要素内のoptionのテキストと値を列挙
            slct = doc.first('select')
            for opt in slct.find('option')
                print opt.text
                print opt.attr('value')
            next

.. _node_object:

HtmlNodeオブジェクト
~~~~~~~~~~~~~~~~~~~~

| パースされたHTMLドキュメントおよびエレメントを示すオブジェクト

.. class:: HtmlNode

    .. method:: find(selector)

        | cssセレクタに該当するエレメント郡を :ref:`node_object` の配列として返す
        | 空ノードの場合常に空の配列を返す

        :param 文字列 selector: cssセレクタ
        :rtype: :ref:`node_object` 配列
        :return: cssセレクタに該当するエレメントの :ref:`node_object` 配列

    .. method:: first(selector)
    .. method:: findfirst(selector)

        | cssセレクタに該当する最初のエレメントを :ref:`node_object` として返す
        | 該当するエレメントがない場合は空ノードを返す
        | 空ノードの場合常に空ノードを返す

        :param 文字列 selector: cssセレクタ
        :rtype: :ref:`node_object`
        :return: cssセレクタに該当する最初のエレメントの :ref:`node_object`

    .. method:: attr(属性名)
    .. method:: attribute(属性名)

        | エレメントの属性名を指定してその値を返す
        | HTMLドキュメント、空ノードの場合は常にEMPTYを返す

        :param 文字列 属性名: 属性の名前
        :rtype: 文字列またはEMPTY
        :return: 該当する属性の値、属性がない場合EMPTY

    .. property:: outerhtml

        - HTMLドキュメント: 全体のHTMLを文字列で返す
        - エレメント: エレメント自身を含むHTMLを文字列で返す
        - 空ノード: EMPTY

    .. property:: innerhtml

        - HTMLドキュメント: EMPTY
        - エレメント: エレメント以下のHTMLを文字列で返す
        - 空ノード: EMPTY

    .. property:: text

        - HTMLドキュメント: EMPTY
        - エレメント: エレメントのテキストノードを文字列の配列で返す
        - 空ノード: EMPTY

    .. property:: isempty

        | 空ノードであればTRUE
