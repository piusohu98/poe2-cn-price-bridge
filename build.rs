#[cfg(windows)]
fn main() {
    let mut resource = winresource::WindowsResource::new();
    resource.set_icon("assets/app.ico");
    resource.set("FileDescription", "QingPrice POE2 CN price checker");
    resource.set("ProductName", "QingPrice POE2");
    resource.set("CompanyName", "zijin");
    resource.set("OriginalFilename", "QingPricePOE2.exe");
    resource.set("LegalCopyright", "Copyright (c) 2026 zijin");
    resource
        .compile()
        .expect("failed to compile Windows resources");
}

#[cfg(not(windows))]
fn main() {}
