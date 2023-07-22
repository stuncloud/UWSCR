スクリプト構文
==============

識別子
------------------------------

| 識別子とは変数、定数、関数などの名前を示す文字列です
| 以下の文字の組み合わせで命名できます

- 英字 (大文字・小文字の区別はしません)
- 数字
- 記号
    - ``_``
- 全角文字

キーワード一覧
^^^^^^^^^^^^^^

識別子 (変数名、定数名、関数名など) に使用できないキーワード
++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++

- 特殊構文キーワード
    - call
    - async
    - await
- 特殊な値を示すもの
    - null
    - empty
    - nothing
    - true, false
    - NaN
- 演算子
    - mod
    - and, andL, andB
    - or, orL, orB
    - xor, xorL, xorB

変数定義
--------

dim
^^^^

| ローカル変数を定義します

.. code:: uwscr

    dim hoge     // 変数 hoge を定義
    dim fuga = 1 // 値の代入も同時に行える
    piyo = 1     // 未宣言変数への代入式で新たな変数が定義される(dim省略)

public
^^^^^^

グローバル変数を定義します

.. sourcecode:: uwscr

    public hoge = 1
    fuga() // 1
    hoge = 2
    fuga() // 2

    procedure fuga()
        print hoge
    fend

const
^^^^^

| 定数を定義します
| 再代入ができません

.. sourcecode:: uwscr

    const hoge = 1
    hoge = 2 // エラー

一括定義
^^^^^^^^^^^^^^^^^^^^^^

``,`` 区切りで変数を一括定義できます

.. sourcecode:: uwscr

    dim a = 1, b = 2, c, d[3], e[] = 1,2,3,4,5
    public f = 1, g = 2
    const h = 1, i = 2

UWSCではエラーになっていたconstの一括定義も可能

.. caution::

    配列定義に続けて記述するのはNG

    .. sourcecode:: uwscr

        dim foo[] = 1,2,3 , a = 1 // a = 1 は定義できない


配列
^^^^^^^^^^^^^^^^^^^^^^

配列の定義はdimを使った方法と、配列リテラル(新機能)を使う方法があります

.. sourcecode:: uwscr

    dim hoge[] = 1, 2, 3 // 従来の配列定義
    print hoge[0]

    fuga = [1, 2, 3]     // 配列リテラル
    print fuga[0]

    print [4, 5, 6][0]   // 配列リテラルにインデックスを指定することも可能


+演算子による要素の追加
+++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++

| ``+`` 演算子で配列の末尾に要素を追加できます

.. sourcecode:: uwscr

    print [1, 2, 3] + 4
    // [1, 2, 3, 4]
    dim arr = [5, 6, 7]
    arr += 8
    print arr
    // [5, 6, 7, 8]

多次元配列
^^^^^^^^^^^^^^^^^^^^^^

.. code-block::

    // 2次元
    dim 配列名[要素数][要素数] = 値, 値, 値, 値 ...
    // 3次元
    dim 配列名[要素数][要素数][要素数] = 値, 値, 値, 値 ...

    // 以下の書式も可能
    dim 配列名[要素数, 要素数] = 値, 値, 値, 値 ...
    dim 配列名[要素数, 要素数, 要素数] = 値, 値, 値, 値 ...

    // 一番左の要素数のみ省略可能
    dim 配列名[][要素数][要素数] = 値, 値, 値, 値 ...
    dim 配列名[, 要素数, 要素数] = 値, 値, 値, 値 ...

    // 呼び出しは次元数分だけ[]をつける

    print 配列名[0][0][0] // 3次元配列の1つ目の要素


.. sourcecode:: uwscr

    // 不足分はEMPTYで埋められる
    dim sample1[2][1] = 0, 1, 2, 3
    print sample1 // [[0, 1], [2, 3], [, ]]

    // 超過分は捨てられる
    dim sample2[1, 1] = 0, 1, 2, 3, 4, 5
    print sample2 // [[0, 1], [2, 3]]

    // 要素数省略
    dim sample3[][1] = 1,2,3,4,5,6,7,8
    print sample3 // [[1,2] ,[3,4], [5,6], [7,8]]

    // 一番左以外は省略不可
    dim bad_sample[][][1] // エラー

配列リテラルを使って多次元配列を作ることもできます

.. sourcecode:: uwscr

    dim sample4[] = [1,2], [3,4], [5,6], [7,8]
    sample5 = [[1,2] ,[3,4], [5,6], [7,8]]


連想配列
^^^^^^^^^^^^^^^^^^^^^^

.. code-block::

    hashtbl 連想配列変数                              // 連想配列を宣言
    hashtbl 連想配列変数 = HASH_CASECARE              // キーの大文字小文字を区別
    hashtbl 連想配列変数 = HASH_SORT                  // キーでソート(※1)
    hashtbl 連想配列変数 = HASH_CASECARE or HASH_SORT // 大小文字区別かつソート

    連想配列変数[キー] = 値                  // 任意のキー名で値を代入、数値のキーは文字列に変換される
    値 = 連想配列変数[キー]                  // キー名で値を読み出す、キーがない場合はEMPTY
    真偽値 = 連想配列変数[キー, HASH_EXISTS] // キーが存在するかどうか ※2
    真偽値 = 連想配列変数[キー, HASH_REMOVE] // キーを削除、成功時はTRUE
    キー = 連想配列変数[i, HASH_KEY]         // i番目の要素のキーを取得 ※3
    値 = 連想配列変数[i, HASH_VAL]           // i番目の要素の値を取得 ※3

    連想配列変数 = HASH_REMOVEALL  // 要素をすべて消す

    // カンマ区切りで一括定義可能、オプションも指定できる
    hashtbl 変数1, 変数2 = HASH_CASECARE, 変数3 = HASH_SORT

.. admonition:: ※1

    ``HASH_SORT`` によるキーソート順はUWSCと異なる場合があります

.. admonition:: ※2

    UWSCとは異なり変数で受けなくてもエラーになりません

.. admonition:: ※3

    iは0から

    - ``HASH_CASECARE`` がない場合は代入した順序
    - ``HASH_CASECARE`` がある場合はキーによりソートされた順序


.. sourcecode:: uwscr

    hashtbl hoge
    hoge["foo"] = 100
    print hoge["foo"] // 100
    hoge["FOO"] = 200
    print hoge["foo"] // 200 大小文字区別がないため上書きされた
    hoge["bar"] = 400
    hoge["baz"] = 600

    for i = 0 to length(hoge) - 1
        print hoge[i, HASH_KEY] // foo, bar, baz の順で表示される
        print hoge[i, HASH_VAL] // 200, 400, 600
    next

    print hoge["bar", HASH_EXISTS] // true
    print hoge["qux", HASH_EXISTS] // false
    hoge["bar", HASH_REMOVE] // 変数で受けなくてもOK
    print hoge["bar", HASH_EXISTS] // false

    hashtbl fuga = HASH_CASECARE
    fuga["foo"] = 1
    fuga["Foo"] = 2
    fuga["FOO"] = 3
    print fuga // {"foo": 1, "Foo": 2, "FOO": 3}

    hashtbl piyo = HASH_SORT
    piyo["b"] = ""
    piyo["z"] = ""
    piyo["a"] = ""
    print piyo // {"A": , "B": , "Z": }

連想配列一括定義
++++++++++++++++

| ``hash-endhash`` で連想配列を一括定義できます

.. code-block::

    hash [public] 変数名 [=オプション]
        [キー = 値]
    endhash

.. object:: public (省略可)

    指定するとグローバル変数、省略時はローカル変数になる

.. object:: オプション (省略可)

    | ``HASH_SORT`` と ``HASH_CASECARE`` を指定可能
    | 省略時はオプションなし

.. object:: キー = 値

    | キーと値の組み合わせを指定する
    | 複数指定可
    | キーは文字列だが `''` や `""` は省略可能
    | 一つも指定しない場合空の連想配列ができる

.. sourcecode:: uwscr

    // 一括定義
    hash foobar
        'foo' = 1 // キー = 値形式で記述
        bar   = 2 // キーは文字列でなくても良い
    endhash
    // 以下と同じ
    // hashtbl foobar
    // foobar['foo'] = 1
    // foobar['bar'] = 2

    // グローバル変数にする
    hash public pub
    endhash
    // 以下と同じ
    // public hashtbl pub

    // オプション指定
    hash with_option = HASH_CASECARE or HASH_SORT
    endhash
    // 以下と同じ
    // hashtbl with_option = HASH_CASECARE or HASH_SORT

enum
----

| 列挙体を定義します
| グローバルスコープの定数として定義されます

.. code-block::

    // 定義
    enum 定数名
        メンバ名
        メンバ名 [ = 数値]
    endenum

    // 呼び出し
    定数名.メンバ名

| メンバには上から順に数値が割り当てられます (0から)
| ``メンバ名 = 数値`` とすることで任意の値を割り当てられます
| ただし前のメンバより大きな値のみ有効です

.. sourcecode:: uwscr

    // 0から順に割り当てられる
    enum E
        foo // 0
        bar // 1
        baz // 2
    endenum

    // 呼び出しは定数名.メンバ名
    print E.foo // 0
    print E.bar // 1
    print E.baz // 2

    // 数値を指定
    enum E
        foo = 10 // 10
        bar = 20 // 20
    endenum

    // 一箇所指定するとそれ以降はその値から加算されていく
    enum E
        foo = 10 // 10
        bar      // 11 (上の10 に +1される)
        baz      // 12
    endenum

    // 途中も可
    enum E
        foo      // 0
        bar = 10 // 10
        baz      // 11
    endenum
    enum E
        foo = 100    // 100
        bar          // 101
        baz = 200    // 200
        qux          // 201
    endenum

    // 以下はNG

    enum E
        foo
        foo // 同じ名前はダメ
    endenum

    // 前の数値より大きくないとダメ
    enum E
        foo // 0
        bar // 1
        baz = 1 // 2以上じゃないとダメ
    endenum
    enum E
        foo = 50
        bar = 1 // 51以上じゃないとダメ
    endenum


関数定義
------------------------------


| 関数名には英数字、一部記号、全角文字列が使えます
| 英字の大文字小文字は区別しません


procedure
^^^^^^^^^
function
^^^^^^^^


