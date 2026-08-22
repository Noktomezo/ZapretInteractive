use gpui::{App, Bounds, ParentElement, Pixels, Window, canvas, prelude::*};

pub trait ElementPrepaintExt: ParentElement + Sized {
    fn on_prepaint(
        self,
        callback: impl FnOnce(Bounds<Pixels>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.child(
            canvas(
                move |bounds, window, cx| callback(bounds, window, cx),
                |_, _, _, _| {},
            )
            .absolute()
            .size_full(),
        )
    }
}

impl<T: ParentElement> ElementPrepaintExt for T {}
