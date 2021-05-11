use winres::WindowsResource;

fn main() {
    if cfg!(target_os = "windows") {
        let mut res = WindowsResource::new();
        // res.set_icon("test.ico");

        let desc = match std::env::var("TARGET").unwrap().as_str() {
            "x86_64-pc-windows-msvc" => "UWSCR x64",
            "i686-pc-windows-msvc" => "UWSCR x86",
            _ => "UWSCR"
        };
        res.set("FileDescription", &desc);
        res.set("LegalCopyright", "Joey Takahashi a.k.a. stuncloud");
        res.set_icon(r#".\icons\UWSC\ico\MAINICON_0016-0256_light.ico"#);
        res.set_manifest(r#"
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
    <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
        <application>
            <!-- Windows 10 -->
            <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/>
            <!-- Windows 8.1 -->
            <supportedOS Id="{1f676c76-80e1-4239-95bb-83d0f6d0da78}"/>
            <!-- Windows 8 -->
            <supportedOS Id="{4a2f28e3-53b9-4441-ba9c-d69d4a4a6e38}"/>
            <!-- Windows 7 -->
            <supportedOS Id="{35138b9a-5d96-4fbd-8e2d-a2440225f93a}"/>
            <!-- Windows Vista -->
            <supportedOS Id="{e2011457-1546-43c5-a5fe-008deee3d3f0}"/>
        </application>
    </compatibility>
</assembly>
        "#);
        res.compile().unwrap();
    }
    // windows-rs
    windows::build!(
        Windows::Win32::KeyboardAndMouseInput::{
            keybd_event, KEYBD_EVENT_FLAGS, MapVirtualKeyW,
        },
        Windows::Win32::WindowsAndMessaging::{
            GetCursorPos,
        },
        Windows::Win32::DisplayDevices::{
            POINT
        },
    )
}