.. code::

    procedure 関数名([引数, 引数, …])
        処理
    fend

    function 関数名([引数, 引数, …])
        [result = 戻り値]
    fend

.. describe:: procedure

    戻り値がありません

.. describe:: function

    ``result`` 変数の値が戻り値となります

    .. object:: result (省略可)

        | 初期値は ``EMPTY`` です
        | 記述がない場合は ``EMPTY`` を返します

.. sourcecode:: uwscr

    hoge(1,2,3) // 6
    print fuga(1,2,3) // 6

    procedure hoge(a, b, c)
        print a + b + c
    fend
    function fuga(a, b, c)
        result = a + b + c
    fend

関数定義の入れ子はダメ

.. sourcecode:: uwscr

    // エラーになる
    procedure p()
        procedure q()
        fend
    fend

特殊な引数
++++++++++

参照渡し
~~~~~~~~

| 引数の前に ``var`` または ``ref`` キーワードをつけることで参照渡しが可能な引数になります
| 引数に変数を渡すとその変数に関数実行中の変更が反映されます
| 変数以外の式を渡した場合は通常の引数と同様に振る舞います

.. sourcecode:: uwscr

    a = 2
    print a // 2
    p(a)
    print a // 6
    q(a)
    print a // 16

    procedure p(ref r)
        r *= 3
    fend

    procedure q(var v)
        v += 10
    fend

配列表記
~~~~~~~~

| ``引数[]`` 形式で記述します
| 互換性のため表記自体はできますが、動作は通常の引数と同様です
| 受けられる引数を配列や連想配列に限定したい場合は :ref:`type_check` を使用してください

.. sourcecode:: uwscr

    // 以下は同じ意味です
    procedure p(arr[])
    procedure p(arr)

デフォルト値
~~~~~~~~~~~~

| ``引数 = 値`` とすることで引数のデフォルト値を指定できます
| 値を省略した場合は ``EMPTY`` がデフォルト値になります
| 呼び出し時に引数を渡さなかった場合デフォルト値が適用されます

.. sourcecode:: uwscr

    print f(2)    // 0
    print f(2, 3) // 6

    function f(n, m = 0)
        result = n * m
    fend

    // デフォルト値を省略した場合はEMPTYが入る
    procedure p(arg = )
        print arg == EMPTY // True
    fend

デフォルト値を持つ引数のあとに別の種類の引数は指定できません

.. sourcecode:: uwscr

    procedure p(a = 1, b = 2, c = 3) // ok
    fend
    procedure q(a = 1, b, c = 3)     // エラー
    fend
    procedure r(a, b = 2, c = 3)     // 前ならok
    fend

可変長引数
~~~~~~~~~~

| 引数の前に ``args`` または ``prms`` キーワードをつけることで可変長の引数を受けられるようになります
| 関数内ではその引数が配列になります
| 可変長引数は最後の引数でなくてはいけません

.. sourcecode:: uwscr

    print f(1)         // 1
    print f(1,2,3,4,5) // 5

    function f(args v)
        result = length(v)
    fend

可変長引数のあとに引数があるとエラーになります

.. sourcecode:: uwscr

    procedure p(prms a, b)    // エラー
    fend
    procedure q(a, b, prms c) // ok
    fend

特殊な引数の組み合わせ
~~~~~~~~~~~~~~~~~~~~~~

| 原則として組み合わせられません
| 配列表記の参照渡しのみOK

.. sourcecode:: uwscr

    procedure p(ref foo[]) // これはOK

    // こういうのはダメ
    procedure p(ref foo = 1) // 参照 + デフォルト値
    procedure p(ref params bar) // 参照 + 可変長
    procedure p(params bar = 1) // 可変長 + デフォルト値

.. _type_check:

引数の型チェック
++++++++++++++++

.. code-block::

    function 関数名(引数名: 型, var 引数名: 型, 引数名: 型 = デフォルト値)

| 通常の引数、参照渡し、デフォルト値を持つ引数であれば受ける型を指定できます
| 関数呼び出し時に指定した型が渡されなかった場合は実行時エラーになります

| 指定可能な型

    .. object:: string

        | 文字列

    .. object:: number

        | 数値

    .. object:: bool

        | 真偽値 (TRUE/FALSE)

    .. object:: array

        | 配列

    .. object:: hash

        | 連想配列

    .. object:: func

        | 関数 (ユーザー定義、無名関数)

    .. object:: uobject

        | UObject

    .. object:: クラス名

        | クラスオブジェクトのインスタンス

    .. object:: 列挙体名

        | 列挙体(enum)メンバの値 (該当する数値でも良い)


.. sourcecode:: uwscr

    function f(str: string)
        result = str
    fend

    print f("hoge") // OK
    print f(123)    // 数値なのでエラー

    // 列挙体名指定の場合
    enum Hoge
        foo
        bar
        baz
    endenum

    function f2(n: Hoge)
        select n
            case Hoge.foo
                result = 'foo!'
            case Hoge.bar
                result = 'bar!'
            case Hoge.baz
                result = 'baz!'
        selend
    fend

    print f2(Hoge.foo) // OK
    print f2("Hoge")   // 文字列はエラー
    print f2(0)        // OK ※Hoge.fooに一致するため
    print f2(100)      // Hogeに含まれない値なのでエラー

無名関数
^^^^^^^^

| 名前を持たない関数です
| 変数に代入して使えます


.. code::

    変数 = function([引数, ...])
        [result = 戻り値]
    fend
    変数 = procedure([引数, ...])
    fend

変数に関数を代入できます

.. sourcecode:: uwscr

    hoge = function(x, y)
        result = x + y
    fend

    print hoge(2, 3) // 5

無名関数の中でpublic/constを宣言した場合は実行時に初めて評価されます

.. sourcecode:: uwscr

    print x // エラー

    proc = procedure()
        public x = 5
    fend

    print x // エラー

    proc()

    print x // 5

通常の関数と同様に特殊な引数も定義できます

.. sourcecode:: uwscr

    f = function(a, b[], var c, d = 0)
    fend
    p = procedure(args e)
    fend

簡易関数式
^^^^^^^^^^

| 無名関数を単行の式で記述できます
| 通常の無名関数と異なり処理部に文は書けません(式のみ)
| その代わりに即時関数として利用できます

.. code::

    関数 = | 引数 [, 引数, …] => 式 [; 式; …] |

| 引数は,区切りで複数指定可能
| ``result`` は省略可能です

.. sourcecode:: uwscr

    func = | a, b => a + b |
    print func(1, 2) // 3

| 式は ``;`` 区切りで複数書けます
| この場合一番最後の式が戻り値となります

.. sourcecode:: uwscr

    func = | a, b => a *= 2; b *= 3; a + b |
    print func(1, 2) // 8

即時関数
++++++++

.. sourcecode:: uwscr

    print | n, m => n * m |(7, 6) // 42
    // 値だけ返す
    print |=> 42|() // 42

    // 関数の引数にする
    function f(fn)
        result = fn("world!")
    fend
    print f(| s => "hello " + s |) // hello world!

特殊な引数にも対応

.. sourcecode:: uwscr

    print | args a => length(a) |(1,2,3,4,5,6) // 6

関数の特殊な使用例
^^^^^^^^^^^^^^^^^^

高階関数
++++++++

関数の引数に関数を指定できます

.. sourcecode:: uwscr

    print Math(10, 5, Add)      // 15
    print Math(10, 5, Multiply) // 50

    subtract = function(n, m)
        result = n - m
    fend

    print Math(10, 5, subtract) // 5


    function Math(n, m, func)
        result = func(n, m)
    fend

    function Add(n, m)
        result = n + m
    fend

    function Multiply(n, m)
        result = n * m
    fend

クロージャ
++++++++++

関数の戻り値として関数(クロージャ)を返すことができます
クロージャは元の関数内での値を保持します

.. sourcecode:: uwscr

    hoge = test(5) // test関数内の変数nを5にする
    // 関数hogeはn=5を保持している
    print hoge(3)    // 8 (5+3が行われる)
    print hoge(7)    // 12 (5+7が行われる)
    print hoge("あ") // 5あ (5+'あ'が行われる)

    function test(n)
        result = function(m)
            result = n + m
        fend
    fend

エイリアス
++++++++++

関数を変数に代入することでその関数を別の名前で呼び出せるようになります

.. sourcecode:: uwscr

    function hoge(n)
        result = n
    fend

    h = hoge // 変数hにhoge関数を代入
    print h('hoge') // hoge

    // ビルトイン関数も代入できる
    mb = msgbox
    mb('ほげほげ')

module
^^^^^^

| 機能のモジュール化
| ``モジュール名.メンバ名`` で各機能を利用可能にします

.. code::

    module モジュール名
        const 定数名 = 式      // モジュール名.定数名 で外部からアクセス可
        public 変数名[ = 式]   // モジュール名.変数名 で外部からアクセス可
        dim 変数名[ = 式]      // 外部からアクセス不可
        procedure モジュール名 // コンストラクタ、module定義の評価直後に実行される
        procedure 関数名()     // モジュール名.関数名() で外部からアクセス可
        function 関数名()      // モジュール名.関数名() で外部からアクセス可
        textblock 定数名       // モジュール名.定数名 で外部からアクセス可
    endmodule


module関数内でのみ使える特殊な書式
++++++++++++++++++++++++++++++++++

.. object:: this

    自module内のメンバの呼び出しを明示する

.. object:: global

    グローバル変数および関数を呼び出す(ビルトイン含む)
    (本家と異なり変数や定数も可)

.. sourcecode:: uwscr

    module sample
        dim d = 1
        public p = 2
        const c = 3

        function f1()
            // 各メンバーには以下のようにアクセス可能
            print d
            print this.d
            print sample.d

            print p
            print this.p
            print sample.p

            print c
            print this.c
            print sample.c

            print f2()
            print this.f2()
            print sample.f2()
        fend

        function f2()
            result = 4
        fend

        function f3()
            print this.f4()   // in   メンバ関数が呼ばれる
            print global.f4() // out  module外の関数が呼ばれる
            print f4()        // in   メンバ関数が呼ばれる
        fend

        function f4()
            result = "in"
        fend
    endmodule

    function f4()
        result = "out"
    fend

