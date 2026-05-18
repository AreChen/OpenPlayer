#![allow(dead_code)]

use std::{ffi::c_void, mem, ptr};

use windows_sys::Win32::{
    Foundation::{HINSTANCE, HWND, RECT},
    Graphics::{
        Gdi::{GetDC, ReleaseDC},
        OpenGL::{
            ChoosePixelFormat, HGLRC, PFD_DOUBLEBUFFER, PFD_DRAW_TO_WINDOW, PFD_MAIN_PLANE,
            PFD_SUPPORT_OPENGL, PFD_TYPE_RGBA, PIXELFORMATDESCRIPTOR, SetPixelFormat, SwapBuffers,
            wglCreateContext, wglDeleteContext, wglGetProcAddress, wglMakeCurrent,
        },
    },
    System::LibraryLoader::{GetModuleHandleW, GetProcAddress},
    UI::WindowsAndMessaging::{
        CS_OWNDC, CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, HWND_BOTTOM,
        RegisterClassW, SWP_NOACTIVATE, SetWindowPos, WNDCLASSW, WS_CHILD, WS_CLIPCHILDREN,
        WS_CLIPSIBLINGS, WS_VISIBLE,
    },
};

use super::RenderViewport;

const SURFACE_CLASS_NAME: &str = "OpenPlayerRenderSurface";
const OPENGL32_DLL: &str = "opengl32.dll";

pub struct Win32RenderSurface {
    parent_hwnd: HWND,
    hwnd: HWND,
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    hglrc: HGLRC,
    viewport: RenderViewport,
}

impl Win32RenderSurface {
    pub unsafe fn new(parent_hwnd: HWND) -> Result<Self, String> {
        if parent_hwnd.is_null() {
            return Err("parent HWND is null".to_string());
        }

        let hinstance = unsafe { GetModuleHandleW(ptr::null()) } as HINSTANCE;
        if hinstance.is_null() {
            return Err("failed to get module handle".to_string());
        }

        let class_name = wide_null(SURFACE_CLASS_NAME);
        let window_class = WNDCLASSW {
            style: CS_OWNDC,
            lpfnWndProc: Some(DefWindowProcW),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: ptr::null_mut(),
            hCursor: ptr::null_mut(),
            hbrBackground: ptr::null_mut(),
            lpszMenuName: ptr::null(),
            lpszClassName: class_name.as_ptr(),
        };

        unsafe { RegisterClassW(&window_class) };

        let mut rect = RECT::default();
        if unsafe { GetClientRect(parent_hwnd, &mut rect) } == 0 {
            return Err("failed to get parent client rect".to_string());
        }

        let viewport = viewport_from_client_size(rect.right - rect.left, rect.bottom - rect.top);
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                class_name.as_ptr(),
                class_name.as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS | WS_CLIPCHILDREN,
                viewport.x,
                viewport.y,
                viewport.width,
                viewport.height,
                parent_hwnd,
                ptr::null_mut(),
                hinstance,
                ptr::null_mut(),
            )
        };
        if hwnd.is_null() {
            return Err("failed to create render surface window".to_string());
        }

        unsafe {
            SetWindowPos(
                hwnd,
                HWND_BOTTOM,
                viewport.x,
                viewport.y,
                viewport.width,
                viewport.height,
                SWP_NOACTIVATE,
            );
        }

        let hdc = unsafe { GetDC(hwnd) };
        if hdc.is_null() {
            unsafe { DestroyWindow(hwnd) };
            return Err("failed to get render surface DC".to_string());
        }

        let pixel_format_descriptor = PIXELFORMATDESCRIPTOR {
            nSize: mem::size_of::<PIXELFORMATDESCRIPTOR>() as u16,
            nVersion: 1,
            dwFlags: PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER,
            iPixelType: PFD_TYPE_RGBA,
            cColorBits: 24,
            cRedBits: 0,
            cRedShift: 0,
            cGreenBits: 0,
            cGreenShift: 0,
            cBlueBits: 0,
            cBlueShift: 0,
            cAlphaBits: 8,
            cAlphaShift: 0,
            cAccumBits: 0,
            cAccumRedBits: 0,
            cAccumGreenBits: 0,
            cAccumBlueBits: 0,
            cAccumAlphaBits: 0,
            cDepthBits: 24,
            cStencilBits: 8,
            cAuxBuffers: 0,
            iLayerType: PFD_MAIN_PLANE as u8,
            bReserved: 0,
            dwLayerMask: 0,
            dwVisibleMask: 0,
            dwDamageMask: 0,
        };

        let pixel_format = unsafe { ChoosePixelFormat(hdc, &pixel_format_descriptor) };
        if pixel_format == 0 {
            unsafe {
                ReleaseDC(hwnd, hdc);
                DestroyWindow(hwnd);
            }
            return Err("failed to choose OpenGL pixel format".to_string());
        }

        if unsafe { SetPixelFormat(hdc, pixel_format, &pixel_format_descriptor) } == 0 {
            unsafe {
                ReleaseDC(hwnd, hdc);
                DestroyWindow(hwnd);
            }
            return Err("failed to set OpenGL pixel format".to_string());
        }

        let hglrc = unsafe { wglCreateContext(hdc) };
        if hglrc.is_null() {
            unsafe {
                ReleaseDC(hwnd, hdc);
                DestroyWindow(hwnd);
            }
            return Err("failed to create WGL context".to_string());
        }

        Ok(Self {
            parent_hwnd,
            hwnd,
            hdc,
            hglrc,
            viewport,
        })
    }

    pub fn make_current(&self) -> Result<(), String> {
        if unsafe { wglMakeCurrent(self.hdc, self.hglrc) } == 0 {
            return Err("failed to make WGL context current".to_string());
        }

        Ok(())
    }

    pub fn swap_buffers(&self) -> Result<(), String> {
        if unsafe { SwapBuffers(self.hdc) } == 0 {
            return Err("failed to swap render surface buffers".to_string());
        }

        Ok(())
    }

    pub fn resize_to_parent(&mut self) -> Result<RenderViewport, String> {
        let mut rect = RECT::default();
        if unsafe { GetClientRect(self.parent_hwnd, &mut rect) } == 0 {
            return Err("failed to get parent client rect".to_string());
        }

        self.viewport = viewport_from_client_size(rect.right - rect.left, rect.bottom - rect.top);
        if unsafe {
            SetWindowPos(
                self.hwnd,
                HWND_BOTTOM,
                self.viewport.x,
                self.viewport.y,
                self.viewport.width,
                self.viewport.height,
                SWP_NOACTIVATE,
            )
        } == 0
        {
            return Err("failed to resize render surface".to_string());
        }

        Ok(self.viewport)
    }

    pub fn viewport(&self) -> RenderViewport {
        self.viewport
    }
}

