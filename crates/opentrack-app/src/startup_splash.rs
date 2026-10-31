//! A standalone native splash shown before the application window is created.

#[cfg(windows)]
mod windows {
    use std::sync::OnceLock;
    use std::time::Instant;

    use windows_sys::Win32::Foundation::{GetLastError, HWND, LPARAM, LRESULT, RECT, WPARAM};
    use windows_sys::Win32::Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BeginPaint, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS,
        CreateFontW, CreateSolidBrush, DEFAULT_CHARSET, DEFAULT_PITCH, DIB_RGB_COLORS, DT_CENTER,
        DT_SINGLELINE, DT_VCENTER, DeleteObject, DrawTextW, EndPaint, FF_DONTCARE, FW_SEMIBOLD,
        FillRect, OUT_DEFAULT_PRECIS, PAINTSTRUCT, RGBQUAD, SRCCOPY, SelectObject, SetBkMode,
        SetTextColor, StretchDIBits, TRANSPARENT, UpdateWindow,
    };
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
        GetSystemMetrics, KillTimer, RegisterClassW, SM_CXSCREEN, SM_CYSCREEN, SetTimer,
        TranslateMessage, WM_DESTROY, WM_NCCREATE, WM_PAINT, WM_QUIT, WM_TIMER, WNDCLASSW,
        WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE,
    };

    const WIDTH: i32 = 600;
    const HEIGHT: i32 = 327;
    const TIMER_ID: usize = 0x4f54;
    const MIN_SPLASH_TIME: std::time::Duration = std::time::Duration::from_secs(2);
    static STARTED: OnceLock<Instant> = OnceLock::new();
    static PIXELS_BGRA: OnceLock<Vec<u8>> = OnceLock::new();

    fn splash_pixels() -> &'static [u8] {
        PIXELS_BGRA.get_or_init(|| {
            let svg = include_bytes!("../assets/opentrack-splash.svg");
            let tree = resvg::usvg::Tree::from_data(svg, &resvg::usvg::Options::default())
                .expect("bundled OpenTrack splash SVG should parse");
            let mut pixmap = resvg::tiny_skia::Pixmap::new(WIDTH as u32, HEIGHT as u32)
                .expect("splash bitmap dimensions are fixed");
            let scale =
                (WIDTH as f32 / tree.size().width()).min(HEIGHT as f32 / tree.size().height());
            let transform = resvg::tiny_skia::Transform::from_scale(scale, scale).post_translate(
                (WIDTH as f32 - tree.size().width() * scale) * 0.5,
                (HEIGHT as f32 - tree.size().height() * scale) * 0.5,
            );
            resvg::render(&tree, transform, &mut pixmap.as_mut());
            let mut bgra = pixmap.data().to_vec();
            for pixel in bgra.chunks_exact_mut(4) {
                let alpha = u32::from(pixel[3]);
                let backdrop = 24 * (255 - alpha) / 255;
                let red = u32::from(pixel[0]) + backdrop;
                let green = u32::from(pixel[1]) + backdrop;
                let blue = u32::from(pixel[2]) + backdrop;
                pixel[0] = blue.min(255) as u8;
                pixel[1] = green.min(255) as u8;
                pixel[2] = red.min(255) as u8;
                pixel[3] = 255;
            }
            bgra
        })
    }

    unsafe extern "system" fn window_proc(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match message {
            WM_NCCREATE => 1,
            WM_PAINT => {
                let mut paint = PAINTSTRUCT::default();
                let dc = unsafe { BeginPaint(hwnd, &mut paint) };
                let bounds = RECT {
                    left: 0,
                    top: 0,
                    right: WIDTH,
                    bottom: HEIGHT,
                };
                let brush = unsafe { CreateSolidBrush(0x0018_1818) };
                unsafe {
                    FillRect(dc, &bounds, brush);
                    DeleteObject(brush);
                }
                let bitmap_info = BITMAPINFO {
                    bmiHeader: BITMAPINFOHEADER {
                        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                        biWidth: WIDTH,
                        biHeight: -HEIGHT,
                        biPlanes: 1,
                        biBitCount: 32,
                        biCompression: BI_RGB,
                        biSizeImage: 0,
                        biXPelsPerMeter: 0,
                        biYPelsPerMeter: 0,
                        biClrUsed: 0,
                        biClrImportant: 0,
                    },
                    bmiColors: [RGBQUAD::default(); 1],
                };
                let pixels = splash_pixels();
                unsafe {
                    StretchDIBits(
                        dc,
                        0,
                        0,
                        WIDTH,
                        HEIGHT,
                        0,
                        0,
                        WIDTH,
                        HEIGHT,
                        pixels.as_ptr().cast(),
                        &bitmap_info,
                        DIB_RGB_COLORS,
                        SRCCOPY,
                    );
                    // usvg is built without text support. Use the supplied
                    // Inter-first font stack for the wordmark with GDI.
                    let face_name: Vec<u16> = "Inter\0".encode_utf16().collect();
                    let font = CreateFontW(
                        -58,
                        0,
                        0,
                        0,
                        FW_SEMIBOLD as i32,
                        0,
                        0,
                        0,
                        DEFAULT_CHARSET as u32,
                        OUT_DEFAULT_PRECIS as u32,
                        CLIP_DEFAULT_PRECIS as u32,
                        CLEARTYPE_QUALITY as u32,
                        (DEFAULT_PITCH | FF_DONTCARE) as u32,
                        face_name.as_ptr(),
                    );
                    if !font.is_null() {
                        let old_font = SelectObject(dc, font as _);
                        SetBkMode(dc, TRANSPARENT as i32);
                        SetTextColor(dc, 0x00FF_FFFF);
                        let wordmark: Vec<u16> = "OpenTrack".encode_utf16().collect();
                        let mut wordmark_bounds = RECT {
                            left: 12,
                            top: 187,
                            right: WIDTH - 12,
                            bottom: 282,
                        };
                        DrawTextW(
                            dc,
                            wordmark.as_ptr(),
                            wordmark.len() as i32,
                            &mut wordmark_bounds,
                            DT_CENTER | DT_VCENTER | DT_SINGLELINE,
                        );
                        SelectObject(dc, old_font);
                        DeleteObject(font as _);
                    }
                    EndPaint(hwnd, &mut paint);
                }
                0
            }
            WM_TIMER if wparam == TIMER_ID => {
                let elapsed = STARTED
                    .get()
                    .is_some_and(|started| started.elapsed() >= MIN_SPLASH_TIME);
                if elapsed {
                    unsafe {
                        KillTimer(hwnd, TIMER_ID);
                        DestroyWindow(hwnd);
                    }
                }
                0
            }
            WM_DESTROY => {
                unsafe {
                    windows_sys::Win32::UI::WindowsAndMessaging::PostQuitMessage(0);
                }
                0
            }
            _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
        }
    }

    pub fn show() -> bool {
        static REGISTERED: OnceLock<bool> = OnceLock::new();
        let class_name: Vec<u16> = "OpenTrackStartupSplash\0".encode_utf16().collect();
        let registered = *REGISTERED.get_or_init(|| unsafe {
            let class = WNDCLASSW {
                lpfnWndProc: Some(window_proc),
                hInstance: GetModuleHandleW(std::ptr::null()),
                lpszClassName: class_name.as_ptr(),
                ..Default::default()
            };
            RegisterClassW(&class) != 0
        });
        if !registered {
            return false;
        }
        let x = (unsafe { GetSystemMetrics(SM_CXSCREEN) } - WIDTH).max(0) / 2;
        let y = (unsafe { GetSystemMetrics(SM_CYSCREEN) } - HEIGHT).max(0) / 2;
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
                class_name.as_ptr(),
                [0u16].as_ptr(),
                WS_POPUP | WS_VISIBLE,
                x,
                y,
                WIDTH,
                HEIGHT,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                GetModuleHandleW(std::ptr::null()),
                std::ptr::null_mut(),
            )
        };
        if hwnd.is_null() {
            let _ = unsafe { GetLastError() };
            return false;
        }
        let _ = STARTED.set(Instant::now());
        unsafe {
            SetTimer(hwnd, TIMER_ID, 25, None);
            UpdateWindow(hwnd);
        }

        // Do not create or initialize eframe's main window until this native
        // splash has completed its minimum display interval and been destroyed.
        let mut message = windows_sys::Win32::UI::WindowsAndMessaging::MSG::default();
        loop {
            let result = unsafe { GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) };
            if result <= 0 || message.message == WM_QUIT {
                break;
            }
            unsafe {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        true
    }
}

