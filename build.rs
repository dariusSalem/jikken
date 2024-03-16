use {
    std::{env, io},
    winres::WindowsResource,
};

fn main() -> io::Result<()> {
    if env::var_os("CARGO_CFG_WINDOWS").is_some() {
        WindowsResource::new()
            // This path can be absolute, or relative to your crate root.
            .set_icon("favicon.ico")
            .set("InternalName", "Jikken CLI")
            .set("OriginalFilename", "jk.exe")
            .set("ProductName", "Jikken")
            .set("FileDescription", "Jikken CLI Tool")
            .set("CompanyName", "JikkenIO")
            .compile()?;
    }
    Ok(())
}
