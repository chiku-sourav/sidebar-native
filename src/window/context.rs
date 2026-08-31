pub mod primitives;
pub mod text;

pub use crate::window::theme::ThemePalette;
pub use text::estimate_wrapped_lines;

use windows::Win32::Graphics::Gdi::HFONT;

pub struct RenderContext {
    pub pal: ThemePalette,
    pub font_scale: f32,
    pub hfont_title: HFONT,
    pub hfont_big_pct: HFONT,
    pub hfont_clock: HFONT,
    pub hfont_tag: HFONT,
    pub hfont_header: HFONT,
    pub hfont_label: HFONT,
    pub hfont_value: HFONT,
    pub hfont_caption: HFONT,
}
