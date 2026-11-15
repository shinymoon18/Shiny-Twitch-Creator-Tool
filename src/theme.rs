use eframe::egui::Color32;
use serde::{Serialize, Deserialize};

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum Theme { #[default] Night, Oled, Ocean, Rose, Forest }
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Appearance { pub theme: Theme, pub accent: Option<[u8; 3]>, pub scale: f32 }
impl Default for Appearance { fn default() -> Self { Self { theme: Theme::Night, accent: None, scale: 1.0 } } }
#[derive(Clone, Copy)]
pub struct Palette { pub bg: Color32, pub card: Color32, pub border: Color32, pub text: Color32, pub muted: Color32, pub accent: Color32, pub mint: Color32 }
impl Appearance {
    pub fn palette(&self) -> Palette {
        let (bg, card, border, accent, mint) = match self.theme {
            Theme::Night => ([11,14,25], [21,26,43], [43,50,73], [163,132,255], [95,222,196]),
            Theme::Oled => ([0,0,0], [13,13,16], [42,42,48], [190,167,255], [120,235,209]),
            Theme::Ocean => ([7,20,31], [14,35,49], [34,64,80], [91,190,250], [98,228,211]),
            Theme::Rose => ([25,12,25], [43,22,40], [75,43,67], [245,149,198], [210,175,255]),
            Theme::Forest => ([9,22,19], [18,37,31], [40,65,53], [120,216,160], [232,203,133]),
        };
        Palette { bg: Color32::from_rgb(bg[0],bg[1],bg[2]), card: Color32::from_rgb(card[0],card[1],card[2]), border: Color32::from_rgb(border[0],border[1],border[2]), text: Color32::from_rgb(231,235,247), muted: Color32::from_rgb(153,166,191), accent: { let c = self.accent.unwrap_or(accent); Color32::from_rgb(c[0],c[1],c[2]) }, mint: Color32::from_rgb(mint[0],mint[1],mint[2]) }
    }
}