プライベート関数
++++++++++++++++

無名関数を用いたプライベート関数の実装例

.. sourcecode:: uwscr

    Sample.Private() // エラー
    Sample.Func()    // OK

    module Sample
        function Func()
            result = Private()
        fend

        dim Private = function()
            result = "OK"
        fend
    endmodule

class
^^^^^

| classを定義します
| ``class名()`` を実行することによりインスタンスを作成します

.. caution::

    UWSCのclassとは互換性がありません

.. code::

    class class名
        procedure class名()    // コンストラクタ (必須)
        procedure _class名_()  // デストラクタ (オプション)
        const 定数名 = 式      // classインスタンス.定数名 で呼び出し可
        public 変数名[ = 式]   // classインスタンス.変数名 で呼び出し可
        dim 変数名[ = 式]      // class内からのみ呼び出し可
        procedure 関数名()     // classインスタンス.関数名() で呼び出し可
        function 関数名()      // classインスタンス.関数名() で呼び出し可
        textblock 定数名       // classインスタンス.定数名 で呼び出し可
    endclass

.. sourcecode:: uwscr

    h1 = hoge(3, 5)
    print h1.Total() // 8

    h2 = hoge(8, 10)
    print h2.Total() // 18

    print hoge(11, 22).Total() // 33

    class hoge
        dim a = 1, b = 2
        procedure hoge(a, b)
            this.a = a
            this.b = b
        fend
        function Total()
            result = this.a + this.b
        fend
    endclass

.. caution::

    moduleと異なりclass名から直接メンバにアクセスすることはできません

    .. sourcecode:: uwscr

        print hoge.p() // エラー

デストラクタ
++++++++++++

| デストラクタはインスタンスへの参照がなくなった際に実行される関数です
| ``_class名_()`` で命名された関数がデストラクタとして定義されます
| デストラクタに引数は指定できません

デストラクタが実行されるタイミング

- すべての参照が失われたとき
- いずれかのインスタンス変数に ``NOTHING`` を代入したとき (明示的に破棄する)
    - インスタンス変数は ``NOTHING`` になります
- ``with``に渡す式でインスタンスを作成した場合で ``endwith`` に到達したとき
- 関数スコープを抜ける際に削除されるこローカルスコープ変数だった場合
- スクリプト終了時に削除されるローカル・グローバル定数だった場合

.. sourcecode:: uwscr

    class Sample
        dim msg
        procedure Sample(msg)
            this.msg = msg
        fend
        procedure _Sample_()
            print msg
        fend
    endclass

    obj1 = Sample("すべての参照が失われた")
    obj2 = obj1
    obj3 = obj1

    obj1 = 1
    obj2 = 1
    obj3 = 1 // すべての参照が失われた がprintされる

    obj1 = Sample("NOTHINGが代入された")
    obj2 = obj1
    obj3 = obj1

    obj1 = NOTHING // NOTHINGが代入された がprintされる
    print obj1 // NOTHING
    print obj2 // NOTHING
    print obj3 // NOTHING

    with Sample("withを抜けた")
    endwith // withを抜けた がprintされる

    procedure p()
        obj = Sample("関数スコープを抜けた")
    fend

    p() // 関数スコープを抜けた がprintされる

UObject
-------

| json互換のオブジェクト

オブジェクトの作成
^^^^^^^^^^^^^^^^^^

1. UObjectリテラル: jsonを ``@`` で括る
2. :any:`FromJson` 関数

.. sourcecode:: uwscr

    obj = @{
       "foo": "fooooo",
       "bar": {
           "baz": true
       },
       "qux": [
           {"quux": 1},
           {"quux": 2},
           {"quux": 3}
       ]
   }@

   arr = @[1, 2, 3]@

有効な値は

- 数値
- 文字列
- 真偽値
- NULL
- 配列
- オブジェクト

.. tip:: UObjectリテラル内での変数展開について

    | ``@`` で括られたjson部分は文字列として扱われます
    | これは展開可能文字列であるため ``"<#変数名>"`` が利用可能です

    .. sourcecode:: uwscr

        foo = '文字列を展開'
        bar = 123
        textblock baz
        ,
        "baz":{
            "qux": "jsonの一部を一気に書き込むことも可能"
        }
        endtextblock

        obj = @{
            "foo": "<#foo>",
            "bar": <#bar>
            <#baz>
        }@

        print obj.foo     // 文字列を展開
        print obj.bar     // 123
        print obj.baz.qux // jsonの一部を一気に書き込むことも可能

値の呼び出し、変更
^^^^^^^^^^^^^^^^^^

.. sourcecode:: uwscr

    print obj.foo // fooooo
    obj.foo = "FOOOOO"
    print obj.foo // FOOOOO
    print obj["foo"] // 配列の添字にしてもOK

    print obj.bar.baz ? "baz is true!": "baz is fasle!" // baz is true!

    obj.qux[1].quux = 5
    print obj.qux[1].quux // 5

    obj.qux[2] = "overwrite!"
    print obj.qux[2] // overwrite!

    obj.corge = 1 // エラー、追加はできない

    // オブジェクトを作って代入ならOK
    obj.foo = fromjson('{"hoge": 1, "fuga": 2}')
    print obj.foo


評価の順序
----------

| グローバル変数や定数、関数定義は実行より先に評価されます

1. public, const, textblockを記述順に評価
2. function, procedure, moduleを記述順に評価
    - 関数内で宣言されているpublicやconstも評価
3. 残りの構文を評価/実行する

スコープ
--------


スコープは大まかに分けると

- スクリプト本文
- 関数内

| という区分になっています
| 変数にはローカルとグローバルという区分があり、

- スクリプト本文のローカル変数はスクリプト本文内でしかアクセスできない
- 関数のローカル変数は関数内でしかアクセスできない
- グローバル変数はいずれからでもアクセスできる

という特徴があります

- ローカル
    - dim宣言した変数
        - 宣言省略した変数も含む
    - hashtbl宣言した連想配列
- グローバル
    - public宣言した変数
        - public hashtbl
    - const宣言した定数
    - 定義した関数 (変数ではないが扱いはグローバル)

.. sourcecode:: uwscr

    public global1 = "グローバル変数1"
    dim local = "本文ローカル"

    print global1 // ok
    print global2 // ok
    print local // ok
    print proc_local // ng
    print func() // ok

    procedure proc()
        public global2 = "グローバル変数2"
        dim proc_local = "関数ローカル"
        print global1 // ok
        print global2 // ok
        print local // ng
        print proc_local // ok
        print func() // ok
    fend

    function func()
        result = "関数"
    fend

無名関数のスコープ
^^^^^^^^^^^^^^^^^^

無名関数の中はスコープが分かれていません
ローカル変数がそのまま使えます

.. sourcecode:: uwscr

    dim local = 1
    dim func = function(n)
        result = local + n
    fend

    print func(1) // 2

moduleのスコープ
^^^^^^^^^^^^^^^^

| moduleメンバに関しては独自のスコープを持ちます
| module関数内で定義したpublic, const, function/procedureはグローバル空間には置かれず、
| moduleメンバのみがアクセスできるmoduleローカル空間に配置されます

これらは ``module名.メンバ名`` でアクセスできます

文字列
------


文字列リテラルは ``""`` または ``''`` で括ります
``"`` で括った文字列では特殊文字が展開されます
``'`` で括った文字列では特殊文字が展開されません

.. sourcecode:: uwscr

    str = "文字列"
    str = '文字列'

文字列の結合は ``+`` 演算子を使います

.. sourcecode:: uwscr

    str = "文字列" + "の" + "結合"
    print str // 文字列の結合

特殊文字の展開
^^^^^^^^^^^^^^

``""`` で括った文字列中にある以下の特殊文字は、それぞれ該当する別の文字に変換されます

- ``<#CR>``: 改行 (CRLF)
- ``<#TAB>``: タブ文字
- ``<#DBL>``: ダブルクォーテーション (``"``)
- ``<#変数名>``: 変数が存在する場合、その値


.. sourcecode:: uwscr

    print "hoge<#CR>fuga<#CR>piyo"
    // hoge
    // fuga
    // piyo
    print "hoge<#TAB>fuga<#TAB>piyo"
    // hoge    fuga    piyo
    print "<#DBL>hoge<#DBL>"
    // "hoge"

    dim a = 123
    print "a is <#a>"
    // a is 123
    print "b is <#b>" // 変数が存在しない場合は展開されない
    // b is <#b>
    print "length of a is <#length(a)>" // 式はダメ、変数のみ展開される
    // length of a is <#length(a)>

    print 'a is <#a>' // シングルクォーテーション文字列は展開しない
    // a is <#a>

ホワイトスペース
----------------

- 半角スペース
- タブ文字
- 全角スペース

| はホワイトスペース扱いです
| 式と式の区切りとして機能します
| 改行(CRLF、CR、LF)は行末扱いです

演算子
------

.. object:: +

        数値の加算、文字列の結合、配列要素の追加

.. object:: +=

        数値の加算、文字列の結合、配列要素の追加をして代入

.. object:: -

        数値の減算

.. object:: -=

        減算して代入

.. object:: *

        数値の乗算、文字列の繰り返し

.. object:: *=

        乗算して代入

.. object:: /

        数値の除算  ※ 0で割ると0を返す

.. object:: /=

        除算して代入

.. object:: mod

        数値の剰余演算 (割った余りを返す)

.. object:: !

        論理否定

.. object:: ? :

        三項演算子 b ? t : f

.. object:: :=

        代入 (代入した値を返す)

.. object:: =

        代入、等価演算

.. object:: ==

        等価演算

.. object:: <>
.. object:: !=

        不等価演算

.. object:: and

        数値のAND演算(ビット演算)

.. object:: or

        数値のOR演算(ビット演算)、真偽値の論理演算

.. object:: xor

        数値のXOR演算(ビット演算)

.. object:: andL

        論理演算 (両辺の真偽性評価を行う)

.. object:: orL

        論理演算 (両辺の真偽性評価を行う)

.. object:: xorL

        論理演算 (両辺の真偽性評価を行う)

.. object:: andB

        ビット演算 (両辺を数値とみなし評価を行う)

.. object:: orB

        ビット演算 (両辺を数値とみなし評価を行う)