#[cfg(windows)]
pub use windows::show;

#[cfg(not(windows))]
pub fn show() -> bool {
    const HELPER_FLAG: &str = "--opentrack-startup-splash";
    let Ok(executable) = std::env::current_exe() else {
        return false;
    };
    std::process::Command::new(executable)
        .arg(HELPER_FLAG)
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(not(windows))]
pub(crate) fn run_if_helper() -> Option<eframe::Result> {
    const HELPER_FLAG: &str = "--opentrack-startup-splash";
    std::env::args()
        .any(|argument| argument == HELPER_FLAG)
        .then(run_helper)
}

#[cfg(windows)]
pub(crate) fn run_if_helper() -> Option<eframe::Result> {
    None
}

#[cfg(not(windows))]
fn run_helper() -> eframe::Result {
    use eframe::egui::{self, Color32};

    struct SplashApp {
        first_frame: Option<std::time::Instant>,
    }

    impl eframe::App for SplashApp {
        fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
            let ctx = ui.ctx();
            let started = *self.first_frame.get_or_insert_with(std::time::Instant::now);
            if let Some(center) = egui::ViewportCommand::center_on_screen(ctx) {
                ctx.send_viewport_cmd(center);
            }
            let rect = ui.max_rect();
            ui.painter()
                .rect_filled(rect, 0.0, Color32::from_rgb(24, 24, 24));
            crate::icon_system::paint_splash(ui, rect);
            if splash_minimum_elapsed(started.elapsed()) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else {
                ctx.request_repaint_after(std::time::Duration::from_millis(25));
            }
        }
    }

    let options = eframe::NativeOptions {
        viewport: splash_viewport_builder(),
        ..Default::default()
    };
    eframe::run_native(
        "OpenTrack Startup Splash",
        options,
        Box::new(|creation| {
            crate::theme::install(&creation.egui_ctx);
            Ok(Box::new(SplashApp { first_frame: None }))
        }),
    )
}

