use std::{
    ffi::{CStr, c_void},
    ptr::{self, NonNull},
    rc::Rc,
    sync::Arc,
};

use anyhow::{Context, Result, anyhow};
use gpui::{Bounds, DevicePixels, Window, WindowCompositionSurface, point, px};
use libmpv2::Mpv;
use libmpv2_sys as sys;
use tokio::sync::mpsc::{self, Receiver, Sender};
use windows::{
    Win32::Graphics::{
        Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_UNKNOWN},
        Direct3D11::{
            D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION, D3D11CreateDevice, ID3D11Device,
            ID3D11DeviceContext, ID3D11Texture2D,
        },
        DirectComposition::IDCompositionVisual,
        Dxgi::{
            Common::{DXGI_ALPHA_MODE_IGNORE, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC},
            CreateDXGIFactory1, DXGI_PRESENT, DXGI_SCALING_STRETCH, DXGI_SWAP_CHAIN_DESC1,
            DXGI_SWAP_CHAIN_FLAG, DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
            DXGI_USAGE_RENDER_TARGET_OUTPUT, IDXGIAdapter, IDXGIAdapter1, IDXGIDevice,
            IDXGIFactory2, IDXGISwapChain1,
        },
    },
    core::{IUnknown, Interface},
};

// libmpv2-sys predates the D3D11 declarations in mpv 0.42.0's render.h and
// render_d3d11.h. The two C parameter structs each contain one COM pointer.
const D3D11_INIT_PARAMS: sys::mpv_render_param_type = 21;
const D3D11_FBO: sys::mpv_render_param_type = 22;

#[repr(C)]
struct D3d11RenderParam {
    interface: *mut c_void,
}

pub(super) struct VideoRenderer {
    // The render context must be freed before its owner Mpv is dropped.
    render: RenderContext,
    device: ID3D11Device,
    immediate: ID3D11DeviceContext,
    surface: Option<VideoSurface>,
}

struct VideoSurface {
    portal: WindowCompositionSurface,
    swap_chain: IDXGISwapChain1,
    bounds: Bounds<DevicePixels>,
}

impl VideoRenderer {
    pub(super) fn new(mpv: Arc<Mpv>, adapter: Option<u32>) -> Result<(Self, Receiver<()>)> {
        let (device, immediate) = create_device(adapter)?;
        let (render, updates) = RenderContext::new(mpv, &device)?;
        Ok((
            Self {
                render,
                device,
                immediate,
                surface: None,
            },
            updates,
        ))
    }

    pub(super) fn attach(&mut self, window: &Window) -> Result<()> {
        let bounds = device_bounds(window);
        let desc = DXGI_SWAP_CHAIN_DESC1 {
            Width: bounds.size.width.0.max(1) as u32,
            Height: bounds.size.height.0.max(1) as u32,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            BufferCount: 2,
            Scaling: DXGI_SCALING_STRETCH,
            SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
            AlphaMode: DXGI_ALPHA_MODE_IGNORE,
            ..Default::default()
        };
        let swap_chain = unsafe {
            let dxgi: IDXGIDevice = self.device.cast()?;
            let factory: IDXGIFactory2 = dxgi.GetAdapter()?.GetParent()?;
            factory.CreateSwapChainForComposition(&self.device, &desc, None)?
        };
        let composition = window.enable_window_composition()?;
        let portal = composition.create_native_surface()?;
        let attach = (|| {
            let attachment = portal.platform_surface()?;
            set_visual_content(attachment.platform_handle()?, &swap_chain)?;
            let chain = swap_chain.clone();
            attachment.set_compositor_recreated_callback(Rc::new(move |visual| {
                set_visual_content(visual, &chain)
            }))?;
            attachment.set_bounds(bounds)
        })();
        if let Err(error) = attach {
            let _ = composition.remove_surface(portal.id());
            return Err(error);
        }
        self.surface = Some(VideoSurface {
            portal,
            swap_chain,
            bounds,
        });
        eprintln!("D3D11 video swap chain attached to the GPUI window");
        Ok(())
    }

    pub(super) fn detach(&mut self, window: &Window) {
        if let Some(surface) = self.surface.take()
            && let Err(error) = window
                .enable_window_composition()
                .and_then(|composition| composition.remove_surface(surface.portal.id()))
        {
            eprintln!("Detach D3D11 video surface: {error:#}");
        }
    }

    pub(super) fn draw(&mut self, window: &Window) -> Result<()> {
        let surface = self
            .surface
            .as_mut()
            .context("missing D3D11 video surface")?;
        let bounds = device_bounds(window);
        if bounds.size.width.0 <= 0 || bounds.size.height.0 <= 0 {
            return Ok(());
        }
        let resized = bounds != surface.bounds;
        if resized {
            // Drop any immediate-context references left by the last mpv draw.
            unsafe {
                self.immediate.ClearState();
                self.immediate.Flush();
                surface.swap_chain.ResizeBuffers(
                    0,
                    bounds.size.width.0 as u32,
                    bounds.size.height.0 as u32,
                    DXGI_FORMAT_B8G8R8A8_UNORM,
                    DXGI_SWAP_CHAIN_FLAG(0),
                )?;
            }
            surface.portal.platform_surface()?.set_bounds(bounds)?;
            surface.bounds = bounds;
        }
        let flags = unsafe { sys::mpv_render_context_update(self.render.raw.as_ptr()) };
        if resized || flags & u64::from(sys::mpv_render_update_flag_MPV_RENDER_UPDATE_FRAME) != 0 {
            let texture = unsafe { surface.swap_chain.GetBuffer::<ID3D11Texture2D>(0)? };
            self.render.draw(&texture)?;
            unsafe {
                surface.swap_chain.Present(0, DXGI_PRESENT(0)).ok()?;
                sys::mpv_render_context_report_swap(self.render.raw.as_ptr());
            }
        }
        Ok(())
    }
}

