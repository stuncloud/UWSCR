use windows::{
    Win32::{
        Foundation::{HWND, RECT, WPARAM, POINT},
        System::{
            Com::{CLSCTX_ALL, CoCreateInstance},
            Variant::{VARIANT, VT_I4},
        },
        UI::{
            Accessibility::{
                CUIAutomation, IUIAutomation, 
                IUIAutomationCondition, IUIAutomationElement, IUIAutomationElementArray, 
                TreeScope_Children, TreeScope_Descendants, 
                UIA_ControlTypePropertyId,
                UIA_CONTROLTYPE_ID, UIA_ScrollBarControlTypeId, UIA_SliderControlTypeId, UIA_WindowControlTypeId,
                UIA_PATTERN_ID, UIA_RangeValuePatternId, 
                IUIAutomationRangeValuePattern, 
                OrientationType, OrientationType_Horizontal, OrientationType_Vertical, 
            },
            WindowsAndMessaging::{
                GetScrollInfo, SCROLLINFO, SB_CTL, SB_HORZ, SB_VERT, SIF_ALL, SIF_POS,
                SendMessageW, WM_HSCROLL, WM_VSCROLL, SB_THUMBTRACK,
            },
        },
        Graphics::Gdi::ScreenToClient,
    }, 
    core::{Result as WinResult, ComInterface}
};

pub trait UiaElementImpl {
    fn element(&self) -> &IUIAutomationElement;
    // fn title(&self) -> Option<BSTR> {
    //     unsafe { self.element().CurrentName().ok() }
    // }
    // fn title_contains(&self, p: &str) -> bool {
    //     self.title().is_some_and(|b| b.to_string().find_ignore_ascii_case(p).is_some() )
    // }
    // fn class_name(&self) -> Option<BSTR> {
    //     unsafe {
    //         self.element().CurrentClassName().ok()
    //     }
    // }
    // fn get_children(&self, condition: &IUIAutomationCondition) -> WinResult<UIAElementArray> {
    //     unsafe {
    //         self.element().FindAll(TreeScope_Children, condition)
    //             .map(Into::into)
    //     }
    // }
    fn get_descendents(&self, condition: &IUIAutomationCondition) -> WinResult<UIAElementArray> {
        unsafe {
            self.element().FindAll(TreeScope_Descendants, condition)
                .map(Into::into)
        }
    }
}

/// ウィンドウ
pub struct UiaWindow {
    inner: IUIAutomationElement,
}
impl UiaWindow {
    // fn new(inner: IUIAutomationElement) -> Self {
    //     Self::from(inner)
    // }
    pub fn from_hwnd(auto: &UIAutomation, hwnd: HWND) -> WinResult<Self> {
        auto.get_window_from_hwnd(hwnd)
    }
    // pub fn hwnd(&self) -> WinResult<HWND> {
    //     unsafe {
    //         self.inner.CurrentNativeWindowHandle()
    //     }
    // }
}
impl From<IUIAutomationElement> for UiaWindow {
    fn from(inner: IUIAutomationElement) -> Self {
        Self { inner }
    }
}
impl UiaElementImpl for UiaWindow {
    fn element(&self) -> &IUIAutomationElement {
        &self.inner
    }
}

/// エレメント
#[derive(Debug, Clone)]
pub struct UiaElement {
    inner: IUIAutomationElement,
}
impl UiaElement {
    fn get_rect(&self) -> Option<RECT> {
        unsafe {
            self.inner.CurrentBoundingRectangle().ok()
        }
    }
    fn hwnd(&self) -> WinResult<HWND> {
        unsafe { self.inner.CurrentNativeWindowHandle() }
    }
    fn parent_hwnd(&self) -> WinResult<HWND> {
        let auto = UIAutomation::new()?;
        let parent = auto.get_parent_elemnt(self)?;
        parent.hwnd()
    }
    // fn control_type_is(&self, id: UIA_CONTROLTYPE_ID) -> bool {
    //     self.control_type_id()
    //         .is_some_and(|cur| cur == id)
    // }
    fn control_type_id(&self) -> Option<UIA_CONTROLTYPE_ID> {
        unsafe {
            self.inner.CurrentControlType().ok()
        }
    }
    // fn get_value(&self, propertyid: UIA_PROPERTY_ID) -> Option<VARIANT> {
    //     unsafe {
    //         self.inner.GetCurrentPropertyValue(propertyid).ok()
    //     }
    // }
    fn as_pattern<T: ComInterface>(&self, patternid: UIA_PATTERN_ID) -> Option<T> {
        unsafe {
            self.inner.GetCurrentPatternAs::<T>(patternid).ok()
        }
    }
    fn get_orientation(&self) -> Option<OrientationType> {
        unsafe {
            self.inner.CurrentOrientation().ok()
        }
    }
    pub fn as_slider(&self) -> Option<UiaSlider> {
        UiaSlider::new(&self)
    }
    
}
impl UiaElementImpl for UiaElement {
    fn element(&self) -> &IUIAutomationElement {
        &self.inner
    }
}
impl From<IUIAutomationElement> for UiaElement {
    fn from(inner: IUIAutomationElement) -> Self {
        Self { inner }
    }
}

