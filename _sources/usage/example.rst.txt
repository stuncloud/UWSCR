サンプルコード集
================

| UWSCRのサンプルコードを掲載していきます
| 思いつき次第拡充していく予定です
| サンプルコードを見たい機能がある場合は `オンラインヘルプ関連 <https://github.com/stuncloud/UWSCR/issues/new/choose>`_ のissueでリクエストしてください


新機能を使ったサンプルコード
----------------------------

時間の計測 (クラス、クロージャ)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

| 実行時間計測用のタイマーを実装してみます
| ``gettime`` を二度呼び出しその差分を得ることで経過時間を取得できますが、UWSCでは以下のような問題がありました

- ミリ秒での計測が面倒
- 最初のgettimeの結果を隠蔽しにくい

| UWSCRの ``gettime`` はミリ秒に対応しています
| それと、新たに追加されたクラスやクロージャ機能を使ってこれらを解決していきます
| この方法だと上記を解決するばかりか、複数の計測を並行するのも簡単になります

クラスを使う
~~~~~~~~~~~~

| Timerクラスを実装し、それにより経過時間を計測します
| インスタンスを複数作ることで並行計測も可能です

.. sourcecode:: uwscr

    class Timer
        dim from
        // コンストラクタで計測開始時の時間をセット
        procedure Timer()
            this.from = this.now()
        fend
        // 現在時刻をミリ秒で取得
        dim now = function()
            result = gettime(, , , TRUE)
        fend
        // 経過時間を返す
        function elapsed()
            result = this.now() - this.from
        fend
    endclass

    t = Timer()
    msgbox("好きなタイミングでOKを押す")
    msg = t.elapsed() + " ミリ秒経過しました"
    msgbox(msg)

クロージャを使う
~~~~~~~~~~~~~~~~

| クラスよりもスッキリ書きたい場合はこちら
| 関数の戻り値を無名関数にすることで、その中に値を閉じ込めておくことができます
| この場合は計測開始時を戻り値の無名関数に持たせておくことで、その関数を実行すると経過時間が得られる仕組みです

.. sourcecode:: uwscr

    // 経過時間を得る関数を返す関数 (エンクロージャ)
    function timer()
        s = gettime(,,,TRUE)
        // 経過時間を返す関数 (クロージャ)
        // 計測開始時間(s)を保持している
        result = function()
            result = gettime(,,,TRUE) - s
        fend
    fend

    elapsed = timer()
    msgbox("好きなタイミングでOKを押す")
    msg = elapsed() + " ミリ秒経過しました"
    msgbox(msg)

UWSCでやるなら？
~~~~~~~~~~~~~~~~

| モジュールを使えば一応要件を満たすことはできます
| id指定により並行計測も可能としていますが、都度id指定が必要なのが不便ですね

.. sourcecode:: uwscr

    Timer.Start(1)
    msgbox("好きなタイミングでOKを押す")
    msg = Timer.End(1) + " ミリ秒経過しました"
    msgbox(msg)

    module Timer
        hashtbl s
        procedure Start(id)
            s[id] = GetTickCount()
        fend
        function End(id)
            result = GetTickCount() - s[id]
        fend

        def_dll GetTickCount():dword:kernel32.dll
    endmodule

TIPS
----

UWSCとUWSCRを判別
^^^^^^^^^^^^^^^^^

| ``GET_UWSC_PRO`` で判別できます
| UWSCではPro版か否かでTRUEまたはFALSEを返していましたが、UWSCRではEMPTYを返します

.. sourcecode:: uwscr

    select GET_UWSC_PRO
        case TRUE
            print "UWSC Pro版です"
        case FALSE
            print "UWSC 無料版です"
        case EMPTY
            print "UWSCRです"
    selend