use gpui::App;

pub trait DocumentSource {
    fn source_text(&self, cx: &App) -> String;
}

pub trait EditorDocument {
    fn serialize_markdown(&self, cx: &App) -> String;
}
