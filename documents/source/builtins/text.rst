文字列操作関数
==============

コピー
------

.. function:: copy(対象文字列, 開始位置, [コピー文字数=EMPTY])

    | 文字列をコピーします

    :param 文字列 対象文字列: コピー元の文字列
    :param 数値 開始位置: コピー開始位置 (1から)
    :param 数値 省略可 コピー文字数: 開始位置からコピーする文字数、省略時は末尾まで
    :return: コピーした文字列

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            moji = "あいうえおかきくけこ"
            print copy(moji, 6)    // かきくけこ
            print copy(moji, 3, 4) // うえおか
            print copy(moji, 11)   // (範囲外のため空文字)


.. function:: betweenstr(対象文字列, [前文字=EMPTY, 後文字=EMPTY, n番目=1, 数え方=FALSE])

    | 対象文字列から前文字列と後文字に挟まれた部分の文字列をコピーします

    :param 文字列 対象文字列: コピー元の文字列
    :param 文字列 省略可 前文字: コピーしたい文字列の前にある文字列、省略時は対象文字列の先頭から
    :param 文字列 省略可 後文字: コピーしたい文字列の後にある文字列、省略時は対象文字列の末尾まで
    :param 数値 省略可 n番目:

        | n番目に一致する前後文字の組み合わせからコピーする、マイナスの場合後ろから探す
        | 0指定時: 前後文字があれば該当文字列をすべて取得、それ以外は1として扱う

    :param 真偽値 省略可 数え方: n番目の数え方を指定します

        - TRUEかつ正順: n番目の前文字を探し、その後に対となる後文字を探す
        - TRUEかつ逆順: 後ろからn番目の後文字を探し、その後に対となる前文字を探す
        - FALSEかつ正順: 前文字とその直後の後文字をペアとし、そのn番目を探す (ペア中に別の前文字があっても無視される)
        - FALSEかつ逆順: 後ろから見て後文字とその直前の前文字をペアとし、そのn番目を探す (ペア中に別の後ろ文字があっても無視される)

        .. admonition:: 前文字または後文字省略時は無視されます
            :class: note

            - 前文字を指定し後文字を省略
            - 前文字を省略し後文字を指定

            | の場合この引数は無視されます
            | その為UWSCとは結果が異なります

    :rtype: 文字列またはEMPTYまたは配列
    :return:

        - 前文字のみ指定: n番目の前文字以降の文字列を返す、該当なしならEMPTY
        - 後文字のみ指定: n番目の後文字までの文字列を返す、該当なしならEMPTY
        - 前後文字指定
            - n番目が0: 該当する文字列すべてを配列で返す、該当なしの場合空配列
            - n番目が0以外: 該当する文字列を返す、該当なしならEMPTY

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            str = "abc?def!ghi?jkl!mno"
            // 前後文字未指定の場合はそのままコピー
            print betweenstr(str)
            // abc?def!ghi?jkl!mno

            // 前文字のみの場合は前文字後から末尾まで
            print betweenstr(str, 'abc')
            // ?def!ghi?jkl!mno

            // 後文字のみの場合は先頭から後文字の前まで
            print betweenstr(str, , 'jkl')
            // abc?def!ghi?

            // 前文字と後文字を指定するとその間
            print betweenstr(str, 'abc', 'jkl')
            // ?def!ghi?

            // n番目の指定
            print betweenstr(str, '?', '!', 1)
            // def
            print betweenstr(str, '?', '!', 2)
            // jkl

            str = "?aaa?bbb!ccc?ddd!eee"
            // 数え方指定
            print betweenstr(str, '?', '!', 2, TRUE)
            // bbb
            print betweenstr(str, '?', '!', 2, FALSE)
            // ddd