fn device_bounds(window: &Window) -> Bounds<DevicePixels> {
    Bounds::new(point(px(0.), px(0.)), window.viewport_size())
        .to_device_pixels(window.scale_factor())
}

fn set_visual_content(handle: Box<dyn std::any::Any>, swap_chain: &IDXGISwapChain1) -> Result<()> {
    let handle = handle
        .downcast::<IUnknown>()
        .map_err(|_| anyhow!("GPUI did not provide a DirectComposition visual"))?;
    let visual: IDCompositionVisual = handle.cast()?;
    unsafe { visual.SetContent(swap_chain)? };
    Ok(())
}

fn create_device(adapter_index: Option<u32>) -> Result<(ID3D11Device, ID3D11DeviceContext)> {
    let adapter: Option<IDXGIAdapter> = adapter_index
        .map(|index| unsafe {
            let factory = CreateDXGIFactory1::<IDXGIFactory2>()?;
            factory.EnumAdapters1(index)?.cast()
        })
        .transpose()?;
    let driver = if adapter.is_some() {
        D3D_DRIVER_TYPE_UNKNOWN
    } else {
        D3D_DRIVER_TYPE_HARDWARE
    };
    let mut device = None;
    let mut immediate = None;
    unsafe {
        D3D11CreateDevice(
            adapter.as_ref(),
            driver,
            Default::default(),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            None,
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            Some(&mut immediate),
        )
        .context("create D3D11 video device")?;
    }
    let device = device.context("D3D11 did not return a device")?;
    let immediate = immediate.context("D3D11 did not return an immediate context")?;
    let dxgi: IDXGIDevice = device.cast()?;
    let adapter: IDXGIAdapter1 = unsafe { dxgi.GetAdapter()?.cast()? };
    let desc = unsafe { adapter.GetDesc1()? };
    eprintln!(
        "D3D11 video adapter: {}",
        String::from_utf16_lossy(&desc.Description).trim_end_matches('\0')
    );
    Ok((device, immediate))
}

struct RenderContext {
    raw: NonNull<sys::mpv_render_context>,
    _mpv: Arc<Mpv>,
    updates: Box<Sender<()>>,
}

impl RenderContext {
    fn new(mpv: Arc<Mpv>, device: &ID3D11Device) -> Result<(Self, Receiver<()>)> {
        let mut init = D3d11RenderParam {
            interface: device.as_raw(),
        };
        let mut params = [
            sys::mpv_render_param {
                type_: sys::mpv_render_param_type_MPV_RENDER_PARAM_API_TYPE,
                data: c"d3d11".as_ptr().cast_mut().cast(),
            },
            render_param(D3D11_INIT_PARAMS, &mut init),
            param_end(),
        ];
        let mut raw = ptr::null_mut();
        mpv_result(unsafe {
            sys::mpv_render_context_create(&mut raw, mpv.ctx.as_ptr(), params.as_mut_ptr())
        })
        .context("initialize libmpv D3D11 render API")?;
        let (sender, receiver) = mpsc::channel(1);
        let mut render = Self {
            raw: NonNull::new(raw).context("libmpv did not return a render context")?,
            _mpv: mpv,
            updates: Box::new(sender),
        };
        unsafe {
            sys::mpv_render_context_set_update_callback(
                raw,
                Some(wake_render),
                ptr::from_mut(render.updates.as_mut()).cast(),
            );
        }
        Ok((render, receiver))
    }

    fn draw(&self, texture: &ID3D11Texture2D) -> Result<()> {
        let mut target = D3d11RenderParam {
            interface: texture.as_raw(),
        };
        let mut block_for_target_time = 0_i32;
        let mut params = [
            render_param(D3D11_FBO, &mut target),
            render_param(
                sys::mpv_render_param_type_MPV_RENDER_PARAM_BLOCK_FOR_TARGET_TIME,
                &mut block_for_target_time,
            ),
            param_end(),
        ];
        mpv_result(unsafe {
            sys::mpv_render_context_render(self.raw.as_ptr(), params.as_mut_ptr())
        })
        .context("render video into the DXGI back buffer")
    }
}

impl Drop for RenderContext {
    fn drop(&mut self) {
        unsafe {
            sys::mpv_render_context_set_update_callback(self.raw.as_ptr(), None, ptr::null_mut());
            sys::mpv_render_context_free(self.raw.as_ptr());
        }
    }
}

unsafe extern "C" fn wake_render(userdata: *mut c_void) {
    // libmpv may call back on any thread. One queued wake is enough.
    let sender = unsafe { &*userdata.cast::<Sender<()>>() };
    let _ = sender.try_send(());
}

fn render_param<T>(type_: sys::mpv_render_param_type, data: &mut T) -> sys::mpv_render_param {
    sys::mpv_render_param {
        type_,
        data: ptr::from_mut(data).cast(),
    }
}

fn param_end() -> sys::mpv_render_param {
    sys::mpv_render_param {
        type_: sys::mpv_render_param_type_MPV_RENDER_PARAM_INVALID,
        data: ptr::null_mut(),
    }
}

fn mpv_result(status: i32) -> Result<()> {
    if status < 0 {
        let error = unsafe { CStr::from_ptr(sys::mpv_error_string(status)) };
        return Err(anyhow!("{}", error.to_string_lossy()));
    }
    Ok(())
}
