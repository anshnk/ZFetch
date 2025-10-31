#[cfg(any(
    target_os = "linux",
    not(any(target_os = "windows", target_os = "macos", target_os = "linux"))
))]
pub fn detect_opengl_renderer() -> Option<String> {
    use std::ffi::CString;

    use glutin::config::ConfigTemplateBuilder;
    use glutin::context::{ContextApi, ContextAttributesBuilder};
    use glutin::display::GetGlDisplay;
    use glutin::prelude::*;
    use glutin_winit::{DisplayBuilder, GlWindow};
    use raw_window_handle::HasRawWindowHandle;
    use winit::event_loop::EventLoop;
    use winit::window::WindowBuilder;

    let event_loop = EventLoop::new();
    let display_builder = DisplayBuilder::new().with_window_builder(Some(WindowBuilder::new()));
    let template = ConfigTemplateBuilder::new();
    let (window, gl_config) = display_builder
        .build(&event_loop, template, |mut configs| {
            configs.next().expect("No GL config found")
        })
        .ok()?;

    let raw_window_handle = window.as_ref().map(|w| w.raw_window_handle());
    let gl_display = gl_config.display();

    let context_attributes = ContextAttributesBuilder::new().with_context_api(ContextApi::OpenGl(
        Some(glutin::context::Version::new(3, 3)),
    ));

    let not_current_gl_context = unsafe {
        gl_display
            .create_context(&gl_config, &context_attributes.build(raw_window_handle))
            .ok()?
    };

    let window = window?;
    let attrs = window.build_surface_attributes(<_>::default());
    let gl_surface = unsafe { gl_display.create_window_surface(&gl_config, &attrs).ok()? };
    let _gl_context = not_current_gl_context.make_current(&gl_surface).ok()?;

    gl::load_with(|symbol| {
        let c_str = CString::new(symbol).unwrap();
        gl_display.get_proc_address(&c_str) as *const _
    });

    let renderer = unsafe {
        let ptr = gl::GetString(gl::RENDERER);
        if ptr.is_null() {
            return None;
        }
        std::ffi::CStr::from_ptr(ptr as *const i8)
            .to_str()
            .ok()
            .map(|s| s.to_string())
    };
    renderer
}

#[cfg(any(
    target_os = "linux",
    not(any(target_os = "windows", target_os = "macos", target_os = "linux"))
))]
pub fn detect_vulkan_gpu() -> Option<String> {
    use ash::vk;
    use ash::Entry;

    let entry = unsafe { Entry::load().ok()? };
    let app_name = std::ffi::CString::new("ZFetch").unwrap();
    let engine_name = std::ffi::CString::new("No Engine").unwrap();
    let app_info = vk::ApplicationInfo::builder()
        .application_name(app_name.as_c_str())
        .application_version(vk::make_api_version(0, 1, 0, 0))
        .engine_name(engine_name.as_c_str())
        .engine_version(vk::make_api_version(0, 1, 0, 0))
        .api_version(vk::API_VERSION_1_0);
    let create_info = vk::InstanceCreateInfo::builder().application_info(&app_info);
    let instance = unsafe { entry.create_instance(&create_info, None).ok()? };
    let physical_devices = unsafe { instance.enumerate_physical_devices().ok()? };
    let props = unsafe { instance.get_physical_device_properties(physical_devices[0]) };
    let name = unsafe { std::ffi::CStr::from_ptr(props.device_name.as_ptr()) };
    Some(name.to_string_lossy().to_string())
}
