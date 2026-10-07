// Embeds the Windows executable icon, so the binary is recognisable in
// Explorer and the task manager rather than showing the default one.
//
// Unconditional on purpose: a package without its icon fails to build here,
// rather than shipping a binary nobody notices is missing its mark.
fn main() {
    #[cfg(windows)]
    {
        const ICON: &str = "assets/icon.ico";
        println!("cargo:rerun-if-changed={ICON}");
        winresource::WindowsResource::new().set_icon(ICON).compile().expect("failed to embed the Windows resources");
    }
}
