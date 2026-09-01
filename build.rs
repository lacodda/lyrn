// Embeds the Windows executable icon and version metadata. The icon is the S
// tile of the lacodda line mark, exported to a multi-size .ico so Explorer
// picks the right resolution for each view.
fn main() {
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=assets/icon.ico");
        winresource::WindowsResource::new()
            .set_icon("assets/icon.ico")
            .compile()
            .expect("failed to embed the Windows resources");
    }
}
