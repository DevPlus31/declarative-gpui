#[allow(unused_imports)]
use std::fmt;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Color {
    Hex(u32),
    Name(&'static str),
    Rgb(u8, u8, u8),
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Style {
    Flex,
    FlexCol,
    FlexRow,
    JustifyCenter,
    JustifyBetween,
    ItemsCenter,
    Gap(f32),
    Padding(f32),
    Width(f32),
    Height(f32),
    Size(f32),
    SizeFull,
    Background(Color),
    TextColor(Color),
    TextSize(f32),
    FontWeightBold,
}

#[allow(dead_code)]
pub struct Element {
    pub tag: String,
    pub styles: Vec<Style>,
    pub children: Vec<Element>,
    pub content: Option<String>,
    pub id: Option<String>,
    pub on_click: Option<Box<dyn Fn(&mut dyn std::any::Any)>>,
}

impl Element {
    pub fn new(tag: &str) -> Self {
        Self {
            tag: tag.to_string(),
            styles: Vec::new(),
            children: Vec::new(),
            content: None,
            id: None,
            on_click: None,
        }
    }

    pub fn style(mut self, style: Style) -> Self {
        self.styles.push(style);
        self
    }

    pub fn styles(mut self, styles: Vec<Style>) -> Self {
        self.styles.extend(styles);
        self
    }

    pub fn child(mut self, child: Element) -> Self {
        self.children.push(child);
        self
    }

    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn on_click<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut dyn std::any::Any) + 'static,
    {
        self.on_click = Some(Box::new(f));
        self
    }
}

// Style Parser
pub fn parse_styles(input: &str) -> Vec<Style> {
    input.split_whitespace()
        .filter_map(|s| {
            match s {
                "flex" => Some(Style::Flex),
                "flex-col" | "col" => Some(Style::FlexCol),
                "flex-row" | "row" => Some(Style::FlexRow),
                "justify-center" => Some(Style::JustifyCenter),
                "justify-between" => Some(Style::JustifyBetween),
                "items-center" => Some(Style::ItemsCenter),
                "font-bold" | "bold" => Some(Style::FontWeightBold),
                "bg-gray" => Some(Style::Background(Color::Hex(0x333333))),
                "bg-dark" => Some(Style::Background(Color::Hex(0x1e1e1e))),
                "bg-light-gray" => Some(Style::Background(Color::Hex(0x333333))),
                "bg-red" => Some(Style::Background(Color::Name("red"))),
                "bg-green" => Some(Style::Background(Color::Name("green"))),
                "bg-blue" => Some(Style::Background(Color::Hex(0x4a90e2))),
                "bg-footer" => Some(Style::Background(Color::Hex(0x252525))),
                "text-white" => Some(Style::TextColor(Color::Hex(0xffffff))),
                "text-gray" => Some(Style::TextColor(Color::Hex(0xcccccc))),
                "text-dim" => Some(Style::TextColor(Color::Hex(0x666666))),
                "size-full" => Some(Style::SizeFull),
                s if s.starts_with("gap-") => {
                    s["gap-".len()..].parse::<f32>().ok().map(Style::Gap)
                }
                s if s.starts_with("p-") => {
                    s["p-".len()..].parse::<f32>().ok().map(Style::Padding)
                }
                s if s.starts_with("px-") => {
                    // Simplified: px-4 sets padding to 4 (in this basic parser)
                    s["px-".len()..].parse::<f32>().ok().map(Style::Padding)
                }
                s if s.starts_with("py-") => {
                    s["py-".len()..].parse::<f32>().ok().map(Style::Padding)
                }
                s if s.starts_with("w-") => {
                    s["w-".len()..].parse::<f32>().ok().map(Style::Width)
                }
                s if s.starts_with("h-") => {
                    s["h-".len()..].parse::<f32>().ok().map(Style::Height)
                }
                s if s.starts_with("size-") => {
                    s["size-".len()..].parse::<f32>().ok().map(Style::Size)
                }
                s if s.starts_with("text-") && s.len() > 5 && s.chars().nth(5).unwrap().is_digit(10) => {
                    s["text-".len()..].parse::<f32>().ok().map(Style::TextSize)
                }
                "text-xl" => Some(Style::TextSize(20.0)),
                "text-lg" => Some(Style::TextSize(18.0)),
                "text-sm" => Some(Style::TextSize(14.0)),
                "text-xs" => Some(Style::TextSize(12.0)),
                "text-2xl" => Some(Style::TextSize(24.0)),
                _ => None,
            }
        })
        .collect()
}

// Helpers
pub fn div() -> Element { Element::new("div") }
pub fn row() -> Element { Element::new("div").style(Style::FlexRow) }
#[allow(dead_code)]
pub fn col() -> Element { Element::new("div").style(Style::FlexCol) }
pub fn text(content: impl Into<String>) -> Element { Element::new("text").content(content) }
pub fn box_elem(size: f32, color: Color) -> Element { 
    Element::new("div").style(Style::Size(size)).style(Style::Background(color)) 
}

// Macros
#[macro_export]
macro_rules! ui {
    // text[content]
    ( text [ $content:expr ] ) => {
        $crate::declarative_ui::text($content)
    };

    // box[size, color]
    ( box [ $size:expr , $color:expr ] ) => {
        $crate::declarative_ui::box_elem($size, $color)
    };

    // div["styles", id, on_click] { children } - Optional id and on_click
    ( div [ $styles:expr ] { $($children:tt)* } ) => {
        {
            let mut el = $crate::declarative_ui::div().styles($crate::declarative_ui::parse_styles($styles));
            $crate::ui_collect_children!(el, $($children)*);
            el
        }
    };

    ( div [ $styles:expr, $id:expr ] { $($children:tt)* } ) => {
        {
            let mut el = $crate::declarative_ui::div().styles($crate::declarative_ui::parse_styles($styles)).id($id);
            $crate::ui_collect_children!(el, $($children)*);
            el
        }
    };

    ( div [ $styles:expr, $id:expr, $on_click:expr ] { $($children:tt)* } ) => {
        {
            let mut el = $crate::declarative_ui::div().styles($crate::declarative_ui::parse_styles($styles)).id($id).on_click($on_click);
            $crate::ui_collect_children!(el, $($children)*);
            el
        }
    };

    // row[gap] { children }
    ( row [ $gap:expr ] { $($children:tt)* } ) => {
        {
            let mut el = $crate::declarative_ui::row().style($crate::declarative_ui::Style::Gap($gap));
            $crate::ui_collect_children!(el, $($children)*);
            el
        }
    };

    // col[gap] { children }
    ( col [ $gap:expr ] { $($children:tt)* } ) => {
        {
            let mut el = $crate::declarative_ui::col().style($crate::declarative_ui::Style::Gap($gap));
            $crate::ui_collect_children!(el, $($children)*);
            el
        }
    };
}

#[macro_export]
macro_rules! ui_collect_children {
    ($el:ident, ) => {};
    ($el:ident, $tag:ident [ $($args:tt)* ] { $($body:tt)* } $($rest:tt)* ) => {
        $el = $el.child($crate::ui! { $tag [ $($args)* ] { $($body)* } });
        $crate::ui_collect_children!($el, $($rest)*);
    };
    ($el:ident, $tag:ident [ $($args:tt)* ] $($rest:tt)* ) => {
        $el = $el.child($crate::ui! { $tag [ $($args)* ] });
        $crate::ui_collect_children!($el, $($rest)*);
    };
}

#[macro_export]
macro_rules! jsx {
    // Top-level entry point for tags WITH bodies
    ( <$tag:ident $($attr:ident={ $($val:tt)* })* > { $($body:tt)* } </$tag_end:ident> ) => {
        {
            let mut el = $crate::jsx_tag!( <$tag $($attr={ $($val)* })* /> );
            $crate::jsx_collect_children!(el, $($body)*);
            el
        }
    };
    // Special case for text
    ( <text>{ $($content:tt)* }</text> ) => {
        $crate::declarative_ui::text($($content)*)
    };
    // Tags WITHOUT bodies
    ( <$tag:ident $($attr:ident={ $($val:tt)* })* /> ) => {
        $crate::jsx_tag!( <$tag $($attr={ $($val)* })* /> )
    };
}

#[macro_export]
macro_rules! jsx_tag {
    ( <div class={ $($styles:tt)* } /> ) => { $crate::declarative_ui::div().styles($crate::declarative_ui::parse_styles($($styles)*)) };
    ( <div /> ) => { $crate::declarative_ui::div() };
    ( <row gap={ $($gap:tt)* } /> ) => { $crate::declarative_ui::row().style($crate::declarative_ui::Style::Gap($($gap)*)) };
    ( <row /> ) => { $crate::declarative_ui::row() };
    ( <col gap={ $($gap:tt)* } /> ) => { $crate::declarative_ui::col().style($crate::declarative_ui::Style::Gap($($gap)*)) };
    ( <col /> ) => { $crate::declarative_ui::col() };
    ( <box size={ $($size:tt)* } color={ $($color:tt)* } /> ) => { $crate::declarative_ui::box_elem($($size)*, $($color)*) };
    ( <text /> ) => { $crate::declarative_ui::text("") };
}

#[macro_export]
macro_rules! jsx_collect_children {
    ($el:ident, ) => {};
    // text tag
    ($el:ident, <text>{ $($content:tt)* }</text> $($rest:tt)* ) => {
        {
            let child = $crate::declarative_ui::text($($content)*);
            $el = $el.child(child);
        }
        $crate::jsx_collect_children!($el, $($rest)*);
    };
    // self closing tags
    ($el:ident, <$tag:ident $($attr:ident={ $($val:tt)* })* /> $($rest:tt)* ) => {
        {
            let child = $crate::jsx! { <$tag $($attr={ $($val)* })* /> };
            $el = $el.child(child);
        }
        $crate::jsx_collect_children!($el, $($rest)*);
    };
    // recursive tags with bodies
    ($el:ident, <$tag:ident $($attr:ident={ $($val:tt)* })* > { $($body:tt)* } </$tag_end:ident> $($rest:tt)* ) => {
        {
            let child = $crate::jsx! { <$tag $($attr={ $($val)* })* > { $($body)* } </$tag_end> };
            $el = $el.child(child);
        }
        $crate::jsx_collect_children!($el, $($rest)*);
    };
}
