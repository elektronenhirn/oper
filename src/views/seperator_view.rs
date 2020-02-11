use cursive::direction::Orientation;
use cursive::theme::{BaseColor, Color, ColorStyle};
use cursive::traits::View;
use cursive::vec::Vec2;
use cursive::Printer;

/// View to display a line to visually seperate other views
pub struct SeperatorView {
    orientation: Orientation,
}

impl SeperatorView {
    pub fn vertical() -> SeperatorView {
        SeperatorView {
            orientation: Orientation::Vertical,
        }
    }
}
impl View for SeperatorView {
    fn draw(&self, printer: &Printer<'_, '_>) {
        let style = ColorStyle::new(Color::Dark(BaseColor::White), Color::Dark(BaseColor::Blue));
        printer.with_style(style, |p| match self.orientation {
            Orientation::Vertical => {
                p.print_vline(self.orientation.make_vec(0, 0), printer.size.y, "│")
            }
            Orientation::Horizontal => {
                p.print_hline(self.orientation.make_vec(0, 0), printer.size.x, "─")
            }
        });
    }

    fn required_size(&mut self, _: Vec2) -> Vec2 {
        Vec2::new(1, 1)
    }
}