#[cfg(not(windows))]
fn splash_viewport_builder() -> eframe::egui::ViewportBuilder {
    const WIDTH: f32 = 600.0;
    const HEIGHT: f32 = 327.0;
    eframe::egui::ViewportBuilder::default()
        .with_title("OpenTrack")
        .with_app_id("opentrack-startup-splash")
        .with_inner_size([WIDTH, HEIGHT])
        .with_min_inner_size([WIDTH, HEIGHT])
        .with_max_inner_size([WIDTH, HEIGHT])
        .with_maximized(false)
        .with_fullscreen(false)
        .with_decorations(false)
        .with_resizable(false)
        .with_window_level(eframe::egui::viewport::WindowLevel::AlwaysOnTop)
        .with_icon(crate::icons::opentrack_icon())
}

#[cfg(not(windows))]
fn splash_minimum_elapsed(elapsed: std::time::Duration) -> bool {
    elapsed >= std::time::Duration::from_secs(2)
}

#[cfg(all(test, not(windows)))]
mod linux_tests {
    use super::{splash_minimum_elapsed, splash_viewport_builder};
    use std::time::Duration;

    #[test]
    fn linux_splash_waits_two_seconds_before_releasing_the_main_window() {
        assert!(!splash_minimum_elapsed(Duration::from_millis(1999)));
        assert!(splash_minimum_elapsed(Duration::from_secs(2)));
    }

    #[test]
    fn linux_splash_forces_its_compact_non_fullscreen_size() {
        let viewport = splash_viewport_builder();
        assert_eq!(viewport.inner_size, Some(eframe::egui::vec2(600.0, 327.0)));
        assert_eq!(viewport.maximized, Some(false));
        assert_eq!(viewport.fullscreen, Some(false));
    }
}
