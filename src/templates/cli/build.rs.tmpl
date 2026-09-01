// Embeds the Windows executable icon and version metadata, so the binary is
// recognisable in Explorer and the task manager rather than showing the
// default cog.
fn main() {
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=assets/icon.ico");
        if std::path::Path::new("assets/icon.ico").exists() {
            winresource::WindowsResource::new()
                .set_icon("assets/icon.ico")
                .compile()
                .expect("failed to embed the Windows resources");
        }
    }
}
