#[cfg(windows)]
fn main() {
    let mut resource = winresource::WindowsResource::new();
    resource.set_icon("assets/app.ico");
    resource.set("FileDescription", "流放2国服查价助手");
    resource.set("ProductName", "流放2查价助手");
    resource.set("CompanyName", "zijin");
    resource.set("OriginalFilename", "POE2PriceHelper.exe");
    resource.set("LegalCopyright", "Copyright (c) 2026 zijin");
    resource
        .compile()
        .expect("failed to compile Windows resources");
}

#[cfg(not(windows))]
fn main() {}
