// Modules
pub mod element;
mod primitive;
mod render;
pub mod rescache;
pub mod span;
#[cfg(test)]
mod tests;

// Reexports
pub use self::span::Span;

// Standard
use std::rc::Rc;

// Library
use vek::*;

// Local
use self::{element::Element, rescache::ResCache};
use crate::{renderer::Renderer, window::Event};

#[allow(dead_code)]
pub struct Ui {
    base: Rc<dyn Element>,
    rescache: ResCache,
}

impl Ui {
    #[allow(dead_code)]
    pub fn new(base: Rc<dyn Element>) -> Ui {
        Ui {
            base,
            rescache: ResCache::new(),
        }
    }

    #[allow(dead_code)]
    pub fn render(&mut self, renderer: &mut Renderer) {
        self.base
            .render(renderer, &mut self.rescache, (Vec2::zero(), Vec2::one()));
    }

    #[allow(dead_code)]
    pub fn handle_event(&self, event: &Event, renderer: &mut Renderer) -> bool {
        self.base.handle_event(
            event,
            renderer.get_view_resolution().map(|e| e as f32),
            (Vec2::zero(), Vec2::one()),
        )
    }
}
