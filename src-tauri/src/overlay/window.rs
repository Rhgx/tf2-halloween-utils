//! The overlay's windows: never focused, always on top, click-through, showing a premultiplied pixmap with
//! per-pixel alpha. Also finds TF2's window and its folder.

use std::mem::{size_of, zeroed};
use std::path::PathBuf;
use std::ptr::{null, null_mut};

use tiny_skia::Pixmap;
use windows_sys::Win32::Foundation::{
    CloseHandle, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM,
};
use windows_sys::Win32::Graphics::Gdi::{
    ClientToScreen, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC,
    SelectObject, AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION,
    DIB_RGB_COLORS,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, FindWindowW, GetClientRect, GetWindowLongPtrW,
    GetWindowThreadProcessId, IsIconic, SetWindowLongPtrW, SetWindowPos, ShowWindow,
    UpdateLayeredWindow, GWL_EXSTYLE, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, SW_HIDE,
    SW_SHOWNOACTIVATE, ULW_ALPHA, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_EX_TRANSPARENT, WS_POPUP,
};

/// A window's left, top, width and height on screen.
pub type Rect = (i32, i32, i32, i32);

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

#[derive(Debug)]
pub struct Tf2 {
    pub hwnd: HWND,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

pub fn tf2_window() -> Option<Tf2> {
    unsafe {
        let hwnd = FindWindowW(wide("Valve001").as_ptr(), null());
        if hwnd.is_null() || IsIconic(hwnd) != 0 {
            return None;
        }
        let mut rect: RECT = zeroed();
        let mut origin = POINT { x: 0, y: 0 };
        GetClientRect(hwnd, &mut rect);
        ClientToScreen(hwnd, &mut origin);
        (rect.right >= 64 && rect.bottom >= 64).then_some(Tf2 {
            hwnd,
            x: origin.x,
            y: origin.y,
            w: rect.right,
            h: rect.bottom,
        })
    }
}

/// The `tf` folder next to the running TF2 executable.
pub fn tf_dir_of(hwnd: HWND) -> Option<PathBuf> {
    unsafe {
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process.is_null() {
            return None;
        }
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok =
            QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut len)
                != 0;
        CloseHandle(process);
        ok.then(|| PathBuf::from(String::from_utf16_lossy(&buf[..len as usize])))?
            .parent()
            .map(|d| d.join("tf"))
    }
}

pub unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

/// A never-focused, always-on-top window showing a premultiplied pixmap with per-pixel alpha.
/// Click-through, except the radar panel while editing. Destroyed on drop, on the thread that made it.
pub struct Layer {
    hwnd: HWND,
    pub visible: bool,
    /// Last position and size on screen.
    pub rect: Rect,
    /// Everything the picture on screen was drawn from, to skip redrawing it unchanged.
    drawn: String,
}

impl Layer {
    pub fn new(class: &[u16]) -> Layer {
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_LAYERED
                    | WS_EX_TRANSPARENT
                    | WS_EX_TOOLWINDOW
                    | WS_EX_NOACTIVATE
                    | WS_EX_TOPMOST,
                class.as_ptr(),
                class.as_ptr(),
                WS_POPUP,
                0,
                0,
                1,
                1,
                null_mut(),
                null_mut(),
                GetModuleHandleW(null()),
                null(),
            )
        };
        Layer {
            hwnd,
            visible: false,
            rect: (0, 0, 0, 0),
            drawn: String::new(),
        }
    }

    /// True when the picture described by `key` is not already on screen.
    pub fn stale(&mut self, key: String) -> bool {
        if self.visible && self.drawn == key {
            return false;
        }
        self.drawn = key;
        true
    }

    pub fn set_click_through(&self, on: bool) {
        unsafe {
            let style = GetWindowLongPtrW(self.hwnd, GWL_EXSTYLE);
            let flag = WS_EX_TRANSPARENT as isize;
            SetWindowLongPtrW(
                self.hwnd,
                GWL_EXSTYLE,
                if on { style | flag } else { style & !flag },
            );
        }
    }

    pub fn show(&mut self, x: i32, y: i32, pm: &Pixmap) {
        let (w, h) = (pm.width() as i32, pm.height() as i32);
        self.rect = (x, y, w, h);
        unsafe {
            let screen = GetDC(null_mut());
            let mem = CreateCompatibleDC(screen);
            let mut info: BITMAPINFO = zeroed();
            info.bmiHeader = BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                ..zeroed()
            };
            let mut bits = null_mut();
            let bmp = CreateDIBSection(mem, &info, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
            if !bmp.is_null() && !bits.is_null() {
                let dst = std::slice::from_raw_parts_mut(bits as *mut u8, (w * h * 4) as usize);
                for (d, s) in dst
                    .as_chunks_mut::<4>()
                    .0
                    .iter_mut()
                    .zip(pm.data().as_chunks::<4>().0)
                {
                    *d = [s[2], s[1], s[0], s[3]]; // premultiplied RGBA -> BGRA
                }
                let old = SelectObject(mem, bmp);
                let blend = BLENDFUNCTION {
                    BlendOp: AC_SRC_OVER as u8,
                    BlendFlags: 0,
                    SourceConstantAlpha: 255,
                    AlphaFormat: AC_SRC_ALPHA as u8,
                };
                let (pos, size, src) =
                    (POINT { x, y }, SIZE { cx: w, cy: h }, POINT { x: 0, y: 0 });
                UpdateLayeredWindow(
                    self.hwnd, screen, &pos, &size, mem, &src, 0, &blend, ULW_ALPHA,
                );
                SelectObject(mem, old);
                DeleteObject(bmp);
            }
            DeleteDC(mem);
            ReleaseDC(null_mut(), screen);
            if !self.visible {
                ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
                self.visible = true;
            }
        }
    }

    /// Moves the window without redrawing it (dragging only changes the position).
    pub fn move_to(&mut self, x: i32, y: i32) {
        if self.visible && (self.rect.0, self.rect.1) != (x, y) {
            self.rect.0 = x;
            self.rect.1 = y;
            unsafe {
                SetWindowPos(
                    self.hwnd,
                    null_mut(),
                    x,
                    y,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                )
            };
        }
    }

    pub fn hide(&mut self) {
        if self.visible {
            unsafe { ShowWindow(self.hwnd, SW_HIDE) };
            self.visible = false;
        }
    }
}

impl Drop for Layer {
    fn drop(&mut self) {
        if !self.hwnd.is_null() {
            unsafe { DestroyWindow(self.hwnd) };
        }
    }
}
