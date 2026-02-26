use std::ops::Deref;
use std::str::FromStr;

use iced::{
    color,
    widget::{button, container, rule, scrollable, text, text_input},
};
use serde::{self, Deserialize, Serialize};

const DEFAULT_BACKGROUND_COLOR: iced::Color = color!(0x1F1F1F, 0.95);
const DEFAULT_FOCUS_HIGHLIGHT_COLOR: iced::Color = color!(0xFFFFFF, 0.12);
const DEFAULT_HOVER_HIGHLIGHT_COLOR: iced::Color = color!(0xFFFFFF, 0.08);
const DEFAULT_BORDER_COLOR: iced::Color = color!(0xA6A6A6, 0.1);
const DEFAULT_MAIN_TEXT: iced::Color = color!(0xF2F2F2);
const DEFAULT_SECONDARY_TEXT: iced::Color = color!(0xFFFFFF, 0.5);
const DEFAULT_DIM_TEXT: iced::Color = color!(0xFFFFFF, 0.5);

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct CustomTheme {
    pub background: HexColor,
    pub border: Border,
    pub prompt: Prompt,
    pub launchpad: Launchpad,
    pub separator: Separator,
}

impl Default for CustomTheme {
    fn default() -> Self {
        Self {
            background: DEFAULT_BACKGROUND_COLOR.into(),
            border: Border::default(),
            prompt: Prompt::default(),
            separator: Separator::default(),
            launchpad: Launchpad::default(),
        }
    }
}

impl iced::theme::Base for CustomTheme {
    fn default(_: iced::theme::Mode) -> Self {
        CustomTheme {
            ..Default::default()
        }
    }

    fn mode(&self) -> iced::theme::Mode {
        iced::theme::Mode::None
    }

    fn base(&self) -> iced::theme::Style {
        iced::theme::Style {
            background_color: iced::Color::TRANSPARENT,
            text_color: Default::default(),
        }
    }

    fn palette(&self) -> Option<iced::theme::Palette> {
        None
    }

    fn name(&self) -> &str {
        "Lucien"
    }
}

#[derive(Debug, Serialize, Copy, Deserialize, Clone)]
#[serde(default)]
pub struct Border {
    pub color: HexColor,
    pub width: f32,
    pub radius: [f32; 4],
}

impl Default for Border {
    fn default() -> Self {
        Self {
            color: DEFAULT_BORDER_COLOR.into(),
            width: 1.0,
            radius: [30.0, 30.0, 30.0, 30.0],
        }
    }
}

impl From<&Border> for iced::Border {
    fn from(value: &Border) -> iced::Border {
        iced::Border {
            color: value.color.into(),
            width: value.width,
            radius: iced::border::Radius {
                top_left: value.radius[0],
                top_right: value.radius[1],
                bottom_right: value.radius[2],
                bottom_left: value.radius[3],
            },
        }
    }
}