.. object:: xorB

        ビット演算 (両辺を数値とみなし評価を行う)

.. object:: <

        小なり

.. object:: <=

        小なりイコール

.. object:: >

        大なり

.. object:: >=

        大なりイコール

.. object:: .

        moduleやオブジェクトのメンバへのアクセス


演算式の優先順位
^^^^^^^^^^^^^^^^

優先順位の高いものから先に演算を行います

1. ``( )`` 内の式
2. ``.``
3. ``!``
4. ``*`` ``/`` ``mod``
5. ``+`` ``-``
6. ``=``(等価比較) ``==`` ``<>`` ``!=``
7. ``and`` (L,Bを含む)
8. ``or`` ``xor`` (L,Bを含む)
9. ``? :`` (三項演算子)
10. ``:=``

代入系の演算子は順位判定とは別に代入処理判定を行っています

- 代入演算子
    - ``=``
- 複合代入演算子
    - ``+=``
    - ``-=``
    - ``*=``
    - ``/=``

.. sourcecode:: uwscr

    // 2つ目の = は代入ではなく比較になるので a にはboolが入る
    a = b = c

    // こういうのはダメ、演算中に代入はしない
    a + b + c += d

| 例外として ``:=`` による代入があります
| ``:=`` による代入は式であり、変数に代入された値を返します

.. sourcecode:: uwscr

    print n := 1               // 1 (代入した値が返る)
    print n                    // 1
    print 1 + 2 + (n := 3) + 4 // 10 (代入した値が返り、その値で計算が行われる)
    print n                    // 3

    // 一度に複数の変数に値を代入することもできる
    a = b := c := 10
    print a // 10
    print b // 10
    print c // 10
    // a := b := c := 10 でも可

特殊な演算
^^^^^^^^^^

| 数値以外を含む演算には一部特殊な仕様があります
| 型に対して不適切な演算子が用いられた場合はエラーになります

.. object:: 数値 + 文字列

    | 右辺の文字列が数値変換可能な場合は数値にします

        .. sourcecode:: uwscr

            print 1 + '2' // 3

    | 右辺の文字列が数値変換できない場合は左辺の数値を文字列にします

        .. sourcecode:: uwscr

            print 1 + 'a' // 1a

.. object:: 数値とEMPTYの演算

    | EMPTYは0として扱われます

    .. sourcecode:: uwscr

        print 3 * EMPTY // 0

.. object:: 数値と真偽値の演算

    | TRUEは1、FALSEは0として扱われます

    .. sourcecode:: uwscr

        print 3 + TRUE // 4

.. object:: 文字列 + 数値

    | 右辺の数値を文字列にします

        .. sourcecode:: uwscr

            print 'a' + 3 // a3
            print '1' + 2 // 12

.. object:: 文字列 * 数値

    | 左辺の文字列が数値変換可能な場合数値にします

        .. sourcecode:: uwscr

            print '2' * 3 // 6
            print '123' * 2 // 246

    | 左辺の文字列が数値に変換できない場合、文字列を数値分繰り返します

        .. sourcecode:: uwscr

            print 'a' * 3 // aaa
            print 'xyz' * 3 // xyzxyzxyz

.. object:: 文字列と数値の演算 (+, * 以外)

    | 左辺の文字列が数値変換可能な場合は数値にします

        .. sourcecode:: uwscr

            print '15' / 3 // 5

    | 左辺の文字列が数値変換できない場合はエラーになります

        .. sourcecode:: uwscr

            print 'a' / 3 // エラー

.. object:: 文字列 + NULL

    | null文字(chr(0))を付け加えます

    .. sourcecode:: uwscr

        hoge = "HOGE" + NULL
        print hoge         // HOGE
        print length(hoge) // 5

.. object:: 文字列 + その他の値

    | 上記例以外の値型はすべて文字列として扱われます

        .. sourcecode:: uwscr

            'a' + TRUE // aTrue

.. object:: 配列 + 値

    | 配列の末尾に値を追加します

        .. sourcecode:: uwscr

            print [1,2,3] + 4 // [1,2,3,4]

.. object:: NULL * 数値

    | 数値分連続したnull文字を返します

        .. sourcecode:: uwscr

            hoge = NULL * 5
            print hoge         // (なにも表示されない)
            print length(hoge) // 5

.. object:: RemoteObject

    | :ref:`remote_object` が以下の型であった場合、通常の値型と同じように演算が行なえます

    - 文字列
    - 数値
    - 真偽値
    - NULL

    .. sourcecode:: uwscr

        elements = browser[0].document.querySelectorAll(selector)
        // lengthプロパティはRemoteObjectだが、数値との比較が可能
        if elements.length > 0 then
            // innerTextプロパティは文字列型のRemoteObjectを返すがEMPTYを足すことで通常の文字列にできる
            innerText = elements[0].innerText + EMPTY
            // ConvertFromRemoteObject関数で通常の型に変換することもできる
            // innerText = ConvertFromRemoteObject(elements[0].innerText)
        endif

三項演算子
^^^^^^^^^^

.. code-block:: none

    式 ? 真で返す式 : 偽で返す式

| 式を評価しその真偽により値を返します
| 単行のIF文に似ていますが、三項演算子は値を返します
| また、IF文とは異なり文を書くことができません

.. sourcecode:: uwscr

    a = FALSE
    print a ? "a is TRUE": "a is FALSE" // a is FALSE

    // 入れ子もできる

    // fizzbuzz
    for i = 1 to 100
        print i mod 15 ? i mod 5 ? i mod 3 ? i : "fizz" : "buzz" : "fizzbuzz"
    next

    // 三項演算子では中に式しか書けない
    // 例: print文を書いた場合
    hoge ? print "hoge is truthy" : print "hoge is falsy" // エラー

ビット演算子、論理演算子
^^^^^^^^^^^^^^^^^^^^^^^^

AND、OR、XORは両辺の値型により論理演算またはビット演算のいずれかを行っていました
UWSCRでは演算子が追加され論理演算およびビット演算を明示的に行うことができます

論理演算子
++++++++++

.. object:: AndL, OrL, XorL

    | 真偽値を返します
    | 両辺に不適切な値型が含まれる場合はエラーになります

    .. sourcecode:: uwscr

        // 両辺の真偽性を評価してから演算を行う
        print true andl false // false
        print true andl NOTHING // false
        print NULL andl 'a' // true
        print 1 xorl [1,2] // false

ビット演算子
++++++++++++

.. object:: AndB, OrB, XorB

    | 数値を返します
    | 両辺に不適切な値型が含まれる場合はエラーになります

    .. sourcecode:: uwscr

        // 両辺を数値として評価してから演算を行う
        print 3 andb 5 // 1
        print 3 orb 5 // 7
        print 3 xorb 5 // 6
        print 1 andb '1' // 1
        print 1 andb true // 1

真偽判定
--------

真偽性の評価が行われる場合(ifの条件式など)に

- FALSE
- EMPTY
- 0
- NOTHING
- 長さ0の文字列
- 長さ0の配列

| は偽となります
| それ以外は真です

.. sourcecode:: uwscr

    print NOTHING ? '真' : '偽' // 偽
    print "" ? '真' : '偽' // 偽
    print "空ではない文字列" ? '真' : '偽' // 真
    print [1,2,3] ? '真' : '偽' // 真
    print [] ? '真' : '偽' // 偽

コメント
--------

| ``//`` 以降は行末までコメントです (構文解析されない)
| ``//`` があった時点で行末扱いになります

.. sourcecode:: uwscr

    a = 1
    // a = a + 1
    print a // 1 が出力される

行結合
------

行末に ``_`` を記述することで次の行と結合させます

.. sourcecode:: uwscr

    a = 1 + 2 + _
    3 + 4

    print a // 10

マルチステートメント
--------------------

``;`` をつけることで複数の文を1行に記述できます

.. sourcecode:: uwscr

    a = 1; a = a + 1; print a // 2


組み込み定数
------------

.. list-table::
    :align: left

    * - `TRUE`
      - true または 1
    * - `FALSE`
      - false または 0
    * - `NULL`
      - 振る舞い未実装
    * - `EMPTY`
      - 空文字
    * - `NOTHING`
      - オブジェクトがない状態
    * - `NaN`
      - Not a number

NaNについて
^^^^^^^^^^^

| ``NaN`` は ``NaN`` 自身を含めあらゆる値と等価ではありません
| また ``NaN`` との大小の比較結果も必ず偽です

.. sourcecode:: uwscr

    print NaN == NaN // False
    print n   == NaN // False (nは何かしらの値)
    print NaN != NaN // True
    print NaN <  n   // False
    print NaN <= n   // False
    print NaN >  n   // False
    print NaN >= n   // False

16進数
------

16進数リテラル表記は ``$`` を使います

.. sourcecode:: uwscr

    print $FF // 255

起動時パラメータ
----------------

スクリプトにパラメータを付与した場合にそれらが ``PARAM_STR[]`` に格納されます


.. sourcecode:: shell

    uwscr hoge.uws foo bar baz

.. sourcecode:: uwscr

    // hoge.uws
    for p in PARAM_STR
        print p
    next

.. sourcecode:: shell

    # 結果
    foo
    bar
    baz

OPTION
------

.. code::

    OPTION 設定名[=値]

bool値指定は省略可能で、省略時はtrueになります

.. sourcecode:: uwscr

     OPTION EXPLICIT // explicit設定をtrueにする

もともと設定がtrueになっているものをfalseに設定したい場合に値指定を使う

.. sourcecode:: uwscr

    OPTION SHORTCIRCUIT=FALSE // デフォルトtrueなのでfalseにする

.. object:: OPTION EXPLICIT[=bool]

    | ローカル変数初期化時にdim宣言が必須かどうか (初期値:false)

.. object:: OPTION SAMESTR[=bool]

    .. caution::

        現時点では無視されます

    | 文字列の比較等で大文字小文字を区別するかどうか (初期値:false)

.. object:: OPTION OPTPUBLIC[=bool]

    .. caution::

        現時点では無視されます

    | public変数の重複定義を禁止するかどうか (初期値:false)

.. object:: OPTION OPTFINALLY[=bool]

    | tryで強制終了時にfinally部を実行するかどうか (初期値:false)

