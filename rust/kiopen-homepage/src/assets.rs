use std::sync::OnceLock;

const TEMPLATE: &str = include_str!("../assets/editor.html");
const EDITOR_CSS: &str = include_str!("../assets/editor.css");
const MODEL_JS: &str = include_str!("../assets/js/model.js");
const RENDER_JS: &str = include_str!("../assets/js/render.js");
const PREVIEW_JS: &str = include_str!("../assets/js/preview.js");
const APP_JS: &str = include_str!("../assets/js/app.js");
const CSS_MARKER: &str = "<!--APP_CSS-->";
const JS_MARKER: &str = "<!--APP_JS-->";

static EDITOR: OnceLock<String> = OnceLock::new();

pub(crate) fn editor_html() -> &'static str {
    EDITOR.get_or_init(assemble).as_str()
}

fn assemble() -> String {
    if TEMPLATE.matches(CSS_MARKER).count() != 1 {
        eprintln!("错误：assets/editor.html 必须且只能包含一个 {CSS_MARKER} 标记");
    }
    if TEMPLATE.matches(JS_MARKER).count() != 1 {
        eprintln!("错误：assets/editor.html 必须且只能包含一个 {JS_MARKER} 标记");
    }

    let mut javascript = String::new();
    for source in [MODEL_JS, RENDER_JS, PREVIEW_JS, APP_JS] {
        javascript.push_str(source);
        javascript.push('\n');
    }
    TEMPLATE
        .replacen(CSS_MARKER, EDITOR_CSS, 1)
        .replacen(JS_MARKER, &javascript, 1)
}
