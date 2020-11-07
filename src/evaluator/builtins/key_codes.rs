use crate::evaluator::object::*;
use std::collections::HashMap;

pub fn set_builtin_constant(map: &mut HashMap<String, Object>) {
    let num_constant = vec![
        ("VK_A"                   , VK_A),
        ("VK_B"                   , VK_B),
        ("VK_C"                   , VK_C),
        ("VK_D"                   , VK_D),
        ("VK_E"                   , VK_E),
        ("VK_F"                   , VK_F),
        ("VK_G"                   , VK_G),
        ("VK_H"                   , VK_H),
        ("VK_I"                   , VK_I),
        ("VK_J"                   , VK_J),
        ("VK_K"                   , VK_K),
        ("VK_L"                   , VK_L),
        ("VK_M"                   , VK_M),
        ("VK_N"                   , VK_N),
        ("VK_O"                   , VK_O),
        ("VK_P"                   , VK_P),
        ("VK_Q"                   , VK_Q),
        ("VK_R"                   , VK_R),
        ("VK_S"                   , VK_S),
        ("VK_T"                   , VK_T),
        ("VK_U"                   , VK_U),
        ("VK_V"                   , VK_V),
        ("VK_W"                   , VK_W),
        ("VK_X"                   , VK_X),
        ("VK_Y"                   , VK_Y),
        ("VK_Z"                   , VK_Z),
        ("VK_0"                   , VK_0),
        ("VK_1"                   , VK_1),
        ("VK_2"                   , VK_2),
        ("VK_3"                   , VK_3),
        ("VK_4"                   , VK_4),
        ("VK_5"                   , VK_5),
        ("VK_6"                   , VK_6),
        ("VK_7"                   , VK_7),
        ("VK_8"                   , VK_8),
        ("VK_9"                   , VK_9),
        ("VK_START"               , VK_START),
        ("VK_BACK"                , VK_BACK),
        ("VK_TAB"                 , VK_TAB),
        ("VK_CLEAR"               , VK_CLEAR),
        ("VK_ESC"                 , VK_ESC),
        ("VK_ESCAPE"              , VK_ESCAPE),
        ("VK_RETURN"              , VK_RETURN),
        ("VK_ENTER"               , VK_ENTER),
        ("VK_RRETURN"             , VK_RRETURN),
        ("VK_SHIFT"               , VK_SHIFT),
        ("VK_RSHIFT"              , VK_RSHIFT),
        ("VK_WIN"                 , VK_WIN),
        ("VK_RWIN"                , VK_RWIN),
        ("VK_ALT"                 , VK_ALT),
        ("VK_MENU"                , VK_MENU),
        ("VK_RALT"                , VK_RALT),
        ("VK_CTRL"                , VK_CTRL),
        ("VK_CONTROL"             , VK_CONTROL),
        ("VK_RCTRL"               , VK_RCTRL),
        ("VK_PAUSE"               , VK_PAUSE),
        ("VK_CAPITAL"             , VK_CAPITAL),
        ("VK_KANA"                , VK_KANA),
        ("VK_FINAL"               , VK_FINAL),
        ("VK_KANJI"               , VK_KANJI),
        ("VK_CONVERT"             , VK_CONVERT),
        ("VK_NONCONVERT"          , VK_NONCONVERT),
        ("VK_ACCEPT"              , VK_ACCEPT),
        ("VK_MODECHANGE"          , VK_MODECHANGE),
        ("VK_SPACE"               , VK_SPACE),
        ("VK_PRIOR"               , VK_PRIOR),
        ("VK_NEXT"                , VK_NEXT),
        ("VK_END"                 , VK_END),
        ("VK_HOME"                , VK_HOME),
        ("VK_LEFT"                , VK_LEFT),
        ("VK_UP"                  , VK_UP),
        ("VK_RIGHT"               , VK_RIGHT),
        ("VK_DOWN"                , VK_DOWN),
        ("VK_SELECT"              , VK_SELECT),
        ("VK_PRINT"               , VK_PRINT),
        ("VK_EXECUTE"             , VK_EXECUTE),
        ("VK_SNAPSHOT"            , VK_SNAPSHOT),
        ("VK_INSERT"              , VK_INSERT),
        ("VK_DELETE"              , VK_DELETE),
        ("VK_HELP"                , VK_HELP),
        ("VK_APPS"                , VK_APPS),
        ("VK_MULTIPLY"            , VK_MULTIPLY),
        ("VK_ADD"                 , VK_ADD),
        ("VK_SEPARATOR"           , VK_SEPARATOR),
        ("VK_SUBTRACT"            , VK_SUBTRACT),
        ("VK_DECIMAL"             , VK_DECIMAL),
        ("VK_DIVIDE"              , VK_DIVIDE),
        ("VK_NUMPAD0"             , VK_NUMPAD0),
        ("VK_NUMPAD1"             , VK_NUMPAD1),
        ("VK_NUMPAD2"             , VK_NUMPAD2),
        ("VK_NUMPAD3"             , VK_NUMPAD3),
        ("VK_NUMPAD4"             , VK_NUMPAD4),
        ("VK_NUMPAD5"             , VK_NUMPAD5),
        ("VK_NUMPAD6"             , VK_NUMPAD6),
        ("VK_NUMPAD7"             , VK_NUMPAD7),
        ("VK_NUMPAD8"             , VK_NUMPAD8),
        ("VK_NUMPAD9"             , VK_NUMPAD9),
        ("VK_F1"                  , VK_F1),
        ("VK_F2"                  , VK_F2),
        ("VK_F3"                  , VK_F3),
        ("VK_F4"                  , VK_F4),
        ("VK_F5"                  , VK_F5),
        ("VK_F6"                  , VK_F6),
        ("VK_F7"                  , VK_F7),
        ("VK_F8"                  , VK_F8),
        ("VK_F9"                  , VK_F9),
        ("VK_F10"                 , VK_F10),
        ("VK_F11"                 , VK_F11),
        ("VK_F12"                 , VK_F12),
        ("VK_NUMLOCK"             , VK_NUMLOCK),
        ("VK_SCROLL"              , VK_SCROLL),
        ("VK_PLAY"                , VK_PLAY),
        ("VK_ZOOM"                , VK_ZOOM),
        ("VK_SLEEP"               , VK_SLEEP),
        ("VK_BROWSER_BACK"        , VK_BROWSER_BACK),
        ("VK_BROWSER_FORWARD"     , VK_BROWSER_FORWARD),
        ("VK_BROWSER_REFRESH"     , VK_BROWSER_REFRESH),
        ("VK_BROWSER_STOP"        , VK_BROWSER_STOP),
        ("VK_BROWSER_SEARCH"      , VK_BROWSER_SEARCH),
        ("VK_BROWSER_FAVORITES"   , VK_BROWSER_FAVORITES),
        ("VK_BROWSER_HOME"        , VK_BROWSER_HOME),
        ("VK_VOLUME_MUTE"         , VK_VOLUME_MUTE),
        ("VK_VOLUME_DOWN"         , VK_VOLUME_DOWN),
        ("VK_VOLUME_UP"           , VK_VOLUME_UP),
        ("VK_MEDIA_NEXT_TRACK"    , VK_MEDIA_NEXT_TRACK),
        ("VK_MEDIA_PREV_TRACK"    , VK_MEDIA_PREV_TRACK),
        ("VK_MEDIA_STOP"          , VK_MEDIA_STOP),
        ("VK_MEDIA_PLAY_PAUSE"    , VK_MEDIA_PLAY_PAUSE),
        ("VK_LAUNCH_MEDIA_SELECT" , VK_LAUNCH_MEDIA_SELECT),
        ("VK_LAUNCH_MAIL"         , VK_LAUNCH_MAIL),
        ("VK_LAUNCH_APP1"         , VK_LAUNCH_APP1),
        ("VK_LAUNCH_APP2"         , VK_LAUNCH_APP2),
        ("VK_OEM_PLUS"            , VK_OEM_PLUS),
        ("VK_OEM_COMMA"           , VK_OEM_COMMA),
        ("VK_OEM_MINUS"           , VK_OEM_MINUS),
        ("VK_OEM_PERIOD"          , VK_OEM_PERIOD),
        ("VK_OEM_1"               , VK_OEM_1),
        ("VK_OEM_RESET"           , VK_OEM_RESET),
        ("VK_OEM_JUMP"            , VK_OEM_JUMP),
        ("VK_OEM_PA1"             , VK_OEM_PA1),
    ];
    for (key, value) in num_constant {
        map.insert(
            key.to_ascii_uppercase(),
            Object::BuiltinConst(Box::new(Object::Num(value.into())))
        );
    }
}

