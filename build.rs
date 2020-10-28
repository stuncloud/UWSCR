use winres::WindowsResource;

fn main() {
    if cfg!(target_os = "windows") {
        let mut res = WindowsResource::new();
        // res.set_icon("test.ico");
        res.set("FileDescription", "uwsc compatible script execution tool");
        res.compile().unwrap();
    }
}