.. function:: token(区切り文字, var 元文字列, [区切り方法=FALSE, ダブルクォート=FALSE])

    | 区切り文字から手前の文字を切り出します
    | もとの文字は切り出された状態になります

    :param 文字列 区切り文字: 区切りとなる文字、文字列の場合それぞれの文字が区切りとなる
    :param 文字列 参照渡し 元文字列: 切り出される文字列、関数実行後に切り出された残りの文字列が戻ります
    :param 真偽値 省略可 区切り方法: 区切り文字が連続していた場合の処理方法を指定

        .. object:: TRUE

            連続した区切り文字を一つの区切りとして扱う

        .. object:: FALSE

            区切り文字が連続していてもそれぞれの文字を区切りとする

    :param 真偽値 省略可 ダブルクォート: ダブルクォートで括られた文字列内で区切るかどうか

        .. object:: TRUE

            ダブルクォートで括られている文字列内の区切り文字を無視

        .. object:: FALSE

            ダブルクォートがあっても区切る

    :return: 切り出した文字列

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            moji = "あ-い-う-え-お"
            print token("-", moji) // あ
            print moji             // い-う-え-お
            print token("-", moji) // い
            print moji             // う-え-お

            // 連続するトークン

            // FALSEは個別に区切る
            moji = "あいうabcえお"
            // a で区切る
            print token("abc", moji, FALSE) // あいう
            print moji                      // bcえお
            // b で区切る
            print token("abc", moji, FALSE) // (空文字)
            print moji                      // cえお

            // TRUEならまとめて区切る
            moji = "あいうabcえお"
            print token("abc", moji, TRUE) // あいう
            print moji                     // えお

            // 該当する区切りがない場合文字列全体が切り出される

            moji = "あいうえお"
            print token("abc", moji) // あいうえお
            print moji               // (空文字)

            // ダブルクォート内の区切り

            csv = "<#DBL>foo,bar<#DBL>,baz"
            print token(",", csv)        // "foo
            print csv                    // bar",baz
            csv = "<#DBL>foo,bar<#DBL>,baz"
            print token(",", csv, ,TRUE) // "foo,bar"
            print csv                    // baz


置換
----

.. function:: replace(対象文字列, 置換対象, 置換文字列, [正規表現モード=FALSE])
.. function:: chgmoj(対象文字列, 置換対象, 置換文字列, [正規表現モード=FALSE])

    | マッチした文字列を指定文字列で置換します
    | 正規表現による置換も可能

    :param 文字列 対象文字列: 対象となる文字列
    :param 文字列 置換対象: 置換する文字列、正規表現モードの場合は正規表現を示す文字列
    :param 正規表現 置換対象: 正規表現オブジェクト (これを指定した場合必ず正規表現モードになる)
    :param 文字列 置換文字列: 置換後の文字列

        .. admonition:: マッチ文字列に置換
            :class: note

            | 正規表現モードでは以下が使用可能
            | ``$0`` がマッチした文字列そのものに置換される
            | ``$1`` 以降はサブマッチ

    :param 真偽値 省略可 正規表現モード:

        | 正規表現による置換を行う場合は ``TRUE``
        | 置換対象に正規表現オブジェクトを渡した場合はこの値は無視される
        | 正規表現モードの場合は大文字小文字が区別されます
        | 正規表現モードでない場合は大文字小文字は区別されません

    :return:

        | 置換された文字列
        | 置換対象が対象文字列にマッチしなかった場合は対象文字列がそのまま返る

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            // 正規表現モードの場合は大文字小文字が区別される
            print replace("aA", "A", "B")       // BB
            print replace("aA", "A", "B", TRUE) // aB

            // マッチ文字列を使った置換
            print replace("aa11bb22cc33", "([a-z]+)(\d+)", "$1 = $2, ", TRUE)
            // aa = 11, bb = 22, cc = 33,

サイズ
------

.. function:: length(値)

    | 文字列の文字数、配列や構造体のサイズを返します
    | 長さを返せない値が渡された場合はエラー

    :param 文字列・配列・連想配列・構造体・RemoteObject 値: 文字数を得たい文字列
    :return: 文字数やサイズを示す数値

    .. admonition:: 対応する値型
        :class: note

        | UWSCとの互換性を保つため数値やbool値も対象です
        | この場合それらを文字列として扱いその長さを返します
        | また、Emptyは0、NULLは1を返します

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            print length("あいうえお") // 5
            print length([1, 2, 3]) // 3

            // 構造体定義
            struct Point
                x: long // 4
                y: long // 4
            endstruct

            print length(Point) // 8

            p = Point() // 構造体インスタンスにも対応
            print length(p) // 8

            sa = safearray(0, 3)
            print length(sa) // 4
            print length(sa, TRUE) // 1 (次元)

            sa = safearray(0, 5, 0, 2)
            print length(sa) // 6
            print length(sa, TRUE) // 2 (次元)

.. function:: lengthb(文字列)

    | 文字列のバイト数(ANSI)を得ます

    :param 文字列 文字列: 長さを得たい文字列
    :return: ANSIバイト数

.. function:: lengthu(文字列)

    | 文字列のバイト数(UTF-8)を得ます

    :param 文字列 文字列: 長さを得たい文字列
    :return: UTF8バイト数