.. object:: OPTION SPECIALCHAR[=bool]

    .. caution::

        | この設定は無視されます
        | ``'文字列'`` 表記で特殊文字が展開されなくなります

.. object:: OPTION SHORTCIRCUIT[=bool]

    .. caution::

        現時点では無視されます

    | 論理演算で短絡評価を行うかどうか (初期値:true)

.. object:: OPTION NOSTOPHOTKEY[=bool]

    .. attention::

        この設定は無効です

.. object:: OPTION TOPSTOPFORM[=bool]

    .. attention::

        この設定は無効です

.. object:: OPTION FIXBALLOON[=bool]

    .. caution::

        現時点では無視されます

    | 吹き出しを仮想デスクトップにも表示するかどうか (初期値:false)

.. object:: OPTION DEFAULTFONT="name,n"

    | ダイアログ等のフォント指定  (初期値:"MS Gothic,12")

.. object:: OPTION POSITION=x,y

    .. caution::

        現時点では無視されます

    | メインGUIの座標指定 (初期値:0, 0)

.. object:: OPTION LOGPATH="path"

    | ログ保存フォルダを指定 (初期値:スクリプトのあるフォルダ)
    | 存在するディレクトリを指定するとそこに ``uwscr.log`` を出力します
    | それ以外はログファイルのパスとして扱われます

.. object:: OPTION LOGLINES=n

    .. caution::

        現時点では無視されます

    | ログファイルの最大行数を指定 (初期値:400)

.. object:: OPTION LOGFILE=n

    | ログファイルの出力方法 ※3 (初期値:1)
    | 設定値は以下の通り、2～4を指定することでログが出力されます
    | 1: ログ出力なし
    | 2: ログを出力あり
    | 3: ログを出力あり
    | 4: ログを出力あり
    | それ以外: ログ出力なし

.. object:: OPTION DLGTITLE="title"

    | ダイアログのタイトルを指定します (初期値:"UWSCR - スクリプト名")


def_dll
-------

| DLL関数 (Win32 APIなど) を呼び出せるようにします
| 32bit版UWSCRでは32bitのDLL、64bit版では64bitのDLLに対応します

.. code::

    def_dll 関数名(型, 型, ...): 戻り型: DLLパス
    def_dll 関数名(型, 型, ...): DLLパス // 戻り値がvoidの場合省略できる

型定義について
^^^^^^^^^^^^^^

| 以下の型を指定できます
| 一部の型はx86/x64でサイズが変わります
| 一部の型は引数定義、または戻り値定義でのみ指定可能です

+---------------------+--------+------------------------------+--------------------+------+--------+-----------------------------+
|        型名         | サイズ |             詳細             |    対応する値型    | 引数 | 戻り値 |            備考             |
+=====================+========+==============================+====================+======+========+=============================+
| int, long, bool     | 4      | 符号あり32ビット整数         | 数値               | ☑    | ☑      |                             |
+---------------------+--------+------------------------------+--------------------+------+--------+-----------------------------+
| uint, dword         | 4      | 符号なし32ビット整数         | 数値               | ☑    | ☑      |                             |
+---------------------+--------+------------------------------+--------------------+------+--------+-----------------------------+
| hwnd                | 4 / 8  | ウィンドウハンドル           | 数値               | ☑    | ☑      |                             |
+---------------------+--------+------------------------------+--------------------+------+--------+-----------------------------+
| float               | 4      | 単精度浮動小数点数           | 数値(小数)         | ☑    | ☑      |                             |
+---------------------+--------+------------------------------+--------------------+------+--------+-----------------------------+
| double              | 8      | 倍精度浮動小数点数           | 数値(小数)         | ☑    | ☑      | UWSCRの標準数値型           |
+---------------------+--------+------------------------------+--------------------+------+--------+-----------------------------+
| word, wchar         | 2      | 符号なし16ビット整数         | 数値               | ☑    | ☑      |                             |
+---------------------+--------+------------------------------+--------------------+------+--------+-----------------------------+
| byte, char, boolean | 1      | 符号なし8ビット整数          | 数値               | ☑    | ☑      |                             |
+---------------------+--------+------------------------------+--------------------+------+--------+-----------------------------+
| longlong            | 8      | 符号あり64ビット整数         | 数値               | ☑    | ☑      |                             |
+---------------------+--------+------------------------------+--------------------+------+--------+-----------------------------+
| string              | 4 / 8  | ANSI文字列のポインタ         | 文字列, NULL       | ☑    |        | OSの現在のANSIコードによる  |
+---------------------+--------+------------------------------+--------------------+------+--------+-----------------------------+
| pchar               | 4 / 8  | ANSI文字列のポインタ         | 文字列, NULL       | ☑    |        | NULL含む                    |
+---------------------+--------+------------------------------+--------------------+------+--------+-----------------------------+
| wstring             | 4 / 8  | ワイド文字列のポインタ       | 文字列, NULL       | ☑    |        | Unicode文字列               |
+---------------------+--------+------------------------------+--------------------+------+--------+-----------------------------+
| pwchar              | 4 / 8  | ワイド文字列のポインタ       | 文字列, NULL       | ☑    |        | NULL含む                    |
+---------------------+--------+------------------------------+--------------------+------+--------+-----------------------------+
| pointer             | 4 / 8  | ポインタを示す数値(符号なし) | 数値               | ☑    |        | 新規                        |
+---------------------+--------+------------------------------+--------------------+------+--------+-----------------------------+
| struct              | 4 / 8  | ユーザー定義構造体のポインタ | 構造体インスタンス | ☑    |        | 新規 struct-endstructで定義 |
+---------------------+--------+------------------------------+--------------------+------+--------+-----------------------------+
| callback            | 4 / 8  | コールバック関数のポインタ   |                    |      |        | 新規 **動作未定義**         |
+---------------------+--------+------------------------------+--------------------+------+--------+-----------------------------+
| safearray           |        | SafeArray型                  | SafeArray          |      |        | **動作未定義**              |
+---------------------+--------+------------------------------+--------------------+------+--------+-----------------------------+
| void                |        | 型がないことを示す           |                    |      | ☑      |                             |
+---------------------+--------+------------------------------+--------------------+------+--------+-----------------------------+

特殊な型定義
++++++++++++

参照渡し
~~~~~~~~

| ``var 型`` で参照渡しになります
| DLL関数により引数が書き換えられる場合に ``var`` を付与することにより変更後の値を渡した変数で受けられる

.. attention::

    varを付与してもポインタが渡されるわけではありません

配列の定義
~~~~~~~~~~

| ``型[]`` で配列変数を受ける型になります
| ``var`` の有無に関わらずDLL関数により値が変更される場合があります

.. note::

    配列変数はポインタになるためサイズは4または8です

構造体の定義
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

``{型, 型, ...}`` で構造体を定義します
構造体定義内では`var`の有無に関わらずDLL関数により値が変更される場合があります
構造体定義はポインタになるためサイズは4または8です

構造体定義のネストはできません
平の定義に展開されます

.. sourcecode:: uwscr

    def_dll hoge({long, {long, long}}):hoge:dll
    // ↑同じ↓
    def_dll hoge({long, long, long}):hoge:dll


DLL関数定義およびその呼び出し方の例
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

.. sourcecode:: uwscr

    // Win32のA関数ではstringかpcharを使う
    def_dll MessageBoxA(hwnd, string, pchar, uint):int:user32.dll
    // Win32のW関数ではwstringかpwcharを使う
    def_dll MessageBoxW(hwnd, wstring, pwchar, uint):int:user32.dll

    // 呼び出す際は単に文字列を渡すだけで良い
    print MessageBoxA(0, 'メッセージ', 'タイトル', 0)
    print MessageBoxW(0, 'メッセージ', 'タイトル', 0)

    // 構造体定義は{}
    def_dll SetWindowPlacement(hwnd, {uint, uint, uint, long, long, long, long, long, long, long, long}):bool:user32.dll
    id = getid("メモ帳")
    h = idtohnd(id)
    // 構造体を渡すときは定義した型の数だけ値を並べる
    SetWindowPlacement(h, 44, 0, 1, 0, 0, 0, 0, 200, 200, 600, 600)

    // 参照渡し
    def_dll GetPrivateProfileStringA(string, string, string, var pchar, dword, string):dword:kernel32
    buffer = NULL * 100
    // bufferがpcharなのでNULLを含んだ文字列が返ってくる
    print GetPrivateProfileStringA(NULL, NULL, "default", buffer, length(buffer), "test.ini")
    print buffer
    def_dll GetPrivateProfileStringA(string, string, string, var string, dword, string):dword:kernel32
    buffer = NULL * 100
    // bufferをstringにすると最初のNULL以前の文字列のみ返ってくる
    print GetPrivateProfileStringA(NULL, NULL, "default", buffer, length(buffer), "test.ini")
    print buffer

    // 構造体で値を受ける
    // varは不要
    def_dll GetCursorPos({long, long}):bool:user32.dll
    dim x, y
    print GetCursorPos(x, y)
    print [x, y]

    // 構造体はそのサイズに合う配列でも代用可能
    // varは不要
    def_dll GetCursorPos(long[]):bool:user32.dll
    dim point = [0, 0] // long, long
    print GetCursorPos(point)
    print point

    // 戻り値の型の制限
    def_dll PathAddBackslashW(var string):string:ShlwApi.dll
    path = "d:\hoge"
    print PathAddBackslashW(path)
    print path
    // このような場合UWSCでは戻り値に文字列が帰りましたが、UWSCRでは安全のためポインタ(の数値)を返します
    // その際警告文が標準出力されます

構造体定義
----------

def_dllのstruct型に渡す構造体を定義します

.. code::

    struct 構造体名
        メンバ名: 型
        ︙
    endstruct

``型`` はdef_dll定義で使うものと同等です(一部使用不可※)

| ``構造体名()`` で構造体のインスタンスを作成します
| インスタンス作成時に数値型は0、文字列型はNULLで初期化されます

.. sourcecode:: uwscr

    struct Point
        x: long
        y: long
    endstruct

    dim p = Point()
    // xとyは0で初期化されるため以下のような初期化処理は記述不要
    // p.x = 0
    // p.y = 0

    def_dll GetCursorPos(struct):bool:user32.dll
    GetCursorPos(p)
    print [p.x, p.y]