const VK_A: i32                   = 65;
const VK_B: i32                   = 66;
const VK_C: i32                   = 67;
const VK_D: i32                   = 68;
const VK_E: i32                   = 69;
const VK_F: i32                   = 70;
const VK_G: i32                   = 71;
const VK_H: i32                   = 72;
const VK_I: i32                   = 73;
const VK_J: i32                   = 74;
const VK_K: i32                   = 75;
const VK_L: i32                   = 76;
const VK_M: i32                   = 77;
const VK_N: i32                   = 78;
const VK_O: i32                   = 79;
const VK_P: i32                   = 80;
const VK_Q: i32                   = 81;
const VK_R: i32                   = 82;
const VK_S: i32                   = 83;
const VK_T: i32                   = 84;
const VK_U: i32                   = 85;
const VK_V: i32                   = 86;
const VK_W: i32                   = 87;
const VK_X: i32                   = 88;
const VK_Y: i32                   = 89;
const VK_Z: i32                   = 90;
const VK_0: i32                   = 48;
const VK_1: i32                   = 49;
const VK_2: i32                   = 50;
const VK_3: i32                   = 51;
const VK_4: i32                   = 52;
const VK_5: i32                   = 53;
const VK_6: i32                   = 54;
const VK_7: i32                   = 55;
const VK_8: i32                   = 56;
const VK_9: i32                   = 57;
const VK_START: i32               = 92;
const VK_BACK: i32                = 8;
const VK_TAB: i32                 = 9;
const VK_CLEAR: i32               = 12;
const VK_ESC: i32                 = 27;
const VK_ESCAPE: i32              = 27;
const VK_RETURN: i32              = 13;
const VK_ENTER: i32               = 13;
const VK_RRETURN: i32             = 901;
const VK_SHIFT: i32               = 16;
const VK_RSHIFT: i32              = 161;
const VK_WIN: i32                 = 91;
const VK_RWIN: i32                = 92;
const VK_ALT: i32                 = 18;
const VK_MENU: i32                = 18;
const VK_RALT: i32                = 165;
const VK_CTRL: i32                = 17;
const VK_CONTROL: i32             = 17;
const VK_RCTRL: i32               = 163;
const VK_PAUSE: i32               = 19;
const VK_CAPITAL: i32             = 20;
const VK_KANA: i32                = 21;
const VK_FINAL: i32               = 24;
const VK_KANJI: i32               = 25;
const VK_CONVERT: i32             = 28;
const VK_NONCONVERT: i32          = 29;
const VK_ACCEPT: i32              = 30;
const VK_MODECHANGE: i32          = 31;
const VK_SPACE: i32               = 32;
const VK_PRIOR: i32               = 33;
const VK_NEXT: i32                = 34;
const VK_END: i32                 = 35;
const VK_HOME: i32                = 36;
const VK_LEFT: i32                = 37;
const VK_UP: i32                  = 38;
const VK_RIGHT: i32               = 39;
const VK_DOWN: i32                = 40;
const VK_SELECT: i32              = 41;
const VK_PRINT: i32               = 42;
const VK_EXECUTE: i32             = 43;
const VK_SNAPSHOT: i32            = 44;
const VK_INSERT: i32              = 45;
const VK_DELETE: i32              = 46;
const VK_HELP: i32                = 47;
const VK_APPS: i32                = 93;
const VK_MULTIPLY: i32            = 106;
const VK_ADD: i32                 = 107;
const VK_SEPARATOR: i32           = 108;
const VK_SUBTRACT: i32            = 109;
const VK_DECIMAL: i32             = 110;
const VK_DIVIDE: i32              = 111;
const VK_NUMPAD0: i32             = 96;
const VK_NUMPAD1: i32             = 97;
const VK_NUMPAD2: i32             = 98;
const VK_NUMPAD3: i32             = 99;
const VK_NUMPAD4: i32             = 100;
const VK_NUMPAD5: i32             = 101;
const VK_NUMPAD6: i32             = 102;
const VK_NUMPAD7: i32             = 103;
const VK_NUMPAD8: i32             = 104;
const VK_NUMPAD9: i32             = 105;
const VK_F1: i32                  = 112;
const VK_F2: i32                  = 113;
const VK_F3: i32                  = 114;
const VK_F4: i32                  = 115;
const VK_F5: i32                  = 116;
const VK_F6: i32                  = 117;
const VK_F7: i32                  = 118;
const VK_F8: i32                  = 119;
const VK_F9: i32                  = 120;
const VK_F10: i32                 = 121;
const VK_F11: i32                 = 122;
const VK_F12: i32                 = 123;
const VK_NUMLOCK: i32             = 144;
const VK_SCROLL: i32              = 145;
const VK_PLAY: i32                = 250;
const VK_ZOOM: i32                = 251;
const VK_SLEEP: i32               = 95;
const VK_BROWSER_BACK: i32        = 166;
const VK_BROWSER_FORWARD: i32     = 167;
const VK_BROWSER_REFRESH: i32     = 168;
const VK_BROWSER_STOP: i32        = 169;
const VK_BROWSER_SEARCH: i32      = 170;
const VK_BROWSER_FAVORITES: i32   = 171;
const VK_BROWSER_HOME: i32        = 172;
const VK_VOLUME_MUTE: i32         = 173;
const VK_VOLUME_DOWN: i32         = 174;
const VK_VOLUME_UP: i32           = 175;
const VK_MEDIA_NEXT_TRACK: i32    = 176;
const VK_MEDIA_PREV_TRACK: i32    = 177;
const VK_MEDIA_STOP: i32          = 178;
const VK_MEDIA_PLAY_PAUSE: i32    = 179;
const VK_LAUNCH_MEDIA_SELECT: i32 = 181;
const VK_LAUNCH_MAIL: i32         = 180;
const VK_LAUNCH_APP1: i32         = 182;
const VK_LAUNCH_APP2: i32         = 183;
const VK_OEM_PLUS: i32            = 187;
const VK_OEM_COMMA: i32           = 188;
const VK_OEM_MINUS: i32           = 189;
const VK_OEM_PERIOD: i32          = 190;
const VK_OEM_1: i32               = 186;
const VK_OEM_RESET: i32           = 233;
const VK_OEM_JUMP: i32            = 234;
const VK_OEM_PA1: i32             = 235;