impl From<Border> for iced::Border {
    fn from(value: Border) -> iced::Border {
        iced::Border {
            color: value.color.into(),
            width: value.width,
            radius: iced::border::Radius {
                top_left: value.radius[0],
                top_right: value.radius[1],
                bottom_right: value.radius[2],
                bottom_left: value.radius[3],
            },
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct HexColor(pub iced::Color);

impl From<iced::Color> for HexColor {
    fn from(value: iced::Color) -> Self {
        HexColor(iced::Color {
            r: value.r,
            g: value.g,
            b: value.b,
            a: value.a,
        })
    }
}

impl From<HexColor> for iced::Color {
    fn from(value: HexColor) -> Self {
        iced::Color {
            r: value.0.r,
            g: value.0.g,
            b: value.0.b,
            a: value.0.a,
        }
    }
}

impl Deref for HexColor {
    type Target = iced::Color;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Serialize for HexColor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let hex_color = {
            let color = self.0;
            let r = (color.r * 255.0) as u8;
            let g = (color.g * 255.0) as u8;
            let b = (color.b * 255.0) as u8;
            let a = (color.a * 255.0) as u8;
            format!("#{:02X}{:02X}{:02X}{:02X}", r, g, b, a)
        };
        serializer.serialize_str(&hex_color)
    }
}

impl<'de> Deserialize<'de> for HexColor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let color = String::deserialize(deserializer)?;
        let converted_color = iced::Color::from_str(&color).map_err(serde::de::Error::custom)?;

        Ok(HexColor(converted_color))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Prompt {
    pub font_size: u16,
    pub background: HexColor,
    pub icon_size: u16,
    pub padding: Padding,
    pub margin: Padding,
    pub border: Border,
    pub placeholder_color: HexColor,
    pub text_color: HexColor,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Padding(f32, f32, f32, f32);

impl From<[f32; 4]> for Padding {
    fn from(value: [f32; 4]) -> Self {
        Padding(value[0], value[1], value[2], value[3])
    }
}

impl From<Padding> for iced::Padding {
    fn from(value: Padding) -> Self {
        iced::Padding {
            top: value.0,
            right: value.1,
            bottom: value.2,
            left: value.3,
        }
    }
}

impl From<&Padding> for iced::Padding {
    fn from(value: &Padding) -> Self {
        iced::Padding {
            top: value.0,
            right: value.1,
            bottom: value.2,
            left: value.3,
        }
    }
}

impl Default for Prompt {
    fn default() -> Self {
        Self {
            background: iced::Color::TRANSPARENT.into(),
            font_size: 18,
            icon_size: 32,
            padding: Padding::from([8., 8., 8., 8.]),
            border: Border {
                color: iced::Color::TRANSPARENT.into(),
                ..Default::default()
            },
            placeholder_color: DEFAULT_DIM_TEXT.into(),
            text_color: DEFAULT_MAIN_TEXT.into(),
            margin: Padding::from([15., 15., 15., 15.]),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Separator {
    pub color: HexColor,
    pub width: u16,
    pub padding: u16,
    pub radius: f32,
}

impl Default for Separator {
    fn default() -> Self {
        Self {
            color: DEFAULT_BORDER_COLOR.into(),
            width: 1,
            padding: 10,
            radius: 0.0,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Entry {
    pub background: HexColor,
    pub focus_highlight: HexColor,
    pub hover_highlight: HexColor,
    pub font_size: u32,
    pub secondary_font_size: u32,
    pub main_text: HexColor,
    pub secondary_text: HexColor,
    pub padding: Padding,
    pub height: f32,
    pub border: Border,
    pub icon_size: u32,
}

impl Default for Entry {
    fn default() -> Self {
        Self {
            icon_size: 32,
            height: 58.0,
            background: DEFAULT_BACKGROUND_COLOR.into(),
            focus_highlight: DEFAULT_FOCUS_HIGHLIGHT_COLOR.into(),
            hover_highlight: DEFAULT_HOVER_HIGHLIGHT_COLOR.into(),
            font_size: 14,
            secondary_font_size: 12,
            main_text: DEFAULT_MAIN_TEXT.into(),
            secondary_text: DEFAULT_SECONDARY_TEXT.into(),
            padding: Padding::from([10., 10., 10., 10.]),
            border: Border {
                color: iced::Color::TRANSPARENT.into(),
                width: 0.0,
                radius: [20., 20., 20., 20.],
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Launchpad {
    pub padding: f32,
    pub entry: Entry,
}

impl Default for Launchpad {
    fn default() -> Self {
        Self {
            padding: 10.,
            entry: Entry::default(),
        }
    }
}

pub enum ButtonClass {
    Itemlist,
    ItemlistSelected,
    Transparent,
}

impl button::Catalog for CustomTheme {
    type Class<'a> = ButtonClass;

    fn default<'a>() -> Self::Class<'a> {
        ButtonClass::Itemlist
    }

    fn style(&self, class: &Self::Class<'_>, status: button::Status) -> button::Style {
        let entry_style = &self.launchpad.entry;

        match (class, status) {
            (ButtonClass::Itemlist, button::Status::Hovered) => button::Style {
                background: Some(iced::Background::Color(entry_style.hover_highlight.into())),
                text_color: entry_style.main_text.into(),
                border: entry_style.border.into(),
                ..Default::default()
            },
            (ButtonClass::Itemlist, _) => button::Style {
                background: Some(iced::Background::Color(iced::Color::TRANSPARENT)),
                text_color: entry_style.main_text.into(),
                border: entry_style.border.into(),
                ..Default::default()
            },
            (ButtonClass::ItemlistSelected, _) => button::Style {
                background: Some(iced::Background::Color(entry_style.focus_highlight.into())),
                text_color: entry_style.main_text.into(),
                border: entry_style.border.into(),
                ..Default::default()
            },
            (ButtonClass::Transparent, _) => button::Style {
                background: Some(iced::Background::Color(iced::Color::TRANSPARENT)),
                ..Default::default()
            },
        }
    }
}

pub enum ContainerClass {
    Default,
    MainContainer,
}

impl container::Catalog for CustomTheme {
    type Class<'a> = ContainerClass;

    fn default<'a>() -> Self::Class<'a> {
        ContainerClass::Default
    }

    fn style(&self, class: &Self::Class<'_>) -> container::Style {
        match class {
            ContainerClass::Default => container::Style::default(),
            ContainerClass::MainContainer => container::Style {
                background: Some(iced::Background::Color(self.background.into())),
                border: self.border.into(),
                ..Default::default()
            },
        }
    }
}

pub enum TextClass {
    Default,
    TextDim,
    SecondaryText,
}

impl text::Catalog for CustomTheme {
    type Class<'a> = TextClass;

    fn default<'a>() -> Self::Class<'a> {
        TextClass::Default
    }

    fn style(&self, item: &Self::Class<'_>) -> text::Style {
        match item {
            TextClass::Default => text::Style::default(),
            TextClass::TextDim => text::Style {
                color: Some(self.prompt.placeholder_color.into()),
            },
            TextClass::SecondaryText => text::Style {
                color: Some(self.launchpad.entry.secondary_text.into()),
            },
        }
    }
}

pub enum ScrollableClass {
    Default,
}

impl scrollable::Catalog for CustomTheme {
    type Class<'a> = ScrollableClass;

    fn default<'a>() -> Self::Class<'a> {
        ScrollableClass::Default
    }

    fn style(&self, _class: &Self::Class<'_>, _status: scrollable::Status) -> scrollable::Style {
        scrollable::Style {
            container: iced::widget::container::Style {
                background: Some(iced::Background::Color(iced::Color::TRANSPARENT)),
                border: iced::Border {
                    radius: iced::border::radius(30),
                    ..Default::default()
                },
                ..Default::default()
            },
            vertical_rail: iced::widget::scrollable::Rail {
                background: Some(iced::Background::Color(iced::Color::TRANSPARENT)),
                scroller: scrollable::Scroller {
                    background: iced::Background::Color(iced::Color::TRANSPARENT),
                    border: iced::Border {
                        width: 0.0,
                        ..Default::default()
                    },
                },
                border: iced::Border::default(),
            },
            horizontal_rail: iced::widget::scrollable::Rail {
                background: None,
                scroller: scrollable::Scroller {
                    background: iced::Background::Color(self.background.into()),
                    border: iced::Border {
                        radius: iced::border::radius(5),
                        ..Default::default()
                    },
                },
                border: iced::Border::default(),
            },
            gap: None,
            auto_scroll: scrollable::AutoScroll {
                background: iced::Background::Color(iced::Color::TRANSPARENT),
                border: iced::Border::default(),
                shadow: iced::Shadow::default(),
                icon: iced::Color::TRANSPARENT,
            },
        }
    }
}

pub enum RuleClass {
    Default,
}

impl rule::Catalog for CustomTheme {
    type Class<'a> = RuleClass;

    fn default<'a>() -> Self::Class<'a> {
        RuleClass::Default
    }

    fn style(&self, _class: &Self::Class<'_>) -> rule::Style {
        rule::Style {
            color: self.separator.color.into(),
            fill_mode: iced::widget::rule::FillMode::Padded(self.separator.padding),
            radius: self.separator.radius.into(),
            snap: false,
        }
    }
}

pub enum TextInputClass {
    Default,
}

impl text_input::Catalog for CustomTheme {
    type Class<'a> = TextInputClass;

    fn default<'a>() -> Self::Class<'a> {
        TextInputClass::Default
    }

    fn style(&self, _class: &Self::Class<'_>, _status: text_input::Status) -> text_input::Style {
        text_input::Style {
            background: iced::Background::Color(self.prompt.background.into()),
            border: self.prompt.border.into(),
            icon: self.prompt.placeholder_color.into(),
            placeholder: self.prompt.placeholder_color.into(),
            value: self.prompt.text_color.into(),
            selection: self.launchpad.entry.focus_highlight.into(),
        }
    }
}