.. function:: lengths(文字列)

    | サロゲートペアの文字を2文字分としてカウントします

    :param 文字列 文字列: 長さを得たい文字列
    :return: サロゲートペアを2文字とした文字数

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            str = "森鷗外𠮟る"
            print length(str)  // 5
            print lengths(str) // 6

.. function:: lengthw(文字列)

    | NULL終端Unicode文字列としての長さを得ます

    :param 文字列 文字列: 長さを得たい文字列
    :return: 符号なし16ビット整数の配列長

正規表現
--------

.. function:: NewRE(正規表現, [大小文字=FALSE, 複数行=FALSE, 改行=FALSE])

    | 正規表現オブジェクトを返します

    :param 文字列 正規表現: 正規表現を表す文字列
    :param 真偽値 省略可 大小文字: 大文字小文字を区別するなら ``TRUE``
    :param 真偽値 省略可 複数行:

        | 複数行を対象とするなら ``TRUE``
        | その場合 ``^`` が行頭、 ``$`` が行末と一致する

    :param 真偽値 省略可 改行: ``TRUE`` であれば ``.`` が ``\n`` にマッチするようになる

    :return: 正規表現オブジェクト

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            print NewRe("hoge", FALSE, TRUE, TRUE) // regex: (?ima)hoge

.. function:: regex(文字列, 正規表現, [操作方法=REGEX_TEST])

    | 正規表現による様々な文字列操作を行います
    | :any:`TestRE`, :any:`Match` 及び :any:`replace` の一部の機能を持ちます

    :param 文字列 文字列: 対象となる文字列
    :param 文字列または正規表現オブジェクト 正規表現: 正規表現を示す文字列またはオブジェクト
    :param 定数または文字列 省略可 操作方法: 指定方法により結果が異なる

        .. object:: REGEX_TEST (定数)

            | 文字列に正規表現がマッチするかを調べる、 詳しくは :any:`TestRE` を参照
            | 結果は真偽値で返る

        .. object:: REGEX_MATCH (定数)

            | 正規表現にマッチした文字列を得る、 詳しくは :any:`Match` を参照
            | 結果は文字列の配列で返る

        .. object:: 文字列

            | 文字列の置換を行う
            | 置換後の文字列を返す

    :return: 操作方法による

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            target = "abc123def"
            re = "\d+"
            print regex(target, re)              // True
            print regex(target, re, REGEX_TEST)  // True
            print regex(target, re, REGEX_MATCH) // [123]
            print regex(target, re, "456")       // abc456def

.. function:: TestRE(文字列, 正規表現)

    | 文字列に対し正規表現がマッチするかを調べます
    | ``RegEx(文字列, 正規表現, REGEX_TEST)`` と同等です

    :param 文字列 文字列: 対象となる文字列
    :param 正規表現 正規表現: 正規表現文字列またはオブジェクト
    :return: 真偽値

.. function:: Match(文字列, 正規表現)

    | 正規表現にマッチした文字列を列挙します
    | ``RegEx(文字列, 正規表現, REGEX_MATCH)`` と同等です

    :param 文字列 文字列: 対象となる文字列
    :param 正規表現 正規表現: 正規表現文字列またはオブジェクト
    :return: 配列

        - グループマッチをしない場合: 文字列の配列

            各要素がマッチした文字列

        - グループマッチした場合: 文字列の二次元配列

            各要素の1番目がマッチした全体の文字列、2番目以降はサブマッチした文字列

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            // グループマッチなし
            for m in match("aa11bb22cc33", "\d+")
                print "found: " + m
            next
            // found: 11
            // found: 22
            // found: 33

            // グループマッチなし
            for matches in match("aa11bb22cc33", "([a-z]+)(\d+)")
                print "found: " + matches[0]
                if length(matches) > 1 then
                    print "  submatches:"
                    for i = 1 to length(matches) - 1
                        print "    " + matches[i]
                    next
                endif
            next
            // found: aa11
            //   submatches:
            //     aa
            //     11
            // found: bb22
            //   submatches:
            //     bb
            //     22
            // found: cc33
            //   submatches:
            //     cc
            //     33

利用可能な正規表現
^^^^^^^^^^^^^^^^^^

`こちら <https://docs.rs/regex/1.6.0/regex/index.html#syntax>`_ を参照してください

JSON
----