※ 利用可能な型

+---------------------+--------+------------------------------+--------------+
|        型名         | サイズ |             詳細             | 対応する値型 |
+=====================+========+==============================+==============+
| int, long, bool     | 4      | 符号あり32ビット整数         | 数値         |
+---------------------+--------+------------------------------+--------------+
| uint, dword         | 4      | 符号なし32ビット整数         | 数値         |
+---------------------+--------+------------------------------+--------------+
| hwnd                | 4 / 8  | ウィンドウハンドル           | 数値         |
+---------------------+--------+------------------------------+--------------+
| float               | 4      | 単精度浮動小数点数           | 数値(小数)   |
+---------------------+--------+------------------------------+--------------+
| double              | 8      | 倍精度浮動小数点数           | 数値(小数)   |
+---------------------+--------+------------------------------+--------------+
| word, wchar         | 2      | 符号なし16ビット整数         | 数値         |
+---------------------+--------+------------------------------+--------------+
| byte, char, boolean | 1      | 符号なし8ビット整数          | 数値         |
+---------------------+--------+------------------------------+--------------+
| longlong            | 8      | 符号あり64ビット整数         | 数値         |
+---------------------+--------+------------------------------+--------------+
| string              | 4 / 8  | ANSI文字列のポインタ         | 文字列, NULL |
+---------------------+--------+------------------------------+--------------+
| pchar               | 4 / 8  | ANSI文字列のポインタ         | 文字列, NULL |
+---------------------+--------+------------------------------+--------------+
| wstring             | 4 / 8  | ワイド文字列のポインタ       | 文字列, NULL |
+---------------------+--------+------------------------------+--------------+
| pwchar              | 4 / 8  | ワイド文字列のポインタ       | 文字列, NULL |
+---------------------+--------+------------------------------+--------------+
| pointer             | 4 / 8  | ポインタを示す数値(符号なし) | 数値         |
+---------------------+--------+------------------------------+--------------+

ポインタから構造体インスタンスを作る
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

DLL関数が返す構造体のポインタからインスタンスを作り値を受けることができます

.. sourcecode:: uwscr

    def_dll somefunc(var pointer):void:some.dll

    // somefuncが返す構造体を定義
    struct SomeStruct
        foo: long
        bar: long
        baz: long
    endstruct

    dim p
    somefunc(p) // ポインタを受け取る
    s = SomeStruct(p) // ポインタから構造体インスタンスを作成
    print s.foo


スレッド
--------

thread
^^^^^^


関数を別のスレッドで実行します

.. sourcecode:: uwscr

    thread func()

- スレッドスコープで実行されます
    - (その中でさらに関数スコープに入ります)
- グローバルスコープへのアクセスは可能
    - public, const, function/procedure, module/class
- 呼び出した関数内でエラーが発生した場合スクリプトが終了します

.. _task_object:

タスク
------

| 関数を非同期実行します
| threadとは異なり関数が完了し次第戻り値を受け取れます

- タスク関数
    - :any:`Task`
    - :any:`WaitTask`
- 構文
    - :ref:`async`
    - :ref:`await`

.. _async:

async
^^^^^

タスクを返す関数を宣言します

.. sourcecode::

    // function宣言の前に async キーワードを付与
    async function 関数名()
    fend


.. sourcecode:: uwscr

    async function MyFuncAsync(n)
        sleep(n)
        result = "<#n>秒待ちました"
    fend

    task = MyFuncAsync(5) // resultの値ではなくタスクを返す

    // 以下と同じ結果になります
    function MyFuncAsync(n)
        sleep(n)
        result = "<#n>秒待ちました"
    fend

    task = Task(MyFuncAsync, 5)

.. _await:

await
^^^^^

async宣言した関数の終了を待ち、resultの値を得ます

.. sourcecode:: uwscr

    async function MyFuncAsync(n)
        sleep(n)
        result = "<#n>秒待ちました"
    fend

    // MyFuncAsync()の処理が終了するまで待つ
    print await MyFuncAsync(5) // 5秒待ちました


with
----

``.`` 演算子の左辺(module名やオブジェクト)を省略できます

.. sourcecode:: uwscr

    module foo
        public bar = 'bar'
        procedure baz()
        fend
    endmodule

    with foo
        print .bar // foo.bar
        .baz()     // foo.baz()
    endwith

    // ネストも可

    module m
        public p = "m.p"
        function f()
            result = m2
        fend
    endmodule

    module m2
        public p = "m2.p"
    endmodule

    with m
        print .p // m.p
        with .f() // m.f() のwithでネスト
            print .p // m2.p
        endwith
        print .p // m.p
    endwith


textblock
---------

