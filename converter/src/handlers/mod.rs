pub mod accents;
pub mod bold;
pub mod handler;
pub mod italic;
pub mod strikethrough;
pub mod surline;
pub mod underline;

pub use bold::bold_handler;
pub use handler::FontHandler;
pub use italic::italic_handler;
pub use strikethrough::{strikethrough_handler, StrikethroughHandler};
pub use surline::{surline_handler, SurlineHandler};
pub use underline::{underline_handler, UnderlineHandler};
