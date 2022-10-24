ブラウザ操作
============

Browserオブジェクト作成
-----------------------

.. function:: BrowserControl(ブラウザ定数, [フィルタ=EMPTY, ポート=9222, ヘッドレス=FALSE])

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

    :param 文字列 省略可 フィルタ: タイトルまたはURLにマッチするタブを操作します、省略時は1番目のタブ
    :param 数値 省略可 ポート: デバッグポートを変更します
    :param 真偽値 省略可 ヘッドレス: TRUEにした場合はブラウザを非表示(ヘッドレス)で起動します、再接続時は無視されます

    :return: :ref:`browser_object`

    .. tip:: ブラウザパスの指定方法

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

.. _browser_object:

Browserオブジェクト
-------------------

| 操作対象となるタブを示すオブジェクト

.. class:: Browser

    .. property:: document

        documentの :ref:`element_object` を返します

    .. property:: url

        開いているページのURLを返します

    .. property:: source

        | 開いているページのHTMLソースを返します

        .. note:: その時点でのDOM構造をHTMLとして返すため、もとのhtmlファイルの内容とは異なる場合があります

    .. property:: pageid

        | 開いているページのIDを返します

    .. method:: windowid()

        | 対象ブラウザのウィンドウIDを返します

        :rtype: ウィンドウID
        :return: ブラウザのウィンドウID、失敗時は-1

    .. method:: navigate(URI)

        | 指定したURIを開きます
        | ページ遷移が完了するまで自動で待機します (最大10秒)

        :param 文字列 URI: 開きたいサイトのURI
        :rtype: 真偽値
        :return: タイムアウトした場合はFALSE

    .. method:: reload([キャッシュ無視=FALSE])

        | ページをリロードします
        | リロードが完了するまで自動で待機します (最大10秒)

        :param 真偽値 省略可 キャッシュ無視: TRUEならキャッシュを無視する(`Shift+refresh` と同等)
        :rtype: 真偽値
        :return: タイムアウトした場合はFALSE

    .. method:: wait([タイムアウト秒=10])

        | ページの読み込みが完了するのを待ちます
        | リンクをクリックした後などに使用します

        .. hint:: navigateやreloadは自動的に待機するためこのメソッドを呼ぶ必要はありません

        :param 数値 省略可 タイムアウト秒: 読み込み完了まで待機する最大時間 (秒)
        :rtype: 真偽値
        :return: タイムアウトした場合はFALSE

    .. method:: close()

        | 操作中のタブを閉じます

        :return: なし

    .. method:: getTabs([フィルタ])

        | タブ一覧を取得します

        :param 文字列 省略可 フィルタ: 指定時はタイトルまたはURLがマッチするタブのみ取得
        :rtype: 二次元配列
        :return: [タイトル, URL, ページID] を要素に持つ二次元配列

        .. admonition:: サンプルコード

            .. sourcecode:: uwscr

                for tab in browser.gettabs()
                    print 'title : ' + tab[0]
                    print 'url   : ' + tab[1]
                    print 'pageid: ' + tab[2]
                next

    .. method:: newTab(URI)

        | 新しいタブを開き、そのタブのBrowserオブジェクトを返します

        :param 文字列 URI: 開きたいサイトのURI
        :rtype: Browserオブジェクト
        :return: 開いたタブのBrowserオブジェクト

    .. method:: activate()

        | 操作対象のタブをアクティブにします

        :return: なし

    .. method:: execute(JavaScript, [引数, 変数名="arg"])

        | ブラウザ上でJavaScriptを実行します

        :param 文字列 JavaScript: 実行するJavaScriptコード
        :param 値 省略可 引数: 実行するJavaScriptに渡す値、UObjectを渡せばJavaScriptのオブジェクトに変換される
        :param 文字列 省略可 変数名: JavaScript上で引数を受ける変数名
        :rtype: 該当する値型
        :return: スクリプトの実行結果

        .. admonition:: サンプルコード

            .. sourcecode:: uwscr

                print browser.execute('3 + 5') // 8

                // 引数を渡す

                // 変数名が未指定の場合argという変数が使える
                // UObjectを渡した場合はJavaScript内でオブジェクトに変換される
                print browser.execute('arg.a * arg.b', @{"a": 3, "b": 5}@) // 15

                // 変数名を指定するとその変数名で引数にアクセスできる
                print browser.execute('3 * hoge', 6, "hoge") // 18

