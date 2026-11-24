mod app;

use gpui_kit::component::theme::{Theme, ThemeMode};
use gpui_kit::component::TitleBar;
use gpui_kit::*;

fn main() {
    gpui_kit::application().with_assets(gpui_kit::assets::Assets).run(move |cx| {
        gpui_kit::init(cx);
        Theme::change(ThemeMode::Dark, None, cx);
        // Single-window app: closing it quits (gpui keeps running otherwise on Linux).
        cx.on_window_closed(|cx, _| cx.quit()).detach();
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(None, size(px(1100.), px(760.)), cx)),
            window_min_size: Some(size(px(900.), px(560.))),
            ..TitleBar::window_options()
        };
        // `espflashtool <build dir | manifest | .bin>` opens it straight away.
        let initial = std::env::args_os().nth(1).map(std::path::PathBuf::from);
        gpui_kit::open_window(options, cx, |window, cx| {
            cx.new(|cx| {
                let mut app = app::FlashApp::new(window, cx);
                if let Some(p) = initial {
                    app.open_path(p, window, cx);
                }
                app
            })
        })
        .expect("failed to open window");
    });
}
