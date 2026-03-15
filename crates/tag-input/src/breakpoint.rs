use dioxus::prelude::*;

/// Responsive breakpoint categories based on viewport width.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Breakpoint {
    /// Width < 768px
    Mobile,
    /// 768px <= Width < 1400px
    Tablet,
    /// Width >= 1400px
    Desktop,
}

impl Breakpoint {
    /// Determine breakpoint from a pixel width.
    pub fn from_width(width: u32) -> Self {
        if width < 768 {
            Breakpoint::Mobile
        } else if width < 1400 {
            Breakpoint::Tablet
        } else {
            Breakpoint::Desktop
        }
    }
}

/// Hook that returns the current responsive breakpoint.
///
/// On WASM targets, reads `window.innerWidth` via web-sys and listens for
/// resize events. On non-WASM targets, defaults to `Breakpoint::Desktop`.
pub fn use_breakpoint() -> Signal<Breakpoint> {
    #[cfg(target_arch = "wasm32")]
    {
        use_breakpoint_wasm()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use_breakpoint_default()
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn use_breakpoint_default() -> Signal<Breakpoint> {
    use_signal(|| Breakpoint::Desktop)
}

#[cfg(target_arch = "wasm32")]
fn use_breakpoint_wasm() -> Signal<Breakpoint> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen::closure::Closure;

    let mut breakpoint = use_signal(|| {
        let width = get_window_width();
        Breakpoint::from_width(width)
    });

    use_hook(|| {
        let closure = Closure::<dyn FnMut()>::new(move || {
            let width = get_window_width();
            let new_bp = Breakpoint::from_width(width);
            breakpoint.set(new_bp);
        });

        let window = web_sys::window().expect("no global window");
        window
            .add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref())
            .expect("failed to add resize listener");

        // Leak the closure so it lives for the lifetime of the page.
        // In a real app you'd store it and remove on drop, but for a
        // singleton hook this is acceptable.
        closure.forget();
    });

    breakpoint
}

#[cfg(target_arch = "wasm32")]
fn get_window_width() -> u32 {
    web_sys::window()
        .and_then(|w| w.inner_width().ok())
        .and_then(|v| v.as_f64())
        .map(|f| f as u32)
        .unwrap_or(1024)
}
