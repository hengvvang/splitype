//! Embedded Lexend fonts loader.

use std::borrow::Cow;
use gpui::App;

pub(super) fn load_fonts(cx: &App) -> gpui::Result<()> {
    let fonts: Vec<Cow<'static, [u8]>> = vec![
        Cow::Borrowed(include_bytes!(
            "../../../../assets/fonts/Lexend-Thin.ttf"
        )),
        Cow::Borrowed(include_bytes!(
            "../../../../assets/fonts/Lexend-ExtraLight.ttf"
        )),
        Cow::Borrowed(include_bytes!(
            "../../../../assets/fonts/Lexend-Light.ttf"
        )),
        Cow::Borrowed(include_bytes!(
            "../../../../assets/fonts/Lexend-Regular.ttf"
        )),
        Cow::Borrowed(include_bytes!(
            "../../../../assets/fonts/Lexend-Medium.ttf"
        )),
        Cow::Borrowed(include_bytes!(
            "../../../../assets/fonts/Lexend-SemiBold.ttf"
        )),
        Cow::Borrowed(include_bytes!(
            "../../../../assets/fonts/Lexend-Bold.ttf"
        )),
        Cow::Borrowed(include_bytes!(
            "../../../../assets/fonts/Lexend-ExtraBold.ttf"
        )),
        Cow::Borrowed(include_bytes!(
            "../../../../assets/fonts/Lexend-Black.ttf"
        )),
    ];
    cx.text_system().add_fonts(fonts)
}