impl Drop for Win32RenderSurface {
    fn drop(&mut self) {
        unsafe {
            wglMakeCurrent(ptr::null_mut(), ptr::null_mut());

            if !self.hglrc.is_null() {
                wglDeleteContext(self.hglrc);
                self.hglrc = ptr::null_mut();
            }

            if !self.hdc.is_null() {
                ReleaseDC(self.hwnd, self.hdc);
                self.hdc = ptr::null_mut();
            }

            if !self.hwnd.is_null() {
                DestroyWindow(self.hwnd);
                self.hwnd = ptr::null_mut();
            }
        }
    }
}

pub fn wide_null(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn viewport_from_client_size(width: i32, height: i32) -> RenderViewport {
    RenderViewport {
        x: 0,
        y: 0,
        width: width.max(1),
        height: height.max(1),
    }
}

pub unsafe extern "C" fn get_proc_address(_ctx: *mut c_void, name: *const i8) -> *mut c_void {
    if name.is_null() {
        return ptr::null_mut();
    }

    let proc_address = unsafe { wglGetProcAddress(name.cast()) };
    let address = proc_address.map_or(ptr::null_mut(), |proc| proc as *const () as *mut c_void);
    if is_valid_wgl_proc_address(address) {
        return address;
    }

    let opengl32 = wide_null(OPENGL32_DLL);
    let module = unsafe { GetModuleHandleW(opengl32.as_ptr()) };
    if module.is_null() {
        return ptr::null_mut();
    }

    unsafe { GetProcAddress(module, name.cast()) }
        .map_or(ptr::null_mut(), |proc| proc as *const () as *mut c_void)
}

fn is_valid_wgl_proc_address(address: *mut c_void) -> bool {
    !matches!(address as usize, 0 | 1 | 2 | 3 | usize::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpv_render::RenderViewport;

    #[test]
    fn wide_null_appends_one_terminator() {
        assert_eq!(
            wide_null("OpenPlayer"),
            vec![79, 112, 101, 110, 80, 108, 97, 121, 101, 114, 0]
        );
    }

    #[test]
    fn viewport_from_client_size_clamps_zero_and_negative_dimensions() {
        assert_eq!(
            viewport_from_client_size(0, -10),
            RenderViewport {
                x: 0,
                y: 0,
                width: 1,
                height: 1,
            }
        );
    }

    #[test]
    fn rejects_wgl_proc_address_sentinel_values() {
        assert!(!is_valid_wgl_proc_address(ptr::null_mut()));
        assert!(!is_valid_wgl_proc_address(1usize as *mut c_void));
        assert!(!is_valid_wgl_proc_address(2usize as *mut c_void));
        assert!(!is_valid_wgl_proc_address(3usize as *mut c_void));
        assert!(!is_valid_wgl_proc_address(usize::MAX as *mut c_void));
        assert!(is_valid_wgl_proc_address(4usize as *mut c_void));
    }
}