.. function:: FromJson(json)

    | json文字列をUObjectにします

    :param 文字列 json: json文字列
    :return: 変換に成功した場合は ``UObject`` 、失敗時は ``EMPTY``

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            textblock json
            {
                "foo": 1,
                "bar": 2
            }
            endtextblock

            obj = fromjson(json)
            print obj.foo // 1

.. function:: ToJson(UObject, [整形=FALSE])

    | UObjectをjson文字列にします

    :param UObject UObject: json文字列にしたいUObject
    :param 真偽値 省略可 整形: TRUEならjsonを見やすい形式にする
    :return: json文字列

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            obj = @{
                "foo": 1,
                "bar": {
                    "baz": 2
                }
            }@

            print tojson(obj)
            // {"bar":{"baz":2},"foo":1}

            // 整形する
            print tojson(obj, TRUE)
            // {
            //   "bar": {
            //     "baz": 2
            //   },
            //   "foo": 1
            // }

            // 子オブジェクトも変換可能
            print tojson(obj.bar)
            // {"baz": 2}

検索
----

.. function:: pos(検索文字列, 対象文字列, [n番目=1])

    | 対象文字列の何文字目に検索文字列があるかを得ます

    :param 文字列 検索文字列: 探す文字列
    :param 文字列 対象文字列: 探される文字列
    :param 数値 省略可 n番目: n番目に一致する位置を得る、マイナスの場合後ろから探す
    :return: 見つかった位置、見つからなかった場合0

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            moji = "ももほげもももほげももももほげもも"
            print pos('ほげ', moji)     // 3
            print pos('ほげ', moji,  2) // 8
            print pos('ほげ', moji,  3) // 14
            print pos('ほげ', moji, -1) // 14 後ろから

            // 見つからない場合は0
            print pos('ほげ', moji,  4) // 0
            print pos('ふが', moji)     // 0

変換系
------

.. function:: chknum(値)

    | 与えられた値が数値に変換可能かどうかを調べる

    :param 値 値: 調べたい値
    :return: 数値に変換可能かどうかを示す真偽値

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            for v in ["1", 2, "３", "四", "Ⅴ", TRUE, "FALSE"]
                print v + ": " + chknum(v)
            next
            // 1: True
            // 2: True
            // ３: False
            // 四: False
            // Ⅴ: False
            // True: True
            // FALSE: False

.. function:: val(文字列, [エラー値=-999999])

    | 文字列を数値に変換します

    :param 文字列 文字列: 数値に変換したい文字列
    :param 数値 省略可 エラー値: 変換できなかった場合に返す数値
    :return: 成功時は変換された数値、失敗時はエラー値

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            print val(1)         // 1
            print val("2")       // 2
            print val("３")      // -999999
            print val(TRUE)      // 1
            print val("ほげ", 0) // 0

.. function:: trim(対象文字列, [全角空白=FALSE])
.. function:: trim(対象文字列, 除去文字列)
    :noindex:

    | 対象文字列の両端にあるホワイトスペースおよび制御文字を除去します

    :param 文字列 対象文字列: トリム対象文字列
    :param 真偽値 省略可 全角空白: TRUEにした場合は全角の空白もトリム対象になります
    :param 文字列 除去文字列: ホワイトスペース・制御文字ではなく指定文字を除去します
    :return: トリム後の文字列

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            print trim("  abc  ")
            // abc

            // 改行なども含む
            print trim(" <#CR> abc<#TAB>  ")
            // abc

            // 制御文字
            print trim(NULL * 3 + 'abc' + NULL * 3)
            // abc

            // 全角スペース
            print trim(" 　abc　  ")
            // 第2引数省略時は全角空白=FALSEとなる
            // 　abc　
            print trim(" 　abc　  ", FALSE)
            // 　abc　
            print trim(" 　abc　  ", TRUE)
            // abc

            // 指定文字
            // この場合 e, d, f のいずれかが連続していれば除去する
            print trim("edeffededdabcedfffedeeddedf", "edf")
            // abc

.. function:: chr(コードポイント)

    | Unicodeコードポイントから文字を得ます

    :param 数値 コードポイント: Unicodeコードポイント
    :return: 該当する文字、なければ空文字

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            print chr(128021) // 🐕

.. function:: chrb(バイトコード)

    | バイトコードからASCII文字を得ます

    :param 数値 バイトコード: 0～255
    :return: 該当する文字、なければ空文字