| 複数行文字列の定数を定義します
| textblock内での改行は ``<#CR>`` と同様です
| 特殊文字(``<#CR>``, ``<#DBL>``, ``<#TAB>``)はtextblock文の評価時に展開されます

.. code::

    textblock [定数名]
    (複数行文字列)
    endtextblock

| 定数名が省略された場合は複数行コメントとなり、スクリプトの一部として扱われません
| (構文木が作られない)

.. sourcecode:: uwscr

    // 定数hogeが作られる
    textblock hoge
    foo
    bar
    baz
    endtextblock

    // 定数省略時はコメント扱い
    // 値を呼び出すことができない
    textblock
    ここはコメントです
    endtextblock

textblockex
^^^^^^^^^^^

| 変数展開が可能なtextblockです
| textblockex変数の評価時に展開されます

.. sourcecode:: uwscr

    textblockex hoge
    <#fuga>
    endtextblock

    fuga = 123
    print hoge // 123
    fuga = 456
    print hoge // 456

call
----

| 他のスクリプトを取り込みます

.. sourcecode:: uwscr

    call hoge.uws          // 実行するスクリプトからの相対パス
    call hoge              // 拡張子のないファイルもOK、見つからない場合は.uwsを付けて開く
    call fuga.uws(1, 2, 3) // 引数を渡すと PARAM_STR にはboolが入る

    // urlから読み込み
    call url[https://example.com/hoge.uws]        // url[ ] の中でurlを指定
    call url[https://example.com/hoge.uws](1,2,3) // url[ ] の後に()をつけて引数を渡せる

- グローバル定義はスクリプト実行前に処理されます
    - public
    - const
    - textblock
    - function
    - procedure
    - module
    - class
- それ以外の処理部分はcall文が呼ばれる際に実行されます
    - 呼び出し元とは異なるスコープで実行されます
    - 呼び出し元の ``PARAM_STR`` にはアクセスできません (独自の ``PARAM_STR`` を持つため)

uwslファイルの読み込み
^^^^^^^^^^^^^^^^^^^^^^

| uwslファイルをcallして使えます

.. sourcecode:: uwscr

    call mylib.uwsl // 拡張子はuwslのみ (省略不可)

uwslファイルについて
^^^^^^^^^^^^^^^^^^^^

| 構文木をバイナリとして保存したものです
| 以下のコマンドでバイナリファイルを生成できます
| ファイルはスクリプトと同じディレクトリに作成されます
| 拡張子は ``.uwsl`` になります

.. code:: powershell

    uwscr --lib path\to\module.uws # module.uwsl が出力される

callでの呼び出しにのみ対応しており、直接実行することはできません

.. code:: powershell

    uwscr module.uwsl // ng

uwslファイル作成の流れ
++++++++++++++++++++++

1. 指定されたスクリプトを読み出す
2. 構文解析を行い構文木を生成する
3. 構文木をバイナリデータとしてファイルに書き出す

使用例
++++++

1. 多段callしているファイルをまとめてバイナリ化

    ファイル構成例

    - mylib.uws (module1 ~ 3 をcall)
        - module1.uws
        - module2.uws (submodule1, 2 をcall)
            - submodule1.uws
            - submodule2.uws
        - module3.uws

    .. code:: powershell

        uwscr -l mylib.uws # mylib.uwslが出力される

2. uwslファイルをcallして使う

    .. sourcecode:: uwscr

         call mylib.uwsl

         MyLib.DoSomething()
         Module1.DoSomethingElse()
         Module2.DoSomethingWithSubmodule(Submodule1.DoSomething)

例外処理
--------

- `try-except-endtry`
- `try-finally-endtry`
- `try-except-finally-endtry`

| try部で発生した実行エラーを抑制し、以下の特殊変数にエラー情報を格納します

- `TRY_ERRMSG`: エラーメッセージ
- `TRY_ERRLINE`: エラー行

| except部はtryでエラーが発生した場合のみ実行されます
| finally部は必ず実行されます
| finally部では ``continue``, ``break``, ``exit`` が使えません (構文解析エラーになる)

| ``try-except-finally-endtry`` は

.. sourcecode:: uwscr

    try
        try
        except
        endtry
    finally
    endtry

と同等です

except例
^^^^^^^^

.. sourcecode:: uwscr

    try
        print 1
        raise("エラー") // ここでエラー
        print 2 // 実行されない
    except
        print TRY_ERRMSG // 実行される
    endtry

    try
        // エラーが発生しない場合
    except
        print 1 // 実行されない
    endtry

finally例
^^^^^^^^^

.. sourcecode:: uwscr

    try
        print 1
        raise("エラー") // ここでエラー
        print 2 // 実行されない
    finally
        print TRY_ERRMSG // 実行される
    endtry

    try
        // エラーが発生しない場合
    finally
        print 1 // 実行される
    endtry

except-finally例
^^^^^^^^^^^^^^^^

.. sourcecode:: uwscr

    try
        print 1
        raise("エラー") // ここでエラー
        print 2 // 実行されない
    except
        print TRY_ERRMSG // 実行される
    finally
        print TRY_ERRMSG // 実行される
    endtry

    try
        // エラーが発生しない場合
    except
        print 1 // 実行されない
    finally
        print 2 // 実行される
    endtry

制御文
------

説明文中の ``式`` とは主に値を返す演算式や関数など
``文`` は制御文のことです
``ブロック文``は ``文`` が複数行ある状態です

if
^^^

.. note::

    | ``if`` と ``ifb`` が区別されません
    | どちらも同じものとして扱われます

単行if
++++++

.. code::

    if 式 then 文 [else 文]

.. sourcecode:: uwscr

    if foo then bar // foo が真の場合 bar が実行され、偽の場合なにもしない
    if foo then bar else baz// foo が真の場合 bar、偽の場合 baz が実行される

    // UWSCとは異なり ifb でもエラーにはならない
    ifb foo then bar

複数行if
++++++++

.. code::

    if 式 [then]
        ブロック文
    [elseif 式 [then]]
        ブロック文
    [else 式]
        ブロック文
    endif

.. note::

    ``elseif`` は複数回記述できる

.. sourcecode:: uwscr

    if foo then
        // fooが真なら実行され偽ならなにもしない
    endif

    if foo then
        // fooが真なら実行される
    else
        // fooが偽なら実行される
    endif


    if foo then
        // fooが真なら実行される
    elseif bar then
        // fooが偽かつbarが真なら実行される
    elseif baz then
        // fooが偽かつbazが真なら実行される
    else
        // foobarbazいずれも偽なら実行される
    endif

for
^^^

.. code::

    for 変数 = 式1 to 式2 [step 式3]
        ブロック文
    next

`式1` ～ `式3` はいずれも数値を返す必要があります
`step 式3` が省略された場合 `step 1` として扱われます
小数が渡された場合は整数に丸められます (UWSCとは仕様が異なります)

1. `変数` に `式1` を代入した状態で `ブロック文` を処理
2. `変数` の値に `式3` を加算したものを再代入し `ブロック文` を処理
3. `変数` に `式2` を超える値が代入されたら終了
4. 終了後も変数の値は維持されます

.. sourcecode:: uwscr

    for i = 0 to 2
          print i // 順に 0 1 2 が出力される
      next
      print i // 3

      for i = 0 to 5 step 2
          print i // 順に 0 2 4 が出力される
      next
      print i // 6

      // stepは減算も可能
      for i = 5 to 0 step -1
          print i
      next

      // ループ変数に代入した場合
      for i = 0 to 0
          print i // 0
          i = 10
          print i // 10
      next
      print i     // 1

      // UWSCでは小数が利用可能でしたがUWSCRでは整数値に変換されます
      for i = 0.1 to 1.9 step 0.1 // 0.1 -> 0, 1.9 -> 2 に丸められます
      next

for-in
^^^^^^

.. code::

    for 変数 in 式
        ブロック文
    next


| `式` は以下を返す必要があります

- 配列
- 連想配列
- 文字列
- COMのコレクション
- Iteratorを実装するRemoteObject

| `式` が返す値をその種類に応じて分解し `変数` に代入していきます

.. sourcecode:: uwscr

    // 文字列は1文字ずつ分解
    for char in "あいうえお"
        print char // あ い う え お が順に出力される
    next

    // 配列は各要素
    for value in ["あ", "い", "う", "え", "お"]
        print value // あ い う え お が順に出力される
    next

    // 連想配列はキーを返す
    hashtbl hoge = HASH_SORT
    hoge["b"] = 2
    hoge["a"] = 1
    hoge["d"] = 3
    hoge["c"] = 4

    for key in hoge
        print key        // a b c d の順に出力される
        print hoge[key]  // 1 2 3 4 の順に出力される
    next

for-else-endfor
^^^^^^^^^^^^^^^

.. sourcecode::

    for i = a to b
        ブロック文
    else
        ブロック文
    endfor

    for a in b
        ブロック文
    else
        ブロック文
    endfor

| forループをbreakで抜けなかった場合にelse句以降が実行されます

.. sourcecode:: uwscr

    for i = 0 to length(items) - 1
        if items[i] == target then
            // 要素のいずれかがtargetと一致した場合break
            target_found()
            break
        endif
    else
        // いずれの要素もtargetと一致しない場合はbreakしないのでこちらが実行される
        target_not_found()
    endfor

    // for-inにも対応
    for item in items
        if item == target then
            target_found()
            break
        endif
    else
        target_not_found()
    endfor

    // ループ内の処理が行われない場合でもelseが実行される
    for i = 0 to -1
        print 1 // 実行されないため表示もされない
    else
        print 2 // 2と表示される
    endfor
    for a in []
        print 3 // 実行されない
    else
        print 4 // 4と表示される
    endfor

while
^^^^^

.. code-block::

    while 式
        ブロック文
    wend

| `式` が真である限り `ブロック文` を繰り返し処理します
| (ループ中に式を偽にしない限り無限ループする)

.. sourcecode:: uwscr

    a = TRUE
    while a
        a = DoSomething() // 偽値を返せばループ終了
    wend

    while false
        // 式が偽なら何も実行されない
    wend

    while TRUE
        print "無限ループ"
    wend

repeat
^^^^^^

.. code-block::

    repeat
        ブロック文
    until 式

| `式` が偽である限り `ブロック文` を繰り返し処理します
| (ループ中に式を真にしない限り無限ループする)

.. sourcecode:: uwscr

    a = false
    repeat
        a = DoSomething() // 真値を返せばループ終了
    until a

    repeat
        // 式が真でも一度は必ず実行される
    until TRUE

    repeat
        print "無限ループ"
    until FALSE

continue
^^^^^^^^

.. code-block::

    continue [式]

| ループ文(for, while, repet)にてループの先頭に戻ります
| `式` は正の整数を指定します
| 省略した場合 `式` は `1` として扱われます
| 多重ループで複数のループをcontinueしたい場合 `式` に2以上(ループの数分)を指定します

.. sourcecode:: uwscr

    for i = 0 to 2
        print "3回出力される"
        continue
        print "出力されない"
    next

    a = 1
    b = 1
    while a < 5
        while TRUE
            a = a + 1
            continue 2
            b = b + 1
        wend
    wend
    print a // 5
    print b // 1

break
^^^^^

.. code-block::

    break [式]

| ループ文(for, while, repet)にてループを抜けます
| `式` は正の整数を指定します
| 省略した場合 `式` は `1` として扱われます
| 多重ループで複数のループをbreakしたい場合 `式` に2以上(ループの数分)を指定します

.. sourcecode:: uwscr

    for i = 0 to 2
        print "1回だけ出力される"
        break
        print "出力されない"
    next

    a = 0
    repeat
        repeat
            repeat
                break 3
                a = a + 1
            until FALSE
            a = a + 1
        until FALSE
        a = a + 1
    until FALSE
    print a // 0

select
^^^^^^

.. code-block::

    select 式
        case 式
            ブロック文
        [case 式, 式 …]
            ブロック文
        [default]
            ブロック文
    selend

| select式を評価し、その結果とcase式が一致した場合にそのcase以下のブロック文が処理されます
| caseに ``,`` 区切りで式を複数指定した場合、いずれかが一致すればそのブロック文が処理されます

1. select式を評価し結果を得る
2. case式を評価しselect式の結果と比較
    - 一致した場合: その下のブロック文を処理しselectを終了する
    - 不一致の場合: 次のcaseまたはdefaultに進む
3. defaultに到達した場合必ずその下のブロック文を処理する
4. defaultがなくいずれのcaseにも一致しない場合なにも行わない

.. sourcecode:: uwscr

    select hoge
       case 1
           // hogeが1なら実行される
       case 2, 3
           // hogeが2か3なら実行される
       case 3
           // hogeが3でも上のcaseが該当してるので実行されない
       default
           // hogeが1～3以外なら実行される
   selend

   select hoge
       default
           // 必ず実行される
   selend

   select 1
       case 2
           // なにも実行されない
   selend

exit
^^^^

- スクリプト本文に記述した場合
    スクリプト実行を終了します
- 関数内に記述した場合
    関数を抜けます
- REPLで実行した場合
    REPLを終了します

.. sourcecode:: uwscr

    hoge() // 2 は出力されない

    procedure hoge()
        print 1
        exit
        print 2
    fend

exitexit
^^^^^^^^

.. code-block::

    exitexit [数値]

| 数値を指定した場合はUWSCRの終了コードになります
| (省略時は終了コード0)

print
^^^^^

| 評価した式を文字列として出力します
| またそれをログファイルに記録します

.. code-block::

    print 式

| 以下の処理が行われます

- 可能であればprintウィンドウに出力
    - 通常実行時かつ ``logprint(FALSE)`` が呼ばれていない
    - コンソール実行時に ``logprint(TRUE)`` が呼ばれたあと
- 可能であればコンソールに出力
    - コンソール実行時のみ
- 可能であればログファイルに記録
    - ログを記録する設定の場合

.. _com_object:

COMオブジェクト
---------------

| :func:`createoleobj`, :func:`getactiveoleobj` により取得可能
| またCOMオブジェクトのプロパティやメソッドが別のCOMオブジェクトを返す場合もあります

プロパティの取得
^^^^^^^^^^^^^^^^

| ``COMオブジェクト.プロパティ名`` でプロパティの値を取得できます

プロパティ取得
++++++++++++++

    .. sourcecode:: uwscr

        ws = createoleobj("WScript.Shell")
        print ws.CurrentDirectory // 現在のワーキングフォルダのパスが表示される


インデックス指定
++++++++++++++++

    .. sourcecode:: uwscr

        ws = createoleobj("WScript.Shell")
        print ws.Environment.item["windir"] // %SystemRoot%
        print ws.Environment.item("windir") // () も可

コレクションに対するインデックス指定
++++++++++++++++++++++++++++++++++++

    | COMオブジェクトがコレクションの場合、インデックス指定で要素を得られる
    | Item(i)の糖衣構文として実装されているため、Itemメソッドを持たない場合はエラーになる

    .. sourcecode:: uwscr

        ws = createoleobj("WScript.Shell")
        // ws.SpecialFoldersはIWshCollectionというコレクション
        print ws.SpecialFolders[0] // いずれかの特殊フォルダのパスが表示される
        print ws.SpecialFolders(0) // ()でもOK
        // これらは以下と同じ
        print ws.SpecialFolders.Item(0)

プロパティの変更
^^^^^^^^^^^^^^^^

| プロパティに対して値を代入することでプロパティを変更できます

代入
++++

.. sourcecode:: uwscr

    ws = createoleobj("WScript.Shell")
    print ws.CurrentDirectory // 元々のカレントディレクトリ
    ws.CurrentDirectory = "D:\Hoge"
    print ws.CurrentDirectory // D:\Hoge

インデックス指定による代入
++++++++++++++++++++++++++

.. sourcecode:: uwscr

    excel = createoleobj("Excel.Application")
    excel.visible = TRUE
    excel.Workbooks.Add()

    range = excel.ActiveSheet.Range("A1:A2")
    // A1に値を代入
    range[1].Value = "hoge"
    print range[1].Value // hoge
    // A2にA1を代入
    range[2] = range[1]
    print range[2].Value // hoge

メソッドの実行
^^^^^^^^^^^^^^

| ``COMオブジェクト.メソッド名([引数, 引数, ...])`` でメソッドを実行できます
| 通常の引数に加え、名前付き引数(``名前 := 値``)や参照渡し(``ref 変数``)が利用可能です

名前なし引数
++++++++++++

.. sourcecode:: uwscr

    ws = createoleobj("WScript.Shell")
    print ws.Popup("テキスト", 0, "タイトル")

名前付き引数
++++++++++++

    | 名前を指定してメソッドに引数を渡すことができます
    | ``引数名 := 値`` と記述します

    .. sourcecode:: uwscr

        ws = createoleobj("WScript.Shell")
        print ws.Popup(Text := "テキスト", Title := "タイトル")

        // 名前なし引数との併記
        // 名前なし引数は正しい位置に書く必要がある
        print ws.Popup("テキスト", Title := "タイトル")

        // 名前付き引数のあとに名前なしは書けない
        print ws.Popup(Text := "テキスト", 0, "タイトル") // エラー

参照渡し
++++++++

    | ``ref`` または ``var`` キーワードで参照渡しになります

    .. sourcecode:: uwscr

        // uwscr x86でのみ動作

        sc = createoleobj("ScriptControl")
        sc.language = "VBScript"
        sc.ExecuteStatement(script)

        dim n = 50
        print sc.CodeObject.Hoge(ref n) // 50
        print n                         // 100

        textblock script
        Function Hoge(ByRef n)
            Hoge = n '引数をそのまま返す
            n = 100  '引数の値を更新する
        End Function
        endtextblock

一部のWMIオブジェクトのメソッドについて
+++++++++++++++++++++++++++++++++++++++

.. admonition:: 一部WMIメソッドの注意点
    :class: important

    | 一部のWMIオブジェクトのメソッドは通常のCOMオブジェクトのようなメソッド実行ができません
    | このようなメソッドに対しては内部で自動的にWMIオブジェクトのメソッド実行処理に切り替わります
    | この場合以下の制限があります

    - 名前付き引数が利用できません

    | 該当するWMIオブジェクトは以下になります

    - ISWbemObject
    - ISWbemObjectEx

    | 以下は実行例です

    .. sourcecode:: uwscr

        dim hDefKey = $80000002
        dim sSubKeyName = "SOFTWARE\Microsoft\Windows NT\CurrentVersion\"
        dim sValueName = "CurrentVersion"

        locator = CreateOleObj("Wbemscripting.SWbemLocator")
        service = locator.ConnectServer("", "root\default")
        stdRegProv = service.Get("StdRegProv")
        print stdRegProv // ComObject(ISWbemObjectEx)

        // stdRegProv.GetStringValueは通常の実行ができないためWMIメソッド処理で実行される
        dim sValue
        print stdRegProv.GetStringValue(hDefKey, sSubKeyName, sValueName, ref sValue)
        print sValue

        // 上記のメソッド実行は以下のコードと同等の処理を内部的に行っています
        inparam = stdRegProv.Methods_.Item("GetStringValue").InParameters.SpawnInstance_()
        inparam.hDefKey = hDefKey
        inparam.sSubKeyName = sSubKeyName
        inparam.sValueName = sValueName
        out = stdRegProv.ExecMethod_("GetStringValue", inparam)
        print out.ReturnValue
        print out.sValue

型の確認
^^^^^^^^

| COMオブジェクトをprintすることで型の名前を確認できます
| オブジェクトがコレクションの場合は ``型名[]`` と表示されます

.. sourcecode:: uwscr

    ws = createoleobj("WScript.Shell")
    print ws                // ComObject(IWshShell3)
    print ws.specialfolders // ComObject(IWshCollection[])

COM_ERR_IGN-COM_ERR_RET
^^^^^^^^^^^^^^^^^^^^^^^

| COMエラーの発生を無視して処理を続行させることができます

| `COM_ERR_IGN` でCOMエラーを抑制します
| `COM_ERR_RET` でCOMエラーの抑制を解除します
|
| `COM_ERR_IGN` から `COM_ERR_RET` の間でCOMエラーが発生した場合
| 実行時エラーで終了することなく処理を続行します
| その際に `COM_ERR_FLG` が ``TRUE`` になります
|
| `COM_ERR_FLG` は `COM_ERR_IGN` を呼んだ際に ``FALSE`` に初期化されます
| `COM_ERR_RET` を呼んだ場合は値がそのまま維持されます
|
| `COM_ERR_IGN` によるCOMエラーの抑制はスレッド単位で有効です

.. sourcecode:: uwscr

    // 通常はCOMエラーで動作停止する
    obj = createoleobj("Some.ComObject")
    obj.FireError() // COMエラー！

    // COMエラーを抑制するパターン
    obj = createoleobj("Some.ComObject")
    // COMエラー抑制開始
    COM_ERR_IGN

    print COM_ERR_FLG // False
    obj.FireError() // エラーになるがスクリプトは停止しない
    print COM_ERR_FLG // Trueになる

    // COMエラー抑制終了
    COM_ERR_RET

    print COM_ERR_FLG // True; COM_ERR_RETでは初期化されない

    obj.FireError() // 抑制していないのでCOMエラー

リストの改行表記
----------------

| 一部リスト表記 (``,`` 区切りの式) で改行を含めることができます
| 従来では ``_`` による行連結が必要だったところを簡潔に記述できるようになりました

.. sourcecode:: uwscr

    // 従来
    hoge = [ _
        "foo", _
        "bar", _
        "baz"  _
    ]
    // 0.5.0以降
    hoge = [
        "foo",
        "bar",
        "baz"
    ]

- 改行を含めることができる構文
    - 配列リテラル
    - 関数呼び出し時の引数
    - 関数定義の引数
    - def_dllの引数型指定時
- 改行を含めることができない構文
    - dim配列定義
    - select文case句の複数条件

.. sourcecode:: uwscr

    // 配列リテラル
    print [     // [ の後の改行
        "foo",  // カンマの後の改行
        "bar"   // 式の後の改行
        ,"baz"  // カンマを式の前に書いてもいい
    ]           // ] 前の改行

    // 関数呼び出し
    print func( // ( の後の改行
        foo,    // カンマの後の改行
        ,       // 引数省略
        bar     // 式の後の改行
        ,baz    // カンマを式の前に書いてもいい
    )           // ) 前の改行

    // 関数定義
    function hoge(
        a,
        ref b,
        c: string,
        d = 1
    )
        b = do_something_with(a)
        do_something_with(c, d)
    fend

    // def_dll
    def_dll MessageBoxA(
        hwnd,
        string,
        string,
        uint
    ):int:user32

    // 以下は対象外

    // dim配列宣言で改行するとエラーになる
    dim fuga[] = 1,
                 2,
                 3
    // 必ず一行で書く
    dim hoge[] = 1,2,3

    // select文case句の複数条件で改行するとエラー
    select fuga
        case 1,
             2,
             3
            print "ng"
    selend
    // 必ず一行で書く
    select hoge
        case 1,2,3
            print "ok"
    selend

値型
----

.. list-table:: UWSCRで使われる値の型
    :header-rows: 1

    * - 型
      - 詳細
      - 参照
    * - 数値
      - double (倍精度浮動小数点型)
      - no
    * - 文字列
      - 文字列
      - no
    * - 真偽値
      - TRUE/FALSE
      - no
    * - 配列
      - 要素として異なる値型を格納できる
      - no
    * - 連想配列
      - 連想配列
      - yes
    * - 無名関数
      - 名前なしで定義されたfunction/procedure
      - no
    * - 関数
      - 名前ありで定義されたfunction/procedure
      - no
    * - 非同期関数
      - async宣言した関数
      - no
    * - 組み込み関数
      - 組み込み(ビルトイン)関数
      - no
    * - モジュール
      - module定義
      - yes
    * - クラス定義
      - class定義
      - no
    * - クラスインスタンス
      - ``クラス名()`` で得られる
      - yes
    * - NULL
      - def_dllにおけるNULL文字(chr(0))、UObjectのnull値
      - no
    * - EMPTY
      - 空の値、場合により空文字や0として扱われる
      - no
    * - NOTHING
      - 空オブジェクト
      - no
    * - 正規表現
      - 正規表現パターン
      - no
    * - UObject
      - UObject
      - no
    * - 列挙型
      - enum定義
      - no
    * - タスク
      - 非同期に行われる処理
      - yes
    * - DLL関数
      - def_dll定義
      - no
    * - 構造体
      - struct定義
      - no
    * - 構造体インスタンス
      - ``構造体名()`` で得られる
      - yes
    * - COMオブジェクト
      - createoleobj/getactiveoleobj
      - yes
    * - Unknownオブジェクト
      - COMオブジェクトのプロパティ・メソッドが返す場合がある
      - yes
    * - VARIANT
      - COMオブジェクトのプロパティ・メソッドが返す値でUWSCR値型に変換できなかったもの
      - no
    * - BrowserBuilderオブジェクト
      - 起動するブラウザを構成するためのオブジェクト
      - yes
    * - Browserオブジェクト
      - 操作対象のブラウザを示すオブジェクト
      - no
    * - TabWindowオブジェクト
      - ブラウザのタブを示すオブジェクト
      - no
    * - RemoteObjectオブジェクト
      - Webページ上のJavaScriptオブジェクト
      - no
    * - WebRequestオブジェクト
      - HTTPリクエストを構成するためのオブジェクト
      - yes
    * - WebResponseオブジェクト
      - HTTPレスポンスを示すオブジェクト
      - no
    * - HtmlNodeオブジェクト
      - パースされたHTMLドキュメントを示すオブジェクト
      - no
    * - ファイルID
      - fopen
      - yes
    * - バイト配列
      - encode
      - no

