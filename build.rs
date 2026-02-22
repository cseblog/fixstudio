fn main() {
    // On Windows, embed an application icon and manifest for DPI awareness.
    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_manifest(
            r#"
            <assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
                <application xmlns="urn:schemas-microsoft-com:asm.v3">
                    <windowsSettings>
                        <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
                        <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
                    </windowsSettings>
                </application>
            </assembly>
            "#,
        );
        res.set_icon("assets/icon.ico");
        res.compile().expect("Failed to compile Windows resources");
    }
}