pub struct UIAutomation {
    inner: IUIAutomation,
}
impl UIAutomation {
    pub fn new() -> WinResult<Self> {
        let inner = unsafe {
            CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL)?
        };
        Ok(Self { inner })
    }
    pub fn true_condition(&self) -> WinResult<IUIAutomationCondition> {
        unsafe {
            self.inner.CreateTrueCondition()
        }
    }
    fn value_from_type_id(type_id: UIA_CONTROLTYPE_ID) -> VARIANT {
        unsafe {
            let mut variant = VARIANT::default();
            let variant00 = &mut variant.Anonymous.Anonymous;
            variant00.vt = VT_I4;
            variant00.Anonymous.lVal = type_id.0 as _;
            variant
        }
    }
    fn prop_condition(&self, type_id: UIA_CONTROLTYPE_ID) -> WinResult<IUIAutomationCondition> {
        unsafe {
            let value = Self::value_from_type_id(type_id);
            self.inner.CreatePropertyCondition(UIA_ControlTypePropertyId, value)
        }
    }
    fn or_condition(&self, condition1: &IUIAutomationCondition, condition2: &IUIAutomationCondition) -> WinResult<IUIAutomationCondition> {
        unsafe {
            self.inner.CreateOrCondition(condition1, condition2)
        }
    }
    /// 複数のUIA_CONTROLTYPE_IDから得られるPropertyConditionをOR連結したconditionを返す
    pub fn controls_condition(&self, controls: &[UIA_CONTROLTYPE_ID]) -> Option<IUIAutomationCondition> {
        let iter = controls.iter();
        iter.fold(None, |condition, type_id| {
            let other = self.prop_condition(*type_id).ok()?;
            match condition {
                Some(condition) => {
                    self.or_condition(&condition, &other).ok()
                },
                None => Some(other),
            }
        })
    }
    /// スクロールバーおよびトラックバーの探索条件
    pub fn slider_condition(&self) -> Option<IUIAutomationCondition> {
        self.controls_condition(&[UIA_ScrollBarControlTypeId, UIA_SliderControlTypeId])
    }
    // fn get_windows(&self) -> WinResult<UIAElementArray> {
    //     unsafe {
    //         let condition = self.prop_condition(UIA_WindowControlTypeId)?;
    //         let desktop = self.inner.GetRootElement()?;
    //         desktop.FindAll(TreeScope_Descendants, &condition)
    //             .map(UIAElementArray::new)
    //     }
    // }
    fn get_window_from_hwnd(&self, hwnd: HWND) -> WinResult<UiaWindow> {
        unsafe {
            self.inner.ElementFromHandle(hwnd).map(Into::into)
        }
    }
    fn get_parent_elemnt(&self, elem: &UiaElement) -> WinResult<UiaElement> {
        unsafe {
            let condition = self.true_condition()?;
            let walker = self.inner.CreateTreeWalker(&condition)?;
            walker.GetParentElement(&elem.inner)
                .map(UiaElement::from)
        }
    }
}
pub struct UIAElementArray {
    inner: IUIAutomationElementArray,
    index: i32,
}
impl UIAElementArray {
    fn new(array: IUIAutomationElementArray) -> Self {
        let index = 0;
        Self { inner: array, index, }
    }
}
impl Iterator for UIAElementArray {
    type Item = IUIAutomationElement;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let element = self.inner.GetElement(self.index).ok()?;
            self.index += 1;
            Some(element)
        }
    }
}
impl From<IUIAutomationElementArray> for UIAElementArray {
    fn from(array: IUIAutomationElementArray) -> Self {
        Self::new(array)
    }
}

