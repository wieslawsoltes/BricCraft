//! Retain native inset notifications instead of querying browser styles each frame.
use briccraft_ui::UiLayout;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{Rect, Thickness};
use ferroui_controls::TopLevel;
use std::{cell::Cell, rc::Rc};

pub(super) struct LayoutMetrics {
    safe: Rc<Cell<Thickness>>,
    keyboard: Rc<Cell<Rect>>,
    subscriptions: Vec<Rc<dyn IDisposable>>,
}

impl LayoutMetrics {
    pub fn new(top: &TopLevel) -> Self {
        let safe = Rc::new(Cell::new(Thickness::default()));
        let keyboard = Rc::new(Cell::new(Rect::default()));
        let mut subscriptions = Vec::new();
        if let Some(insets) = top.insets_manager() {
            safe.set(insets.safe_area_padding());
            let current = safe.clone();
            subscriptions.push(insets.safe_area_changed(Rc::new(move |event| {
                current.set(event.safe_area_padding());
            })));
        }
        if let Some(pane) = top.input_pane() {
            keyboard.set(pane.occluded_rect());
            let current = keyboard.clone();
            subscriptions.push(pane.state_changed(Rc::new(move |event| {
                current.set(event.end_rect());
            })));
        }
        Self {
            safe,
            keyboard,
            subscriptions,
        }
    }

    pub fn measure(&self, size: [f64; 2], mut occlusion: [f64; 2], touch: bool) -> UiLayout {
        let safe = self.safe.get();
        let covered = self.keyboard.get();
        if covered.width > 0.0 && covered.height > 0.0 {
            occlusion[1] = occlusion[1].max((size[1] - covered.y).max(0.0));
        }
        UiLayout::new(
            size,
            [safe.left, safe.top, safe.right, safe.bottom],
            occlusion,
            touch,
        )
    }
}

impl Drop for LayoutMetrics {
    fn drop(&mut self) {
        for subscription in &self.subscriptions {
            subscription.dispose();
        }
    }
}
