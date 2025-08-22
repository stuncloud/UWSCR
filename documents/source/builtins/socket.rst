ソケット通信
============

- UDP
- TCP
- WebSocket

| を利用した通信を行うための関数群です

共通
----

.. function:: sclose(ソケット)

    | ソケットを閉じます

    .. note:: 自動クローズ

        | ソケットはソケットオブジェクトが破棄された時点で閉じられます
        | 以下の方法のいずれでもソケットを閉じることができます

        .. sourcecode:: uwscr

            udp1 = udpclient(addr, port)
            sclose(udp1) // sclose関数を使う

            udp2 = udpclient(addr, port)
            udp2 = EMPTY // 別の値で上書きしソケットオブジェクトが失われれば自動クローズされる



    :param ソケットオブジェクト ソケット: 以下のいずれかを指定

        - UDPClient
        - WebSocket


UDP通信
-------

.. admonition:: サンプルコード

    .. sourcecode:: uwscr

        const PORT_SEND = 50101
        const PORT_RECV = 50303

        function sender()
            // 呼び出しから3秒後にデータを送信する
            const LOCALHOST = "127.0.0.1"
            client = udpclient(LOCALHOST, PORT_SEND)
            sleep(3)
            udpsend(client, LOCALHOST, PORT_RECV, "UDP通信テスト")
        fend


        client = udpclient("0.0.0.0", PORT_RECV)

        // 送信スレッドを呼ぶ
        thread sender()

        // 受信待機
        r = udprecv(client, 100)

        // 受信データを整形
        data = decode(r[0], CODE_BYTEARRAYU)
        addr = r[1]
        port = r[2]

        print "<#addr>:<#port> からメッセージを受信しました: <#data>"
        // 127.0.0.1:50101 からメッセージを受信しました: UDP通信テスト

        sclose(client)

.. function:: UdpClient(IPアドレス, ポート)

    | 任意のアドレスとポートで待ち受けるUDPクライアントオブジェクトを返す

    :param 文字列 IPアドレス: 自身の待ち受けIPアドレス
    :param 数値 ポート: 自身の待ち受けポート
    :rtype: UDPクライアント
    :return: UDP送受信を行うためのオブジェクト

.. function:: UdpSend(udp, IPアドレス, ポート, 送信データ)

    | UDPによるデータ送信を行う

    :param UDPクライアント udp: データを送信するUDPクライアント
    :param 文字列 IPアドレス: 送信先IPアドレス
    :param 数値 ポート: 送信先ポート
    :param 値 送信データ:

        | 以下のいずれかの型の値に対応

        - 文字列: UTF8バイト配列に変換される
        - UObject: json文字列としてUTF8バイト配列に変換される
        - バイト配列: encode関数の戻り値等
        - 数値配列: 数値 (0-255) の配列、数値以外や範囲外が含まれていたらエラーとなる

    :rtype: 真偽値
    :return: 送信成功時TRUE


.. function:: UdpRecv(バッファサイズ)

    | UDPによるデータ受信を行う
    | データを受信するまでブロックする

    :param 数値 バッファサイズ:

        | 受信するデータ (バイト配列) のバッファサイズ
        | 実際の受信データより小さいとデータが欠損する場合があります

    :rtype: [バイト配列, 文字列, 数値]
    :return: [受信データ, 送信元IPアドレス, 送信元ポート]


TCP通信
-------

.. function:: TcpSend(IPアドレス, ポート, 送信データ)

    | TCPで接続先にデータを送信し、受け取ったレスポンスを返す

    :param 文字列 IPアドレス: 対象サーバーのIPアドレス
    :param 数値 ポート: 対象サーバーのポート
    :param 値 送信データ:

        | 以下のいずれかの型の値に対応

        - 文字列: UTF8バイト配列に変換される
        - UObject: json文字列としてUTF8バイト配列に変換される
        - バイト配列: encode関数の戻り値等
        - 数値配列: 数値 (0-255) の配列、数値以外や範囲外が含まれていたらエラーとなる

    :rtype: バイト配列
    :return: レスポンスデータを示すバイト配列

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            // example.comにGETリクエストを送る

            // GETリクエストデータ
            // 末尾に改行を2つ入れないとダメ
            textblock request
            GET /index.html HTTP/1.1
            Host: example.com
            Connection: close


            endtextblock

            // GETリクエストを送信
            res = TcpSend("23.192.228.80", 80, request)
            // レスポンスデータを文字列に変換してprint
            print decode(res, CODE_BYTEARRAYU)