.. _element_object:

Elementオブジェクト
-------------------

| DOMにおけるエレメントオブジェクトを示すオブジェクト

.. class:: Element

    | UWSCRのElementオブジェクト専用のプロパティおよびメソッドです
    | 名前の大小文字を区別しません

    .. property:: parent

        親となるElementオブジェクトを取得します

        .. admonition:: サンプルコード

            .. sourcecode:: uwscr

                element = browser.document.querySelector(selector)
                parent = element.parent


    .. method:: querySelector(セレクタ)

        | CSSセレクタを指定し該当するエレメントのElementオブジェクトを取得します

        :param 文字列 セレクタ: エレメントを指定するCSSセレクタ
        :rtype: Elementオブジェクト
        :return: 該当するエレメントがなかった場合はEMPTY

        .. admonition:: サンプルコード

            .. sourcecode:: uwscr

                form = browser.document.querySelector("form")
                input_pwd = form.querySelector('input[type="password"]')

    .. method:: querySelectorAll(セレクタ)

        | CSSセレクタを指定し該当するエレメントすべてのElementオブジェクトを取得します

        :param 文字列 セレクタ: エレメントを指定するCSSセレクタ
        :rtype: 配列
        :return: 該当するすべてのElementオブジェクトの配列、該当なしなら空配列

        .. admonition:: サンプルコード

            .. sourcecode:: uwscr

                form = browser.document.querySelector("form")
                inputs = form.querySelectorAll("input")
                for input in inputs
                    print input.type
                next

    .. method:: focus()

        | エレメントをフォーカスします

        :return: なし

    .. method:: input(入力値)

        | input要素などに指定文字列を入力します

        :param 文字列 入力値: 入力する文字列、入力は一文字ずつ行われる
        :return: なし

    .. method:: clear()

        | エレメントのvalueを空にします

        :return: なし

        .. admonition:: サンプルコード

            .. sourcecode:: uwscr

                element = browser.document.querySelector('input[type="text"]')
                print element.value // 元の入力値
                element.clear()
                print element.value // 空になっている

    .. method:: setFile(ファイルパス)
    .. method:: setFile(ファイルパス配列)
        :noindex:

        | ファイル選択(``input[type="file"]``)に値を入力します

        :param 文字列 ファイルパス: ``input[type="file"]`` に入力するファイルパス
        :param 配列 ファイルパス配列: ``multiple`` が有効な場合に複数のパスを指定できる

        :return: なし

        .. admonition:: サンプルコード

            .. sourcecode:: uwscr

                element = browser.document.querySelector('input[type="file"]')
                element.SetFile("hoge.jpg")

    .. method:: click()

        | エレメントをクリックします

        :return: なし

    .. method:: select()

        | チェックボックスやラジオボタンを選択状態にします

        :return: なし

    .. method:: execute(JavaScript, [引数, 変数名="arg"])

        | JavaScriptを実行します
        | エレメント自身は ``$0`` でアクセス可能

        :param 文字列 JavaScript: 実行するJavaScriptコード
        :param 値 省略可 引数: 実行するJavaScriptに渡す値、UObjectを渡せばJavaScriptのオブジェクトに変換される
        :param 文字列 省略可 変数名: JavaScript上で引数を受ける変数名
        :rtype: 該当する値型
        :return: スクリプトの実行結果

        .. admonition:: サンプルコード

            .. sourcecode:: uwscr

                element = browser.document.querySelector('input[type="button"]')
                element.execute('$0.onclick()') // エレメントのonclickを実行する

属性(アトリビュート)値やプロパティの取得・変更
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

| DOMにおけるエレメントオブジェクトが持つ属性値やプロパティの取得及び変更が可能です
| ``Elementオブジェクト.名前`` でアクセスできます
| 属性値やプロパティの名前は大小文字を区別します (ブラウザの仕様)
| 存在しない名前を指定した場合NULLが返ります

.. admonition:: サンプルコード

    .. sourcecode:: uwscr

        element = browser.document.querySelector(selector)
        // valueを得る
        print element.value
        // innerHTMLを書き換える
        inner = element.innerHTML
        element.innerHTML = "<p><#inner></p>" // innerHTMLをPタグで包む

.. warning:: メソッドにはアクセスできません