use winres::WindowsResource;

fn main() {
    if cfg!(target_os = "windows") {
        let mut res = WindowsResource::new();
        // res.set_icon("test.ico");

        let desc = match std::env::var("TARGET").unwrap().as_str() {
            "x86_64-pc-windows-msvc" => {
                res.set_icon(r#"..\icons\UWSC\ico\MAINICON_0016-0256_light.ico"#);
                match (cfg!(feature="chkimg"), cfg!(feature="gui")) {
                    (true, false) => "UWSCR x64",
                    (false, false) => "UWSCR x64 w/o chkimg",
                    (true, true) => "UWSCR x64 GUI",
                    (false, true) => "UWSCR x64 GUI w/o chkimg",
                }
            },
            "i686-pc-windows-msvc" => {
                res.set_icon(r#"..\icons\UWSC\ico\MAINICON_0016-0256_dark.ico"#);
                match (cfg!(feature="chkimg"), cfg!(feature="gui")) {
                    (true, false) => "UWSCR x86",
                    (false, false) => "UWSCR x86 w/o chkimg",
                    (true, true) => "UWSCR x86 GUI",
                    (false, true) => "UWSCR x86 GUI w/o chkimg",
                }
            },
            _ => "UWSCR"
        };
        res.set("FileDescription", desc);
        res.set("LegalCopyright", "Joey Takahashi a.k.a. stuncloud");
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
    <dependency>
        <dependentAssembly>
            <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls" version="6.0.0.0" processorArchitecture="*" publicKeyToken="6595b64144ccf1df" language="*" />
        </dependentAssembly>
    </dependency>
</assembly>
        "#);
        res.compile().unwrap();
    }
}