.. function:: asc(文字)

    | 文字からUnicodeコードポイントを得ます

    :param 文字列 文字: コードポイントを得たい文字 (文字列の場合最初の文字のみ)
    :return: 該当するUnicodeコードポイント、なければ0

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            print asc("🐕") // 128021

.. function:: ascb(文字)

    | ASCII文字からバイトコードを得ます

    :param 文字列 文字: バイトコードを得たい文字 (文字列の場合最初の文字のみ)
    :return: 該当するバイトコード、なければ0

.. function:: isunicode(対象文字列)

    | 文字列中にUnicode専用文字(ANSIにない文字)が含まれるかどうかを調べる

    :param 文字列 対象文字列: 調べたい文字列
    :return: Unicode専用文字が含まれていればTRUE

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            print isunicode("森鴎外叱る") // FALSE
            print isunicode("森鷗外𠮟る") // TRUE

.. function:: strconv(対象文字列, 変換方法)

    | 文字列を変換します (大文字↔小文字、ひらがな↔カタカナ、全角↔半角)
    | 指定方法で変換できない文字列はそのまま出力されます

    :param 文字列 対象文字列: 変換したい文字列
    :param 定数 変換方法: 変換方法を以下の定数で指定

        .. object:: SC_LOWERCASE

            小文字に変換

        .. object:: SC_UPPERCASE

            大文字に変換

        .. object:: SC_HIRAGANA

            ひらがなに変換

        .. object:: SC_KATAKANA

            カタカナに変換

        .. object:: SC_HALFWIDTH

            半角文字に変換

        .. object:: SC_FULLWIDTH

            全角文字に変換

    :return: 変換された文字列

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            print strconv('あいうえお', SC_KATAKANA)                  // アイウエオ
            print strconv('あいうえお', SC_HALFWIDTH)                 // あいうえお
            print strconv('あいうえお', SC_KATAKANA or SC_HALFWIDTH)  // ｱｲｳｴｵ
            print strconv('カキクケコ', SC_HIRAGANA)                  // かきくけこ
            print strconv('カキクケコ', SC_HALFWIDTH)                 // ｶｷｸｹｺ
            print strconv('ｻｼｽｾｿ', SC_FULLWIDTH)                      // サシスセソ
            print strconv('ｻｼｽｾｿ', SC_FULLWIDTH or SC_HIRAGANA)       // さしすせそ
            print strconv('abcde', SC_UPPERCASE)                      // ABCDE
            print strconv('abcde', SC_UPPERCASE or SC_FULLWIDTH)      // ＡＢＣＤＥ

.. function:: format(数値, 幅, [桁数=0, 埋め方法=FMT_DEFAULT])

    | 数値を指定方法でフォーマットした文字列を返します

    :param 数値 数値: フォーマットしたい数値
    :param 数値 幅: フォーマット後の文字列幅

        | 幅が入力値の桁を越えている場合、埋め方法に従い不足分を埋めます

    :param 数値 省略可 桁数: 小数点以下の桁数、または変換方法を指定

        .. object:: 1以上の数値

            小数点以下を指定桁数に丸める

        .. object:: 0

            変換しない

        .. object:: -1

            16進数に変換 (アルファベット大文字)

        .. object:: -2

            16進数に変換 (アルファベット小文字)

        .. object:: -3

            2進数に変換

    :param 定数 省略可 埋め方法: 幅に対する不足分を埋める方法

        .. object:: FMT_DEFAULT

            半角スペースで左埋め

        .. object:: FMT_ZERO

            0で左埋め

        .. object:: FMT_RIGHT

            半角スペースで右埋め

        .. object:: FMT_ZEROR

            0で右埋め

    :return: フォーマットされた文字列

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            // 幅指定
            print format(1, 8)                // '       1'
            // 小数点
            print format(1, 8, 2)             // '    1.00'
            // 丸め
            print format(1.234, 0, 2)         // 1.23
            print format(1.235, 0, 2)         // 1.24
            // 16進数
            print format(42, 0, -1)           // 2A
            // 16進数 (小文字)
            print format(42, 0, -2)           // 2a
            // 2進数
            print format(42, 0, -3)           // 101010

            // 0埋め
            print format(42, 4, -1, FMT_ZERO) // 002A
            // 右埋め
            print format(1, 8, 0, FMT_RIGHT)  // '1       '
            // 右0埋め
            print format(1, 8, 0, FMT_ZEROR)  // '10000000'

