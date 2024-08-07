use tower_lsp::lsp_types::*;

pub fn get_snippets() -> Vec<CompletionItem> {
    vec![
        new_snippet(
            "for-in", "forin",
r#"for ${1:item} in ${2:items}
    ${0}
next
"#,
"for-inループ",
r#"inの右辺の配列やコレクションの要素を左辺の変数で順に受ける

```uwscr
for n in [1, 2, 3]
    print n
next
// 1
// 2
// 3
```
"#
        ),
        new_snippet(
            "for-to", "forto",
r#"for ${1:i} = ${2} to ${3}
    ${0}
next
"#,
"forループ",
r#"toの左辺から右辺まで順番に変数で受ける  
step 1 と同義

```uwscr
for i = 1 to 3
    print i
next
// 1
// 2
// 3
```
"#
        ),
        new_snippet(
            "for-to-step", "fortostep",
r#"for ${1:i} = ${2:0} to ${3:0} step ${4:0}
    ${0}
next
"#,
"ステップforループ",
r#"toの左辺から右辺まで、step数分加算して変数で受ける

```uwscr
for i = 0 to 5 step 2
    print i
next
// 0
// 2
// 4
```
"#
        ),
        new_snippet(
            "anonymous function", "function",
r#"function(${1:arguments})
    ${2:result = ${0:0}}
fend
"#,
"無名function",
r#"名前を持たない関数、変数に代入することでその変数は関数として振る舞う  
resultに代入することで値を返す

```uwscr
f = function(n: number)
    result = n * 2
fend

print f(3) // 6
print f(5) // 10

// クロージャ
function enclosure(n: number)
    result = function(m: number)
        result = n * m
    fend
fend

closure = enclosure(5)
print closure(3) // 15
print closure(8) // 40
```
"#
        ),
        new_snippet(
            "anonymous procedure", "procedure",
r#"procedure(${1:arguments})
    $0
fend
"#,
"無名procedure",
r#"名前を持たない関数、変数に代入することでその変数は関数として振る舞う  
値を返さない

```uwscr
p = procedure(ref n: number)
    n *= 2
fend

x = 10
p(x)
print x // 20
```
"#
        ),
        new_snippet(
            "async function", "asyncfunction",
r#"async function ${1:name}(${2:variables})
    result = ${0:0}
fend
"#,
"非同期function",
r#"非同期実行される関数、Taskを返す  
resultに代入することで終了したTaskから値を受けられる

```uwscr
async function f(n: number)
    sleep(1)
    result = n * 2
fend

print await f(5) // 10
```
"#
        ),
        new_snippet(
            "async procedure", "asyncprocedure",
r#"async procedure ${1:name}(${2:variables})
    $0
fend
"#,
"非同期procedure",
r#"非同期実行される関数、Taskを返す  
Taskは値を返さない

```uwscr
async procedure p(s)
    sleep(1)
    print s
fend

t = p("hoge")
sleep(3) // 待機中に hoge がprintされる
waittask(t)
```
"#
        ),
        new_snippet(
            "class-endclass", "class",
r#"class ${1:name}
    // constructor
    procedure ${1:name}()
        $0
    fend

${2:    // destructor (optional)
    procedure _${1:name}_()

    fend}
endclass
"#,
"クラス定義",
r#"`クラス名()` を実行することでインスタンスを返す  
すべての参照がなくなるとデストラクタが実行される

```uwscr
class MyClass
    dim name
    procedure MyClass(name: string)
        this.name = name
    fend
    function name()
        result = this.name
    fend
    procedure _MyClass_()
        print "<#name>のデストラクタが実行されました"
    fend
endclass

hoge = MyClass("hoge")
hoge2 = hoge // hogeのコピー
fuga = MyClass("fuga")
print hoge.name() // hoge
print fuga.name() // fuga

// NOTHING代入で明示的に破棄できる
hoge = NOTHING // hogeのデストラクタが実行されました
// hoge2も破棄されている
print hoge2 // NOTHING

// スクリプト終了時にfugaが破棄される
sleep(1)
// fugaのデストラクタが実行されました
```
"#
        ),
        new_snippet(
            "def_dll", "def_dll",
r#"def_dll ${1:funcname}(${2:varType}):${3:retType}:${4:dllName}.dll
"#,
"dll関数定義",
r#"dll関数を呼び出せるようにする

```uwscr
def_dll GetCursorPos({long, long}):bool:user32.dll
dim x, y
GetCursorPos(x, y)
print [x, y]
```
"#
        ),
        new_snippet(
            "def_dll alias", "def_dllalias",
r#"def_dll ${1:alias}:${2:funcname}(${4:varType}):${4:retType}:${5:dllName}.dll
"#,
"dll関数別名定義",
r#"dll関数に任意の名前を付けて呼び出せるようにする

```uwscr
// GetCursorPosをMousePosとして呼び出す
def_dll MousePos:GetCursorPos({long, long}):bool:user32.dll
dim x, y
MousePos(x, y)
print [x, y]
```
"#
        ),
        new_snippet(
            "enum-endenum", "enum",
r#"enum ${1:identifier}
    $0
endenum
"#,
"列挙体定義",
r#"列挙体を定義する

```uwscr
enum E
    Foo
    Bar
endenum

// 列挙体は引数の型にできる
function f(n: E)
    select n
        case E.Foo
            result = "Foo"
        case E.Bar
            result = "Bar"
        default
            result = "unreachable"
    selend
fend

print f(E.Foo) // Foo
print f(100)   // エラー
```
"#
        ),
        new_snippet(
            "function", "function",
r#"function ${1:name}(${2:variables})
    result = ${0:0}
fend
"#,
"関数定義(戻り値あり)",
r#"関数を定義する  
resultに代入することで値を返す

```uwscr
function f(n: number)
    result = n * n
fend

print f(5)  // 25
print f(10) // 100
```
"#
        ),
        new_snippet(
            "hash-endhash", "hash",
r#"hash ${1:public }${2:name}${3: = HASH_${4:*}}
    ${5:key} = ${6:value}
    ${0}
endhash
"#,
"連想配列一括定義",
r#"hashtblの糖衣構文  
予め連想配列のキーと値を設定できる

```uwscr
hash hoge = HASH_SORT
    foo = 100
    bar = 200
    baz = 300
endhash
for key in hoge
    print key
    print hoge[key]
next
// BAR
// 200
// BAZ
// 300
// FOO
// 100
```
"#
        ),
        new_snippet(
            "if-endif", "ifendif",
r#"if ${1:expression} then
    $0
endif
"#,
"if文",
r#"式が真ならブロック内が実行される

```uwscr
if true then
    print "printされる"
endif
if false then
    print "printされない"
endif
```
"#
        ),
        new_snippet(
            "if-else-endif", "ifelseendif",
r#"if ${1:expression} then
    $2
else
    $3
endif
"#,
"if-else文",
r#"式が真ならthenブロックが、偽ならelseブロックが実行される

```uwscr
if true then
    print "printされる"
else
    print "printされない"
endif
```
"#
        ),
        new_snippet(
            "if-else単行", "ifelsesingle",
r#"if ${1:expression} then $2 else $3
"#,
"単行if-else文",
r#"式が真ならthen以降が、偽ならelse以降の文が実行される

```uwscr
if true then print "printされる" else print "printされない"
```
"#
        ),
        new_snippet(
            "if-elseif", "ifelseif",
r#"if ${1:expression} then
    $3
elseif ${2:expression2}
    $0
endif
"#,
"if-elseif文",
r#"ifの式が偽であればelseifの式を評価し、真であればそのブロックを実行し偽であれば更に次のelseifまたはelseの式の評価に移行する

```uwscr
a = 1
if a == 0 then
    print "0でした"
elseif a == 1
    print "1でした"
else
    print "0でも1でもありません"
endif
```
"#
        ),
        new_snippet(
            "if単行", "ifsingle",
r#"if ${1:expression} then $0
"#,
"単行if文",
r#"式が真であればthen以降の文が実行される

```uwscr
if true then print "printされる"
if false then print "printされない"
```
"#
        ),
        new_snippet(
            "module-endmodule", "module",
r#"module ${1:name}
    // constructor
    procedure ${1:name}
        $2
    fend
    $0
endmodule
"#,
"モジュール",
r#"関数定義等をモジュール化する  
モジュールのコンストラクタはスクリプト開始時に実行される

```uwscr
// スクリプト開始時に実行される がprintされる

print MyModule.f() // モジュール関数

module MyModule
    procedure MyModule
        print "スクリプト開始時に実行される"
    fend
    function f()
        result = "モジュール関数"
    fend
endmodule
```
"#
        ),
        new_snippet(
            "procedure", "procedure",
r#"procedure ${1:name}(${2:variables})
    $0
fend
"#,
"関数定義(戻り値なし)",
r#"値を返さない関数を定義する

```uwscr
procedure p(s)
    print "hello <#s>!"
fend

p("world") // hello world!
```
"#
        ),
        new_snippet(
            "repeat-until", "repeatuntil",
r#"repeat
    $0
until ${1:expression}
"#,
"repeat文",
r#"untilが真になるまでループする

```uwscr
n = 0
repeat
    n += 1
until n > 5
print n // 6
```
"#
        ),
        new_snippet(
            "select-selend", "selectselend",
r#"select ${1:expression}
    case $2
        $3
    ${4:default
        $5}
selend
"#,
"select文",
r#"式を評価し値が一致するcaseのブロックを実行する  
一致するものがなければdefaultブロックが実行される

```uwscr
a = 5
select a
    case 1
        print "1でした"
    // , 区切りで複数の条件を設定できる
    case 2,3,4
        print "2～4でした"
    default
        print "1～4ではありませんでした"
selend
```
"#
        ),
        new_snippet(
            "struct-endstruct", "struct",
r#"struct ${1:identifier}
    ${2:name}: ${3:type}
endstruct
"#,
"構造体定義",
r#"Cライクな構造体を定義する  
`構造体名()` を実行することでインスタンスを返す

```uwscr
// Point構造体を定義
// 各メンバは0で初期化される
struct Point
    x: int
    y: int
endstruct

// def_dllには型として struct を指定
// 構造体はポインタが渡るためvar/refは不要
def_dll GetCursorPos(struct):bool:user32.dll

// インスタンスを作る
p = Point()

// dll関数に引数として渡す
GetCursorPos(p)

// 受けた値をprint
print [p.x, p.y]
```
"#
        ),
        new_snippet(
            "textblock-endtextblock", "textblock",
r#"textblock ${1:name}
$0
endtextblock
"#,
"複数行文字列定数定義",
r#"複数行の文字列による定数を定義する  
改行がそのまま反映される

定数名を省略した場合は評価されない  
※ 複数行コメントとして利用可能

```uwscr
textblock t
Foo
Bar
Baz
endtextblock

print t
// Foo
// Bar
// Baz

textblock
名前を省略した場合、評価されない
ここに書かれた文字列を得る手段もない
endtextblock
```
"#
        ),
        new_snippet(
            "textblockex-endtextblock", "textblockex",
r#"textblockex ${1:name}
$0
endtextblock
"#,
"展開可能textblock",
r#"変数展開可能なtextblock  
展開される変数は遅延評価される

```uwscr
textblockex t
<#foo>
endtextblock

// fooが存在しないので展開されない
print t // <#foo>

foo = 123
print t // 123

foo = "ほげほげ"
print t // ほげほげ
```
"#
        ),
        new_snippet(
            "try-except", "tryexcept",
r#"try
    $1
except
    $2
endtry
"#,
"try-except文",
r#"tryブロックでエラーが発生した場合のみexceptブロックが処理される

```uwscr
try
    print "エラーなし"
except
    print "printされない"
endtry

try
    raise("エラー発生", "サンプルコード")
except
    print TRY_ERRMSG // [サンプルコード] エラー発生
endtry
```
"#
        ),
        new_snippet(
            "try-except-finally", "tryexceptfinally",
r#"try
    $1
except
    $2
finally
    $3
endtry
"#,
"try-except-finally文",
r#"exceptとfinallyの複合  
exceptはtryブロックでエラー発生時のみ処理される  
finallyは必ず処理される

```uwscr
try
    print "エラーなし"
except
    print "printされない"
finally
    print "printされる"
endtry

try
    raise("エラー発生", "サンプルコード")
except
    print TRY_ERRMSG // [サンプルコード] エラー発生
finally
    print "printされる"
endtry
```
"#
        ),
        new_snippet(
            "try-finally", "tryfinally",
r#"try
    $1
finally
    $2
endtry
"#,
"try-finally文",
r#"エラーの有無にかかわらずfinallyブロックが処理される

```uwscr
try
    print "エラーなし"
finally
    print "printされる"
endtry

try
    raise("エラー発生", "サンプルコード")
finally
    print TRY_ERRMSG // [サンプルコード] エラー発生
endtry
```
"#
        ),
        new_snippet(
            "while-wend", "whilewend",
r#"while ${1:expression}
    $0
wend
"#,
"whileループ",
r#"式が真である限りブロックを処理する

```uwscr
a = 0
while a < 5
    a += 1
wend
print a // 5
```
"#
        ),
        new_snippet(
            "with-endwith", "with",
r#"with ${1:expression}
    $0
endwith
"#,
"with文",
r#"式がオブジェクトであれば、メンバの呼び出しでドットの左辺を省略できる

```uwscr
class Hoge
    procedure Hoge()
    fend
    function one
        result = 1
    fend
    function two
        result = 2
    fend
    procedure _Hoge_()
        print "withを抜けたときに破棄される"
    fend
endclass

with Hoge()
    print .one() // 1
    print .two() // 2
endwith // withを抜けたときに破棄される がprintされる
```
"#
        ),
//         new_snippet(
//             "三項演算子", "?:",
// r#"${1:cond} ? ${2:cons} : ${3:alt}
// "#,
// r#"### 三項演算子

// cond式が真であればcons式、偽であればalt式が処理される
// 単行ifと違い全体が式である
// また、consとaltに文を記述できない

// ```uwscr
// print true ? "真" : "偽"  // 真
// print false ? "真" : "偽" // 偽
// ```
// "#
//         ),
        new_snippet(
            "hashtbl", "hashtbl",
r#"hashtbl ${1:ident}${2: = HASH_$3}
"#,
"連想配列定義",
r#"key-value式の配列  
宣言時に以下を指定することができる (OR連結可)

- HASH_SORT: キーをソートする、未指定時は挿入順になる
- HASH_CASECARE: キーは大文字小文字を区別する

```uwscr
hashtbl h
// キー(文字列)に対して値を代入
h["b"] = 2
h["a"] = 1
print h["a"] // 1
print h["b"] // 2

// 挿入順に格納される
for key in h
    print key
next
// B
// A

hashtbl t = HASH_SORT or HASH_CASECARE
t["b"] = 100
t["a"] = 200
t["A"] = 300

// 大文字小文字の区別
print t["a"] // 200
print t["A"] // 300

// キー順ソートされている
for key in t
    print key
next
// A
// a
// b
```
"#
        ),
        new_snippet(
            "com_err_ign", "COM_ERR_IGN",
r#"COM_ERR_IGN
$0"#,
"COMエラー抑止開始",
r#"COM_ERR_IGN記述位置からCOMエラーが発生してもエラーで停止しないようにする

エラー抑止中にエラーが発生していた場合は特殊変数 `COM_ERR_FLG` が `TRUE` になる
"#
        ),
        new_snippet(
            "com_err_ret", "COM_ERR_RET",
r#"COM_ERR_RET
$0"#,
"COMエラー抑止終了",
r#"COM_ERR_IGNからCOM_ERR_RET記述位置までをCOMエラー抑止範囲とする

エラー抑止中にエラーが発生していた場合は特殊変数 `COM_ERR_FLG` が `TRUE` になる
"#
        ),
        new_snippet(
            "OPTION EXPLICIT", "OPTION EXPLICIT",
r#"OPTION EXPLICIT${1:=${2:TRUE}}
$0"#,
"OPTION設定: 変数宣言を強制",
r#"有効にすることで`dim`または`public`宣言していない変数はエラーになる

```uwscr
dim a
public b
a = 1
b = 2
// 未宣言変数cはエラーになる
c = 3
```
"#
        ),
        new_snippet(
            "OPTION SAMESTR", "OPTION SAMESTR",
r#"OPTION SAMESTR${1:=${2:TRUE}}
$0"#,
"OPTION設定: 文字列比較の大小文字区別",
r#"有効にすることで文字列の比較時に大文字小文字を区別する

```uwscr
OPTION SAMESTR

print "a" == "A" // False
```
"#
        ),
        new_snippet(
            "OPTION OPTPUBLIC", "OPTION OPTPUBLIC",
r#"OPTION OPTPUBLIC${1:=${2:TRUE}}
$0"#,
"OPTION設定: public変数宣言重複禁止",
r#"有効にすることでpublic変数宣言の重複を禁止する

```uwscr
OPTION OPTPUBLIC

public a = 1
public a = 2 // エラー
```
"#
        ),
        new_snippet(
            "OPTION OPTFINALLY", "OPTION OPTFINALLY",
r#"OPTION OPTFINALLY${1:=${2:TRUE}}
$0"#,
"OPTION設定: try強制終了時のfinally実行",
r#"有効にすることでtry節で強制終了した場合でもfinally節を実行する

```uwscr
OPTION OPTFINALLY

try
    exitexit
finally
    print "printされる"
endtry
```
"#
        ),
        new_snippet(
            "OPTION SPECIALCHAR", "OPTION SPECIALCHAR",
r#"OPTION SPECIALCHAR${1:=${2:TRUE}}
$0"#,
"OPTION設定: 文字列展開無効",
r#"有効にすることで`<#CR>`などの特殊文字及び変数の展開が無効になる

```uwscr
OPTION SPECIALCHAR

a = 1
print "hoge<#CR>fuga<#a>" // hoge<#CR>fuga<#a>
```
"#
        ),
        new_snippet(
            "OPTION SHORTCIRCUIT", "OPTION SHORTCIRCUIT",
r#"OPTION SHORTCIRCUIT${1:=${2:FALSE}}
$0"#,
"OPTION設定: 短絡評価",
r#"有効にすることで論理演算を短絡評価する  
デフォルト有効なので無効にしたい場合にFALSEを指定

```uwscr
// 短絡評価しない
OPTION SHORTCIRCUIT=FALSE

function f()
    sleep(3)
    result = TRUE
fend

// f() が評価されるため3秒停止してFALSEを返す
print FALSE andl f()
```

```uwscr
// 短絡評価する
OPTION SHORTCIRCUIT=TRUE

function f()
    sleep(3)
    result = TRUE
fend

// f() が評価されないため即FALSEを返す
print FALSE andl f()
```
"#
        ),
        new_snippet(
            "OPTION FIXBALLOON", "OPTION FIXBALLOON",
r#"OPTION FIXBALLOON${1:=${2:TRUE}}
$0"#,
"OPTION設定: 仮想デスクトップ吹き出し表示",
r#"有効にすることで吹き出しを仮想デスクトップにも表示する

```uwscr
OPTION FIXBALLOON

balloon("3秒後に仮想デスクトップ移動")

sleep(3)
// Ctrl+Win+D で仮想デスクトップを新規作成して移動
sckey(0, VK_CTRL, VK_WIN, VK_D)

msgbox("吹き出しが表示されている")
```
"#
        ),
        new_snippet(
            "OPTION DEFAULTFONT", "OPTION DEFAULTFONT",
r#"OPTION DEFAULTFONT="${1:name},${2:size}"
$0"#,
"OPTION設定: ダイアログフォント",
r#"ダイアログ等のフォントを変更する  
デフォルトでは `"Yu Gothic UI,20"`

```uwscr
OPTION DEFAULTFONT="MS Gothic,30"

msgbox("フォント変更")
```
"#
        ),
        new_snippet(
            "OPTION LOGFILE", "OPTION LOGFILE",
r#"OPTION LOGFILE=${1:n}
$0"#,
"OPTION設定: ログ出力オプション",
r#"ログファイルの出力オプションを数値で指定  
デフォルトではログを出力しない

- 0: 通常のログ出力
- 1: ログ出力なし (デフォルト)
- 2: 日時出力なし
- 3: 通常のログ出力 (標準で秒を含むため0と同じ)
- 4: 以前のログを破棄
- それ以外: ログ出力なし

```uwscr
OPTION LOGFILE=4

print "hoge"
print fget(fopen("uwscr.log", F_READ), F_ALLTEXT) // 202X-XX-XX XX:XX:XX [PRINT]  hoge
```
"#
        ),
        new_snippet(
            "OPTION LOGPATH", "OPTION LOGPATH",
r#"OPTION LOGPATH="${1:path}"
$0"#,
"OPTION設定: ログ保存パス",
r#"ログファイルのパスを指定  
デフォルトではスクリプト実行ディレクトリに`uwscr.log`が作成される  
ディレクトリパス指定時はそこに`uwscr.log`が作成される  
ファイルパス指定時はそのファイルにログが書き込まれる

```uwscr
OPTION LOGFILE=4
OPTION LOGPATH="C:\uwscr\test.log"

print "hoge"
print fget(fopen("C:\uwscr\test.log", F_READ), F_ALLTEXT) // 202X-XX-XX XX:XX:XX [PRINT]  hoge
```
"#
        ),
        new_snippet(
            "OPTION LOGLINES", "OPTION LOGLINES",
r#"OPTION LOGLINES=${1:n}
$0"#,
"OPTION設定: ログ保存行数",
r#"ログファイルの最大行数を指定  
この行を越えた場合古いものから削除される

```uwscr
OPTION LOGFILE=4
// 最大3行
OPTION LOGLINES=3

print "foo"
print "bar"
print "baz"
print "qux"
print "quxx"

print fget(fopen("uwscr.log", F_READ), F_ALLTEXT)
// 202X-XX-XX XX:XX:XX [PRINT]  baz
// 202X-XX-XX XX:XX:XX [PRINT]  qux
// 202X-XX-XX XX:XX:XX [PRINT]  quux
```
"#
        ),
        new_snippet(
            "OPTION DLGTITLE", "OPTION DLGTITLE",
r#"OPTION DLGTITLE="${1:title}"
$0"#,
"OPTION設定: ダイアログタイトル",
r#"ダイアログのタイトルを変更する

```uwscr
OPTION DLGTITLE="変更しました"

msgbox("OPTION DLGTITLE")
```
"#
        ),
        new_snippet(
            "OPTION GUIPRINT", "OPTION GUIPRINT",
r#"OPTION GUIPRINT${1:=${2:TRUE}}
$0"#,
"OPTION設定: print GUI出力",
r#"有効にすることでprint時にコンソールではなくLogPrintウィンドウに出力する

```uwscr
OPTION GUIPRINT

print "foo"
print "bar"
print "baz"

id = getid(GET_LOGPRINT_WIN)
while status(id, ST_VISIBLE)
    sleep(0.1)
wend
```
"#
        ),
        new_snippet(
            "OPTION FORCEBOOL", "OPTION FORCEBOOL",
r#"OPTION FORCEBOOL${1:=${2:TRUE}}
$0"#,
"OPTION設定: 条件式の真偽値強制",
r#"有効にすることでif文などの条件式で真偽値以外を受け付けなくなる

```uwscr
OPTION FORCEBOOL

// 真偽値を返さない式を記述するとエラー
if 1 then // [評価エラー] 条件式はTRUEまたはFALSEを返す必要があります
    print 1
endif
```
"#
        ),
        new_snippet(
            "OPTION CONDUWSC", "OPTION CONDUWSC",
r#"OPTION CONDUWSC${1:=${2:TRUE}}
$0"#,
"OPTION設定: 条件式の判定をUWSC方式にする",
r#"有効にすることで条件式にてUWSCと同等の判定を行う

```uwscr
OPTION CONDUWSC

if "0" then
    print "出力されない"
else
    print "文字列0は偽と判定されるため出力される"
endif

if "hoge" then // 数値変換できないためエラー
endif
```
"#
        ),
    ]
}

fn new_snippet(detail: &str, label: &str, snippet: &str, doc_title: &str, doc: &str) -> CompletionItem {
    CompletionItem {
        label: label.to_string(),
        label_details: Some(CompletionItemLabelDetails {
            detail: None,
            description: Some(doc_title.to_string())
        }),
        kind: Some(CompletionItemKind::SNIPPET),
        detail: Some(detail.to_string()),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format!("### {doc_title}\n\n{doc}")
        })),
        // deprecated: todo!(),
        // preselect: todo!(),
        // sort_text: todo!(),
        // filter_text: todo!(),
        insert_text: Some(snippet.to_string()),
        // insert_text: None,
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        insert_text_mode: Some(InsertTextMode::ADJUST_INDENTATION),
        text_edit: None,
        additional_text_edits: None,
        // command: todo!(),
        // commit_characters: todo!(),
        // data: todo!(),
        // tags: todo!(),
        ..Default::default()
    }
}