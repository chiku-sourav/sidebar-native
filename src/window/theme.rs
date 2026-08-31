use crate::config::AppTheme;
use windows::Win32::Foundation::COLORREF;

#[derive(Debug, Clone, Copy)]
pub struct ThemePalette {
    pub bg_window: COLORREF,
    pub bg_card: COLORREF,
    pub card_border: COLORREF,
    pub text_primary: COLORREF,
    pub text_secondary: COLORREF,
    pub text_muted: COLORREF,
    pub accent_teal: COLORREF,
    pub accent_purple: COLORREF,
    pub accent_cyan: COLORREF,
    pub accent_amber: COLORREF,
    pub accent_green: COLORREF,
    pub accent_red: COLORREF,
    pub progress_track: COLORREF,
    pub scroll_thumb: COLORREF,
}

impl ThemePalette {
    pub fn resolve(theme: AppTheme, is_system_dark: bool) -> Self {
        let is_dark = match theme {
            AppTheme::Auto => is_system_dark,
            AppTheme::DarkSlate | AppTheme::OledBlack | AppTheme::Nord | AppTheme::Cyberpunk => {
                true
            }
            AppTheme::LightMode => false,
        };

        match theme {
            AppTheme::LightMode => Self {
                bg_window: COLORREF(0x00F5F2EF),
                bg_card: COLORREF(0x00FFFFFF),
                card_border: COLORREF(0x00DDD5CC),
                text_primary: COLORREF(0x001C1A18),
                text_secondary: COLORREF(0x005A534E),
                text_muted: COLORREF(0x008E847C),
                accent_teal: COLORREF(0x00806600),
                accent_purple: COLORREF(0x00883377),
                accent_cyan: COLORREF(0x00996600),
                accent_amber: COLORREF(0x000088DD),
                accent_green: COLORREF(0x002E7D32),
                accent_red: COLORREF(0x003333D3),
                progress_track: COLORREF(0x00E2DCD5),
                scroll_thumb: COLORREF(0x00C4BCB4),
            },
            AppTheme::OledBlack => Self {
                bg_window: COLORREF(0x00000000),
                bg_card: COLORREF(0x00080808),
                card_border: COLORREF(0x001E1E1E),
                text_primary: COLORREF(0x00FFFFFF),
                text_secondary: COLORREF(0x00B0B0B0),
                text_muted: COLORREF(0x00707070),
                accent_teal: COLORREF(0x00E0A526),
                accent_purple: COLORREF(0x00D970B0),
                accent_cyan: COLORREF(0x00D8C040),
                accent_amber: COLORREF(0x0038BDF8),
                accent_green: COLORREF(0x004ADE80),
                accent_red: COLORREF(0x004755F8),
                progress_track: COLORREF(0x00151515),
                scroll_thumb: COLORREF(0x00404040),
            },
            AppTheme::Nord => Self {
                bg_window: COLORREF(0x003B2E24),
                bg_card: COLORREF(0x0043342E),
                card_border: COLORREF(0x0054433B),
                text_primary: COLORREF(0x00ECEFF4),
                text_secondary: COLORREF(0x00D8DEE9),
                text_muted: COLORREF(0x00928072),
                accent_teal: COLORREF(0x00C0A388),
                accent_purple: COLORREF(0x00BA88B4),
                accent_cyan: COLORREF(0x00D0BC8F),
                accent_amber: COLORREF(0x006CBBEB),
                accent_green: COLORREF(0x007AA8A3),
                accent_red: COLORREF(0x006561BF),
                progress_track: COLORREF(0x004C3A31),
                scroll_thumb: COLORREF(0x00614D43),
            },
            AppTheme::Cyberpunk => Self {
                bg_window: COLORREF(0x00140A05),
                bg_card: COLORREF(0x001F0F0B),
                card_border: COLORREF(0x00451B12),
                text_primary: COLORREF(0x0000FFFF),
                text_secondary: COLORREF(0x00FF00EA),
                text_muted: COLORREF(0x00806050),
                accent_teal: COLORREF(0x0000F0FF),
                accent_purple: COLORREF(0x00D000FF),
                accent_cyan: COLORREF(0x00FFB800),
                accent_amber: COLORREF(0x0000D0FF),
                accent_green: COLORREF(0x0000FF70),
                accent_red: COLORREF(0x003030FF),
                progress_track: COLORREF(0x002B120B),
                scroll_thumb: COLORREF(0x00801840),
            },
            AppTheme::DarkSlate | AppTheme::Auto => {
                if !is_dark {
                    Self::resolve(AppTheme::LightMode, false)
                } else {
                    Self {
                        bg_window: COLORREF(0x001C1410),
                        bg_card: COLORREF(0x00281E18),
                        card_border: COLORREF(0x003D2E26),
                        text_primary: COLORREF(0x00F5F2F0),
                        text_secondary: COLORREF(0x00C4B8B0),
                        text_muted: COLORREF(0x008A7C74),
                        accent_teal: COLORREF(0x00DEC038),
                        accent_purple: COLORREF(0x00C770B0),
                        accent_cyan: COLORREF(0x00E0BC40),
                        accent_amber: COLORREF(0x0038BDF8),
                        accent_green: COLORREF(0x004ADE80),
                        accent_red: COLORREF(0x004B55F4),
                        progress_track: COLORREF(0x0033261F),
                        scroll_thumb: COLORREF(0x00554238),
                    }
                }
            }
        }
    }
}