.. function:: format(文字列, 幅)
    :noindex:

    :param 文字列 文字列: フォーマットしたい文字列
    :param 数値 幅: フォーマット後の文字列幅

        | 幅が元の文字列長を越えた場合、指定幅まで元の文字を繰り返します

    :return: フォーマットされた文字列

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            // 文字列をフォーマット
            print format("abc", 8) // abcabcab
            print format("1", 8)   // 11111111

.. function:: format(秒数, 日時フォーマット文字列, [ミリ秒=FALSE])
    :noindex:

    :param 数値 秒数: 2000/01/01からの秒数またはミリ秒数
    :param 文字列 日時フォーマット文字列:

        | 日時形式を示すフォーマット文字列
        | 変換される日時はローカルタイムゾーン準拠

        .. admonition:: 時刻フォーマットの書式
            :class: hint

            | 2023/01/23 13:24:56を基準に書式の例を以下に記します

            .. list-table::
                :header-rows: 1
                :align: left

                * - 書式
                  - 出力
                  - 備考
                * - %Y
                  - 2023
                  - 年(4桁)
                * - %y
                  - 23
                  - 年(下4桁)
                * - %m
                  - 01
                  - 月(左0埋め)
                * - %d
                  - 23
                  - 日(左0埋め)
                * - %F
                  - 2023-01-23
                  - 年-月-日
                * - %H
                  - 13
                  - 時(左0埋め、24時間)
                * - %I
                  - 01
                  - 時(左0埋め、12時間)
                * - %M
                  - 24
                  - 分(左0埋め)
                * - %S
                  - 56
                  - 秒(左0埋め)
                * - %R
                  - 13:24
                  - hh:mm
                * - %R
                  - 13:24
                  - 時:分
                * - %T
                  - 13:24:56
                  - 時:分:秒
                * - %X
                  - 13時24分56秒
                  - ローカル時刻表示(日本の場合)
                * - %+
                  - 2023-01-23T13:24:56+09:00
                  - ISO8601/RFC3339形式

            | 詳細な書式一覧は `このリンク <https://docs.rs/chrono/latest/chrono/format/strftime/index.html>`_ から確認できます

        .. admonition:: 表記のローカライズについて
            :class: note

            | 日本語環境でのみ日本語にローカライズされます
            | それ以外では英語(en-US)表記になります

    :param 真偽値 省略可 ミリ秒: TRUEなら秒数をミリ秒として扱う

    :return: フォーマットされた文字列

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            // 日時フォーマット
            timestamp = gettime(, "2023/04/01 10:10:10")
            print format(timestamp, "%c") // 2023年04月01日 10時10分10秒

.. function:: encode(元文字列, 変換方式)

    | 文字列をエンコードします

    :param 文字列 元文字列: エンコードしたい文字列
    :param 定数 変換方式: 変換方式を示す定数

        .. object:: CODE_URL

            URLエンコードを行う

        .. object:: CODE_HTML

            一部の記号等を文字実態参照にする (``<`` → ``&lt;``)

        .. object:: CODE_BYTEARRAY

            バイト配列(ANSI)にする

        .. object:: CODE_BYTEARRAYW

            バイト配列(Unicode)にする

        .. object:: CODE_BYTEARRAYU

            バイト配列(UTF8)にする

        .. object:: CODE_ANSI
        .. object:: CODE_UTF8

            互換性のため定数は存在していますが、無視されます

        .. object:: 上記以外

            元の文字列が返されます

    :return: 変換方式による

.. function:: decode(文字列, 変換方式)
.. function:: decode(バイト配列, 変換方式)
    :noindex:

    | 文字列またはバイト配列をデコードします

    :param 文字列 文字列: デコードする文字列
    :param バイト配列 バイト配列: デコードするバイト配列
    :param 定数 変換方式: 変換方式を示す定数

        .. object:: CODE_URL

            URLエンコードされた文字列を元の文字列に戻す

        .. object:: CODE_HTML

            文字参照を文字に戻す (``&lt;`` → ``<``)

        .. object:: CODE_BYTEARRAY

            バイト配列(ANSI)を文字列に戻す

        .. object:: CODE_BYTEARRAYW

            バイト配列(Unicode)を文字列に戻す

        .. object:: CODE_BYTEARRAYU

            バイト配列(UTF8)を文字列に戻す

        .. object:: CODE_UTF8

            互換性のため定数は存在していますが、無視されます

        .. object:: 上記以外

            EMPTYが返されます

    :return: デコードされた文字列、変換できない場合は元文字列またはEMPTYを返す