.. function:: TcpListener(IPアドレス, ポート, ハンドラ, [終端文字="<#CR>", タイムアウト秒=10])

    | 指定アドレス及びポートでTCP接続の待ち受けを行う

    :param 文字列 IPアドレス: 待ち受けIPアドレス
    :param 数値 ポート: 待ち受けポート
    :param 関数 ハンドラ:

        | 受信したデータをバイト配列として受け、クライアントに返信するデータを戻り値とする関数
        | 返信に有効な型は以下

        - 文字列: UTF8バイト配列に変換され返信される
        - UObject: json文字列がUTF8バイト配列に変換され返信される
        - バイト配列: encode関数の戻り値等
        - 数値配列: バイト配列に変換可能であれば返信される
        - FALSE, NULL, EMPTY: 待ち受け状態を抜ける (クライアントには空データが返る)

    :param 文字 省略可 終端文字:

        | 受信データの終端と判断するASCII文字 (chr(0)～chr(255))
        | この文字が送られてこないとデータ受信が終わらずレスポンスを返せない
        | 省略時はCRLF (``"#CR"``)

    :param 数値 タイムアウト秒: 受信できない場合のタイムアウト秒 (終端文字が送られない場合などにタイムアウトする可能性がある)
    :return: なし

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            // 受信データハンドラ
            // 受信内容により返信を変更する
            function handler(bytes)
                received = decode(bytes, CODE_BYTEARRAYU)
                select received
                    case "Ping"
                        result = "Pong"
                    case "さようなら"
                        result = "またね"
                    default
                        result = "こんにちは、<#received>さん"
                selend
            fend

            // 別スレッドでリッスン開始
            thread TcpListener("0.0.0.0", 9999, handler)

            // データ送信関数ラッパー
            send = function(data: string)
                // デフォルトではTcpListenerのデータ終端が改行なので末尾に<#CR>を加える
                res = TcpSend("127.0.0.1", 9999, "<#data><#CR>")
                result = decode(res, CODE_BYTEARRAYU)
            fend

            sleep(1)
            print send("🐊")
            // こんにちは、🐊さん
            sleep(1)
            print send("Ping")
            // Pong
            sleep(1)
            print send("さようなら")
            // またね


WebSocket
---------

.. admonition:: サンプルコード

    .. sourcecode:: uwscr

        // MSEdgeのデバッグポートを開いて起動
        shexec("msedge.exe", "--remote-debugging-port=9515")
        sleep(1)

        // WebSocket用のURLを得る
        res = webrequest("http://localhost:9515/json/version")
        uri = res.json.webSocketDebuggerUrl
        print "webSocketDebuggerUrl: <#uri>"

        // WebSocketオブジェクトを作成
        ws = WebSocket(uri)
        print ws

        // リクエスト用jsonオブジェクトを作る
        request = @{
            "id": 1,
            "method": "Target.getTargets",
            "params": {}
        }@
        // リクエストを送信
        WsSend(ws, request)

        while TRUE
            // データを受信
            res = WsRecv(ws)
            obj = fromjson(res)
            if obj.id == request.id then
                // idが一致したら抜ける
                break
            endif
        wend

        // Target.getTargetsメソッドの戻り値のうち、ページを示すものの情報を表示
        for info in obj.result.targetInfos
            if info.type == "page" then
                print
                print "type : " + info.type
                print "title: " + info.title
                print "url  : " + info.url
            endif
        next

.. function:: WebSocket(wsuri)

    | WebSocketに接続する

    :param 文字列 wsuri: ``ws://`` から始まるURI
    :rtype: WebSocket
    :return: WebSocketオブジェクト

.. function:: WsSend(WebSocket, 送信データ)

    | WebSocketでデータを送信する

    :param WebSocket WebSocket: WebSocketオブジェクト
    :param 値 送信データ:

        | 以下のいずれかの型の値に対応

        - 文字列
        - UObject
        - バイト配列
        - 定数
            - ``WS_PING`` : pingを送信する
            - ``WS_PONG`` : pongを送信する

    :rtype: 戻り値の型
    :return: 戻り値の説明

.. function:: WsRecv(WebSocket)

    | WebSocketでデータを受信する

    :param WebSocket WebSocket: WebSocketオブジェクト
    :rtype: 文字列、バイト配列、定数、EMPTY
    :return: 受信データによる

        .. admonition:: 受信データの型に注意
            :class: important

            | データの型が不明な場合は ``type_of`` 関数で型のチェックを行ってください

            .. sourcecode:: uwscr

                res = WsRecv(ws)
                select type_of(res)
                    case TYPE_STRING
                        print "received string: <#res>"
                    case TYPE_BYTE_ARRAY
                        print "received bytes: <#res>"
                    case TYPE_NUMBER
                        select res
                            case WS_PING
                                print "received ping"
                            case WS_PONG
                                print "received pong"
                        selend
                    default
                        print "received invalid data: <#res>"
                selend

