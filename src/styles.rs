use cursive::theme::{BaseColor, Color, ColorStyle};

lazy_static! {
    pub static ref GREEN: ColorStyle =
        ColorStyle::new(Color::Dark(BaseColor::Green), Color::Dark(BaseColor::Black),);
    pub static ref LIGHT_GREEN: ColorStyle = ColorStyle::new(
        Color::Light(BaseColor::Green),
        Color::Dark(BaseColor::Black),
    );
    pub static ref BLUE: ColorStyle =
        ColorStyle::new(Color::Dark(BaseColor::Blue), Color::Dark(BaseColor::Black),);
    pub static ref LIGHT_BLUE: ColorStyle =
        ColorStyle::new(Color::Light(BaseColor::Blue), Color::Dark(BaseColor::Black),);
    pub static ref RED: ColorStyle =
        ColorStyle::new(Color::Dark(BaseColor::Red), Color::Dark(BaseColor::Black),);
    pub static ref WHITE: ColorStyle =
        ColorStyle::new(Color::Dark(BaseColor::White), Color::Dark(BaseColor::Black),);
    pub static ref YELLOW: ColorStyle = ColorStyle::new(
        Color::Dark(BaseColor::Yellow),
        Color::Dark(BaseColor::Black),
    );
    pub static ref MAGENTA: ColorStyle = ColorStyle::new(
        Color::Dark(BaseColor::Magenta),
        Color::Dark(BaseColor::Black),
    );
}