// /// ウィンドウを返すイテレータ
// pub struct UiaWindowIter {
//     windows: UIAElementArray,
// }
// impl UiaWindowIter {
//     pub fn new(auto: &UIAutomation) -> WinResult<Self> {
//         let windows = auto.get_windows()?;
//         Ok(Self { windows, })
//     }
// }
// impl Iterator for UiaWindowIter {
//     type Item = UiaWindow;

//     fn next(&mut self) -> Option<Self::Item> {
//         self.windows.next().map(UiaWindow::new)
//     }
// }

/// ウィンドウのエレメントを返すイテレータ
pub struct UiaElementIter {
    array: UIAElementArray,
}
impl UiaElementIter {
    pub fn new(condition: &IUIAutomationCondition, window: &UiaWindow) -> WinResult<Self> {
        let array = window.get_descendents(condition)?;
        Ok(Self { array })
    }
}
impl Iterator for UiaElementIter {
    type Item = UiaElement;

    fn next(&mut self) -> Option<Self::Item> {
        self.array.next().map(Into::into)
    }
}

#[derive(Debug)]
pub struct ScrollBar {
    hwnd: HWND,
    info: SCROLLINFO,
}
impl ScrollBar {
    fn info(&self) -> &SCROLLINFO {
        &self.info
    }
    fn info_mut(&mut self) -> &mut SCROLLINFO {
        &mut self.info
    }
}
#[derive(Debug)]
pub enum SliderKind {
    ScrollBar(ScrollBar),
    TrackBar(IUIAutomationRangeValuePattern),
}
impl SliderKind {
    fn new_trackbar(pattern: IUIAutomationRangeValuePattern) -> Self {
        Self::TrackBar(pattern)
    }
    fn new_scrollbar(info: SCROLLINFO, hwnd: HWND) -> Self {
        Self::ScrollBar(ScrollBar { hwnd, info })
    }
    fn max(&self) -> Option<f64> {
        match self {
            SliderKind::ScrollBar(sb) => {
                Some(sb.info().nMax.into())
            },
            SliderKind::TrackBar(range) => unsafe {
                range.CurrentMaximum().ok()
            },
        }
    }
    fn min(&self) -> Option<f64> {
        match self {
            SliderKind::ScrollBar(sb) => {
                Some(sb.info().nMin.into())
            },
            SliderKind::TrackBar(range) => unsafe {
                range.CurrentMinimum().ok()
            },
        }
    }
    fn page(&self) -> Option<f64> {
        match self {
            SliderKind::ScrollBar(sb) => {
                Some(sb.info().nPage.into())
            },
            SliderKind::TrackBar(range) => unsafe {
                range.CurrentLargeChange().ok()
            },
        }
    }
    fn value(&self) -> Option<f64> {
        match self {
            SliderKind::ScrollBar(sb) => {
                Some(sb.info().nPos.into())
            },
            SliderKind::TrackBar(range) => unsafe {
                range.CurrentValue().ok()
            },
        }
    }
    fn update_scrollinfo(hwnd: HWND, dir: &SliderDir, info: &mut SCROLLINFO) {
        let nbar = match dir {
            SliderDir::Horizontal => SB_HORZ,
            SliderDir::Vertical => SB_VERT,
            SliderDir::None => SB_CTL,
        };
        unsafe { let _ = GetScrollInfo(hwnd, nbar, info); }
    }
    fn set_value(&mut self, dir: &SliderDir, value: f64) -> bool {
        match self {
            SliderKind::ScrollBar(sb) => unsafe {
                let hwnd = sb.hwnd;
                let msg = match dir {
                    SliderDir::Horizontal => WM_HSCROLL,
                    SliderDir::Vertical => WM_VSCROLL,
                    SliderDir::None => return false,
                };
                let info = sb.info_mut();
                let npos = (value as i32).max(info.nMin).min(info.nMax - info.nPage as i32 + 1);
                let wparam = (SB_THUMBTRACK.0 | (npos&0xFFFF) << 16) as usize;
                SendMessageW(hwnd, msg, WPARAM(wparam), None);
                Self::update_scrollinfo(hwnd, dir, info);
                info.nPos == npos
            },
            SliderKind::TrackBar(range) => unsafe {
                range.SetValue(value).is_ok()
            },
        }
    }
}
/// スクロールバーやトラックバー
#[derive(Debug)]
pub struct UiaSlider {
    rect: RECT,
    dir: SliderDir,
    kind: SliderKind,
    // scroll: Option<IUIAutomationScrollPattern>,
}
impl UiaSlider {
    #[allow(non_upper_case_globals)]
    fn new(elem: &UiaElement) -> Option<Self> {
        match elem.control_type_id()? {
            UIA_ScrollBarControlTypeId => {
                let rect = elem.get_rect()?;
                let dir = Self::slider_dir(elem)?;
                let hwnd = elem.parent_hwnd().ok()?;
                let info = Self::get_scroll_info(hwnd, &dir).ok()?;
                let kind = SliderKind::new_scrollbar(info, hwnd);
                Some(Self { rect, dir, kind })
            },
            UIA_SliderControlTypeId => {
                let rect = elem.get_rect()?;
                let dir = Self::slider_dir(elem)?;
                let range = elem.as_pattern(UIA_RangeValuePatternId)?;
                let kind = SliderKind::new_trackbar(range);
                Some(Self { rect, dir, kind, })
            },
            _ => None
        }
    }
    fn slider_dir(elem: &UiaElement) -> Option<SliderDir> {
        let orientation = elem.get_orientation()?;
        Some(SliderDir::from(orientation))
    }
    fn get_scroll_info(hwnd: HWND, dir: &SliderDir) -> WinResult<SCROLLINFO> {
        unsafe {
            let mut info = SCROLLINFO {
                cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
                fMask: SIF_ALL,
                ..Default::default()
            };
            let nbar = match dir {
                SliderDir::Horizontal => SB_HORZ,
                SliderDir::Vertical => SB_VERT,
                SliderDir::None => SB_CTL,
            };
            GetScrollInfo(hwnd, nbar, &mut info)
                .map(|_| info)
        }
    }
    // pub fn scroll_info(&self) -> Option<&SCROLLINFO> {
    //     match &self.kind {
    //         SliderKind::ScrollBar(sb) => Some(sb.info()),
    //         SliderKind::TrackBar(_) => None,
    //     }
    // }
    pub fn _x(&self) -> i32 {
        self.rect.left
    }
    pub fn clx(&self, hwnd: HWND) -> i32 {
        unsafe {
            let mut point = POINT { x: self.rect.left, y: 0 };
            ScreenToClient(hwnd, &mut point);
            point.x
        }
    }
    pub fn _y(&self) -> i32 {
        self.rect.top
    }
    pub fn cly(&self, hwnd: HWND) -> i32 {
        unsafe {
            let mut point = POINT { x: 0, y: self.rect.top };
            ScreenToClient(hwnd, &mut point);
            point.y
        }
    }
    pub fn dir(&self) -> &SliderDir {
        &self.dir
    }
    pub fn max(&self) -> Option<f64> {
        self.kind.max()
    }
    pub fn min(&self) -> Option<f64> {
        self.kind.min()
    }
    pub fn value(&self) -> Option<f64> {
        self.kind.value()
    }
    pub fn page(&self) -> Option<f64> {
        self.kind.page()
    }
    pub fn set_value(&mut self, value: f64) -> bool {
        self.kind.set_value(&self.dir, value)
    }
}

#[derive(Debug)]
pub enum SliderDir {
    Horizontal,
    Vertical,
    None,
}
impl From<OrientationType> for SliderDir {
    #[allow(non_upper_case_globals)]
    fn from(value: OrientationType) -> Self {
        match value {
            OrientationType_Horizontal => Self::Horizontal,
            OrientationType_Vertical => Self::Vertical,
            _ => Self::None,
        }
    }
}

// /// 大文字小文字を無視して部分一致するかどうかを調べる
// pub trait FindIgnoreAsciiCase 
// where Self: std::ops::Deref<Target = str> {
//     /// 大文字小文字を無視した部分一致探索\
//     /// 一致があった場合その位置を返す
//     fn find_ignore_ascii_case(&self, other: &str) -> Option<usize> {
//         let bytes = other.as_bytes();
//         self.as_bytes()
//             .windows(bytes.len())
//             .enumerate()
//             .find_map(|(i, win)| win.eq_ignore_ascii_case(bytes).then_some(i) )
//     }
// }
// impl<T: std::ops::Deref<Target = str>> FindIgnoreAsciiCase for T {}
