use strum_macros::{EnumString, EnumVariantNames, EnumProperty};
use num_derive::{ToPrimitive};

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, EnumVariantNames, ToPrimitive)]
pub enum VirtualKeyCodes {
    VK_A                   = 65,
    VK_B                   = 66,
    VK_C                   = 67,
    VK_D                   = 68,
    VK_E                   = 69,
    VK_F                   = 70,
    VK_G                   = 71,
    VK_H                   = 72,
    VK_I                   = 73,
    VK_J                   = 74,
    VK_K                   = 75,
    VK_L                   = 76,
    VK_M                   = 77,
    VK_N                   = 78,
    VK_O                   = 79,
    VK_P                   = 80,
    VK_Q                   = 81,
    VK_R                   = 82,
    VK_S                   = 83,
    VK_T                   = 84,
    VK_U                   = 85,
    VK_V                   = 86,
    VK_W                   = 87,
    VK_X                   = 88,
    VK_Y                   = 89,
    VK_Z                   = 90,
    VK_0                   = 48,
    VK_1                   = 49,
    VK_2                   = 50,
    VK_3                   = 51,
    VK_4                   = 52,
    VK_5                   = 53,
    VK_6                   = 54,
    VK_7                   = 55,
    VK_8                   = 56,
    VK_9                   = 57,
    VK_BACK                = 8,
    VK_TAB                 = 9,
    VK_CLEAR               = 12,
    #[strum(props(alias="VK_ESCAPE"))]
    VK_ESC                 = 27,
    #[strum(props(alias="VK_ENTER"))]
    VK_RETURN              = 13,
    VK_RRETURN             = 901,
    VK_SHIFT               = 16,
    VK_RSHIFT              = 161,
    VK_WIN                 = 91,
    #[strum(props(alias="VK_RWIN"))]
    VK_START               = 92,
    #[strum(props(alias="VK_MENU"))]
    VK_ALT                 = 18,
    VK_RALT                = 165,
    #[strum(props(alias="VK_CONTROL"))]
    VK_CTRL                = 17,
    VK_RCTRL               = 163,
    VK_PAUSE               = 19,
    VK_CAPITAL             = 20,
    VK_KANA                = 21,
    VK_FINAL               = 24,
    VK_KANJI               = 25,
    VK_CONVERT             = 28,
    VK_NONCONVERT          = 29,
    VK_ACCEPT              = 30,
    VK_MODECHANGE          = 31,
    VK_SPACE               = 32,
    VK_PRIOR               = 33,
    VK_NEXT                = 34,
    VK_END                 = 35,
    VK_HOME                = 36,
    VK_LEFT                = 37,
    VK_UP                  = 38,
    VK_RIGHT               = 39,
    VK_DOWN                = 40,
    VK_SELECT              = 41,
    VK_PRINT               = 42,
    VK_EXECUTE             = 43,
    VK_SNAPSHOT            = 44,
    VK_INSERT              = 45,
    VK_DELETE              = 46,
    VK_HELP                = 47,
    VK_APPS                = 93,
    VK_MULTIPLY            = 106,
    VK_ADD                 = 107,
    VK_SEPARATOR           = 108,
    VK_SUBTRACT            = 109,
    VK_DECIMAL             = 110,
    VK_DIVIDE              = 111,
    VK_NUMPAD0             = 96,
    VK_NUMPAD1             = 97,
    VK_NUMPAD2             = 98,
    VK_NUMPAD3             = 99,
    VK_NUMPAD4             = 100,
    VK_NUMPAD5             = 101,
    VK_NUMPAD6             = 102,
    VK_NUMPAD7             = 103,
    VK_NUMPAD8             = 104,
    VK_NUMPAD9             = 105,
    VK_F1                  = 112,
    VK_F2                  = 113,
    VK_F3                  = 114,
    VK_F4                  = 115,
    VK_F5                  = 116,
    VK_F6                  = 117,
    VK_F7                  = 118,
    VK_F8                  = 119,
    VK_F9                  = 120,
    VK_F10                 = 121,
    VK_F11                 = 122,
    VK_F12                 = 123,
    VK_NUMLOCK             = 144,
    VK_SCROLL              = 145,
    VK_PLAY                = 250,
    VK_ZOOM                = 251,
    VK_SLEEP               = 95,
    VK_BROWSER_BACK        = 166,
    VK_BROWSER_FORWARD     = 167,
    VK_BROWSER_REFRESH     = 168,
    VK_BROWSER_STOP        = 169,
    VK_BROWSER_SEARCH      = 170,
    VK_BROWSER_FAVORITES   = 171,
    VK_BROWSER_HOME        = 172,
    VK_VOLUME_MUTE         = 173,
    VK_VOLUME_DOWN         = 174,
    VK_VOLUME_UP           = 175,
    VK_MEDIA_NEXT_TRACK    = 176,
    VK_MEDIA_PREV_TRACK    = 177,
    VK_MEDIA_STOP          = 178,
    VK_MEDIA_PLAY_PAUSE    = 179,
    VK_LAUNCH_MEDIA_SELECT = 181,
    VK_LAUNCH_MAIL         = 180,
    VK_LAUNCH_APP1         = 182,
    VK_LAUNCH_APP2         = 183,
    VK_OEM_PLUS            = 187,
    VK_OEM_COMMA           = 188,
    VK_OEM_MINUS           = 189,
    VK_OEM_PERIOD          = 190,
    VK_OEM_1               = 186,
    VK_OEM_2               = 191,
    VK_OEM_3               = 192,
    VK_OEM_4               = 219,
    VK_OEM_5               = 220,
    VK_OEM_6               = 221,
    VK_OEM_7               = 222,
    VK_OEM_8               = 223,
    VK_OEM_RESET           = 233,
    VK_OEM_JUMP            = 234,
    VK_OEM_PA1             = 235,
    VK_OEM_PA2             = 236,
    VK_OEM_PA3             = 237,
}

#[allow(non_camel_case_types)]
#[derive(Debug, EnumString, EnumProperty, EnumVariantNames, ToPrimitive)]
pub enum VirtualMouseButton {
    VK_LBUTTON = 1,
    VK_RBUTTON = 2,
    VK_MBUTTON = 4,
}