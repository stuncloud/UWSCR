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

イテレータ
^^^^^^^^^^

| classや無名関数を用いてイテレータっぽいものが作れます

実装
~~~~

.. sourcecode:: uwscr

    class Iter
        dim list
        dim index = 0
        dim type

        procedure Iter(list)
            t = type_of(list)
            select t
                case TYPE_ARRAY
                    this.list = list
                    this.type = t
                case TYPE_HASHTBL
                    hashtbl cpy
                    for key in list
                        cpy[key] = list[key]
                    next
                    this.list = cpy
                    this.type = t
                default
                    raise("<#t>はイテレータにできません", "Iter型エラー")
            selend
        fend

        function to_list()
            result = this.list
        fend


        function next()
            if this.index < length(list) then
                select this.type
                    case TYPE_ARRAY
                        result = this.list[this.index]
                    case TYPE_HASHTBL
                        result = this.list[this.index, HASH_VAL]
                selend
                this.index += 1
            else
                result = EMPTY
            endif
        fend

        function map(f: func)
            select this.type
                case TYPE_ARRAY
                    for i = this.index to length(this.list) - 1
                        list[i] = f(this.list[i])
                    next
                case TYPE_HASHTBL
                    for i = this.index to length(this.list) - 1
                        key = this.list[i, HASH_KEY]
                        list[key] = f(this.list[key])
                    next
            selend
            result = this
        fend

        function filter(f: func)
            select this.type
                case TYPE_ARRAY
                    new = []
                    for i = this.index to length(this.list) - 1
                        if f(this.list[i]) then
                            new += this.list[i]
                        endif
                    next
                    this.list = new
                case TYPE_HASHTBL
                    for key in this.list
                        if ! f(key, this.list[key]) then
                            |=>this.list[key, HASH_REMOVE]|()
                        endif
                    next
            selend
            result = this
        fend

        function find(f: func)
            select this.type
                case TYPE_ARRAY
                    for i = this.index to length(this.list) - 1
                        if f(this.list[i]) then
                            result = this.list[i]
                            exit
                        endif
                    next
                case TYPE_HASHTBL
                    for key in this.list
                        if f(key, this.list[key]) then
                            result = this.list[key]
                            exit
                        endif
                    next
            selend
            result = this
        fend

        function reduce(f: func)
            select this.type
                case TYPE_ARRAY
                    result = this.list[this.index]
                    for i = this.index + 1 to length(this.list) - 1
                        result = f(result, this.list[i])
                    next
                case TYPE_HASHTBL
                    result = this.list[this.index, HASH_VAL]
                    for i = this.index + 1 to length(this.list) - 1
                        result = f(result, this.list[i, HASH_VAL])
                    next
            selend
        fend
    endclass

使い方
~~~~~~

.. sourcecode:: uwscr

    a = [1,2,3]

    hash h
        "a" = 1
        "b" = 2
        "c" = 3
    endhash

    // map
    f = | n => n * 2|
    print Iter(a).map(f).to_list() // [2, 4, 6]
    print Iter(h).map(f).to_list() // {"A": 2, "B": 4, "C": 6}


    // filter
    print Iter(a).filter(|n => n mod 2 == 1|).to_list() // [1, 3]
    // 連想配列はキーと値を受けてフィルタできる
    print Iter(h).filter(|k,v => v mod 2 == 1|).to_list() // {"A": 1, "C": 3}

    // reduce
    f = | x, y => x + y |
    print Iter(a).reduce(f) // 6
    print Iter(h).reduce(f) // 6

    // find
    print Iter(a).find(| n => n mod 2 == 0|) // 2
    print Iter(h).find(| k, v => k == "c"|) // 3

    // 複合
    print Iter([1,2,3,4,5,6,7,8,9]) _
            .filter(| n => n mod 2 == 0 |) _ // 偶数
            .map(| n => n + 1 |) _ // それぞれに+1
            .reduce(| m, n => m + n|) // 合計を出す: 24

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

