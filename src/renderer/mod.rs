pub use self::html_handlebars::HtmlHandlebars;
pub use self::pandoc::PandocRenderer;

mod html_handlebars;
mod pandoc;

use errors::*;

pub trait Renderer {
    fn render(&self, book: &::book::MDBook) -> Result<()>;
}
