use std::{
    ops::Deref,
    os::fd::OwnedFd,
    sync::{Arc, RwLock},
};

use crate::{
    HaruhiShotState, TopLevel, WlOutputInfo,
    haruhierror::HaruhiError,
    overlay::LayerShellState,
    state::{CaptureInfo, CaptureState, FrameInfo},
    utils::{Position, Region, Size},
};
use image::ColorType;
use memmap2::MmapMut;
use tracing::debug;
use wayland_client::{
    EventQueue, WEnum,
    protocol::{
        wl_buffer::WlBuffer,
        wl_compositor::WlCompositor,
        wl_output::{self, WlOutput},
        wl_shm,
        wl_surface::WlSurface,
    },
};
use wayland_protocols::{
    ext::image_copy_capture::v1::client::{
        ext_image_copy_capture_frame_v1::FailureReason, ext_image_copy_capture_manager_v1::Options,
    },
    wp::viewporter::client::wp_viewporter::WpViewporter,
};

use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{Layer, ZwlrLayerShellV1},
    zwlr_layer_surface_v1::{Anchor, ZwlrLayerSurfaceV1},
};

use std::os::fd::{AsFd, AsRawFd};
use std::{
    fs::File,
    time::{SystemTime, UNIX_EPOCH},
};

use nix::{
    fcntl,
    sys::{memfd, mman, stat},
    unistd,
};

/// capture_output_frame.
fn create_shm_fd() -> std::io::Result<OwnedFd> {
    // Only try memfd on linux and freebsd.
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    loop {
        // Create a file that closes on successful execution and seal it's operations.
        match memfd::memfd_create(
            c"wayshot",
            memfd::MFdFlags::MFD_CLOEXEC | memfd::MFdFlags::MFD_ALLOW_SEALING,
        ) {
            Ok(fd) => {
                // This is only an optimization, so ignore errors.
                // F_SEAL_SRHINK = File cannot be reduced in size.
                // F_SEAL_SEAL = Prevent further calls to fcntl().
                let _ = fcntl::fcntl(
                    fd.as_fd(),
                    fcntl::F_ADD_SEALS(
                        fcntl::SealFlag::F_SEAL_SHRINK | fcntl::SealFlag::F_SEAL_SEAL,
                    ),
                );
                return Ok(fd);
            }
            Err(nix::errno::Errno::EINTR) => continue,
            Err(nix::errno::Errno::ENOSYS) => break,
            Err(errno) => return Err(std::io::Error::from(errno)),
        }
    }

    // Fallback to using shm_open.
    let sys_time = SystemTime::now();
    let mut mem_file_handle = format!(
        "/wayshot-{}",
        sys_time.duration_since(UNIX_EPOCH).unwrap().subsec_nanos()
    );
    loop {
        match mman::shm_open(
            // O_CREAT = Create file if does not exist.
            // O_EXCL = Error if create and file exists.
            // O_RDWR = Open for reading and writing.
            // O_CLOEXEC = Close on successful execution.
            // S_IRUSR = Set user read permission bit .
            // S_IWUSR = Set user write permission bit.
            mem_file_handle.as_str(),
            fcntl::OFlag::O_CREAT
                | fcntl::OFlag::O_EXCL
                | fcntl::OFlag::O_RDWR
                | fcntl::OFlag::O_CLOEXEC,
            stat::Mode::S_IRUSR | stat::Mode::S_IWUSR,
        ) {
            Ok(fd) => match mman::shm_unlink(mem_file_handle.as_str()) {
                Ok(_) => return Ok(fd),
                Err(errno) => match unistd::close(fd.as_raw_fd()) {
                    Ok(_) => return Err(std::io::Error::from(errno)),
                    Err(errno) => return Err(std::io::Error::from(errno)),
                },
            },
            Err(nix::errno::Errno::EEXIST) => {
                // If a file with that handle exists then change the handle
                mem_file_handle = format!(
                    "/wayshot-{}",
                    sys_time.duration_since(UNIX_EPOCH).unwrap().subsec_nanos()
                );
                continue;
            }
            Err(nix::errno::Errno::EINTR) => continue,
            Err(errno) => return Err(std::io::Error::from(errno)),
        }
    }
}

/// The data of the image, for the whole screen
#[derive(Debug, Clone)]
pub struct ImageInfo {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub color_type: ColorType,
}

#[allow(unused)]
#[derive(Debug, Clone)]
struct CaptureOutputData {
    output: WlOutput,
    buffer: WlBuffer,
    real_width: u32,
    real_height: u32,
    width: u32,
    height: u32,
    frame_bytes: u32,
    stride: u32,
    transform: wl_output::Transform,
    frame_format: wl_shm::Format,
    screen_position: Position,
}

#[allow(unused)]
#[derive(Debug, Clone)]
struct CaptureTopLevelData {
    buffer: WlBuffer,
    width: u32,
    height: u32,
    frame_bytes: u32,
    stride: u32,
    frame_format: wl_shm::Format,
    transform: wl_output::Transform,
}
/// Image view means what part to use
/// When use the project, every time you will get a picture of the full screen,
/// and when you do area screenshot, This lib will also provide you with the view of the selected
/// part
#[derive(Debug, Clone)]
pub struct ImageViewInfo {
    pub info: ImageInfo,
    pub region: Region,
}

/// Describe the capture option
/// Now this library provide two options
/// [CaptureOption::PaintCursors] and [CaptureOption::None]
/// It decides whether cursor will be shown
#[derive(Debug, Clone, Copy)]
pub enum CaptureOption {
    PaintCursors,
    None,
}

impl From<CaptureOption> for Options {
    fn from(val: CaptureOption) -> Self {
        match val {
            CaptureOption::None => Options::empty(),
            CaptureOption::PaintCursors => Options::PaintCursors,
        }
    }
}

pub trait AreaSelectCallback {
    fn slurp(self, state: &HaruhiShotState) -> Result<Region, HaruhiError>;
}

impl<F> AreaSelectCallback for F
where
    F: Fn(&HaruhiShotState) -> Result<Region, HaruhiError>,
{
    fn slurp(self, state: &HaruhiShotState) -> Result<Region, HaruhiError> {
        self(state)
    }
}
impl AreaSelectCallback for Region {
    fn slurp(self, _state: &HaruhiShotState) -> Result<Region, HaruhiError> {
        Ok(self)
    }
}
impl HaruhiShotState {
    fn capture_toplevel_inner<T: AsFd>(
        &mut self,
        TopLevel { handle, .. }: TopLevel,
        option: CaptureOption,
        fd: T,
        file: Option<&File>,
    ) -> Result<CaptureTopLevelData, HaruhiError> {
        let mut event_queue = self.take_event_queue();
        let img_manager = self.toplevel_image_manager();
        let capture_manager = self.image_copy_capture_manager();
        let qh = self.qhandle();

        let source = img_manager.create_source(&handle, qh, ());
        let info = Arc::new(RwLock::new(FrameInfo::default()));
        let session = capture_manager.create_session(&source, option.into(), qh, info.clone());

        let capture_info = CaptureInfo::new();
        let frame = session.create_frame(qh, capture_info.clone());
        event_queue.blocking_dispatch(self).unwrap();
        let qh = self.qhandle();

        let shm = self.shm();
        let info = info.read().unwrap();

        let Size { width, height } = info.size();
        let WEnum::Value(frame_format) = info.format() else {
            return Err(HaruhiError::NotSupportFormat);
        };
        if !matches!(
            frame_format,
            wl_shm::Format::Xbgr2101010
                | wl_shm::Format::Abgr2101010
                | wl_shm::Format::Argb8888
                | wl_shm::Format::Xrgb8888
                | wl_shm::Format::Xbgr8888
        ) {
            return Err(HaruhiError::NotSupportFormat);
        }
        let frame_bytes = 4 * height * width;
        let mem_fd = fd.as_fd();

        if let Some(file) = file {
            file.set_len(frame_bytes as u64).unwrap();
        }

        let stride = 4 * width;

        let shm_pool = shm.create_pool(mem_fd, (width * height * 4) as i32, qh, ());
        let buffer = shm_pool.create_buffer(
            0,
            width as i32,
            height as i32,
            stride as i32,
            frame_format,
            qh,
            (),
        );
        frame.attach_buffer(&buffer);
        frame.capture();

        let transform;
        loop {
            event_queue.blocking_dispatch(self)?;
            let info = capture_info.read().unwrap();
            match info.state() {
                CaptureState::Succeeded => {
                    transform = info.transform();
                    break;
                }
                CaptureState::Failed(info) => match info {
                    WEnum::Value(reason) => match reason {
                        FailureReason::Stopped => {
                            return Err(HaruhiError::CaptureFailed("Stopped".to_owned()));
                        }

                        FailureReason::BufferConstraints => {
                            return Err(HaruhiError::CaptureFailed("BufferConstraints".to_owned()));
                        }
                        FailureReason::Unknown | _ => {
                            return Err(HaruhiError::CaptureFailed("Unknown".to_owned()));
                        }
                    },
                    WEnum::Unknown(code) => {
                        return Err(HaruhiError::CaptureFailed(format!(
                            "Unknown reason, code : {code}"
                        )));
                    }
                },
                CaptureState::Pending => {}
            }
        }

        self.reset_event_queue(event_queue);

        Ok(CaptureTopLevelData {
            transform,
            buffer,
            width,
            height,
            frame_bytes,
            stride,
            frame_format,
        })
    }

    fn capture_output_inner<T: AsFd>(
        &mut self,
        WlOutputInfo {
            output,
            logical_size:
                Size {
                    width: real_width,
                    height: real_height,
                },
            position: screen_position,
            ..
        }: WlOutputInfo,
        option: CaptureOption,
        fd: T,
        file: Option<&File>,
    ) -> Result<CaptureOutputData, HaruhiError> {
        let mut event_queue = self.take_event_queue();
        let img_manager = self.output_image_manager();
        let capture_manager = self.image_copy_capture_manager();
        let qh = self.qhandle();

        let source = img_manager.create_source(&output, qh, ());
        let info = Arc::new(RwLock::new(FrameInfo::default()));
        let session = capture_manager.create_session(&source, option.into(), qh, info.clone());

        let capture_info = CaptureInfo::new();
        let frame = session.create_frame(qh, capture_info.clone());
        event_queue.blocking_dispatch(self).unwrap();
        let qh = self.qhandle();

        let shm = self.shm();
        let info = info.read().unwrap();

        let Size { width, height } = info.size();
        let WEnum::Value(frame_format) = info.format() else {
            return Err(HaruhiError::NotSupportFormat);
        };
        if !matches!(
            frame_format,
            wl_shm::Format::Xbgr2101010
                | wl_shm::Format::Abgr2101010
                | wl_shm::Format::Argb8888
                | wl_shm::Format::Xrgb8888
                | wl_shm::Format::Xbgr8888
        ) {
            return Err(HaruhiError::NotSupportFormat);
        }
        let frame_bytes = 4 * height * width;
        let mem_fd = fd.as_fd();

        if let Some(file) = file {
            file.set_len(frame_bytes as u64).unwrap();
        }

        let stride = 4 * width;

        let shm_pool = shm.create_pool(mem_fd, (width * height * 4) as i32, qh, ());
        let buffer = shm_pool.create_buffer(
            0,
            width as i32,
            height as i32,
            stride as i32,
            frame_format,
            qh,
            (),
        );
        frame.attach_buffer(&buffer);
        frame.capture();

        let transform;
        loop {
            event_queue.blocking_dispatch(self)?;
            let info = capture_info.read().unwrap();
            match info.state() {
                CaptureState::Succeeded => {
                    transform = info.transform();
                    break;
                }
                CaptureState::Failed(info) => match info {
                    WEnum::Value(reason) => match reason {
                        FailureReason::Stopped => {
                            return Err(HaruhiError::CaptureFailed("Stopped".to_owned()));
                        }

                        FailureReason::BufferConstraints => {
                            return Err(HaruhiError::CaptureFailed("BufferConstraints".to_owned()));
                        }
                        FailureReason::Unknown | _ => {
                            return Err(HaruhiError::CaptureFailed("Unknown".to_owned()));
                        }
                    },
                    WEnum::Unknown(code) => {
                        return Err(HaruhiError::CaptureFailed(format!(
                            "Unknown reason, code : {code}"
                        )));
                    }
                },
                CaptureState::Pending => {}
            }
        }

        self.reset_event_queue(event_queue);

        Ok(CaptureOutputData {
            output,
            buffer,
            width,
            height,
            frame_bytes,
            stride,
            frame_format,
            real_width: real_width as u32,
            real_height: real_height as u32,
            transform,
            screen_position,
        })
    }

    pub fn capture_single_output_with_fd<F: AsFd>(
        &mut self,
        option: CaptureOption,
        output: WlOutputInfo,
        file: F,
    ) -> Result<(), HaruhiError> {
        self.capture_output_inner(output, option, file.as_fd(), None)?;
        Ok(())
    }

    /// Capture a single output
    pub fn capture_single_output(
        &mut self,
        option: CaptureOption,
        output: WlOutputInfo,
    ) -> Result<ImageInfo, HaruhiError> {
        let mem_fd = create_shm_fd().unwrap();
        let mem_file = File::from(mem_fd);
        let CaptureOutputData {
            width,
            height,
            frame_format,
            ..
        } = self.capture_output_inner(output, option, mem_file.as_fd(), Some(&mem_file))?;

        let mut frame_mmap = unsafe { MmapMut::map_mut(&mem_file).unwrap() };

        let converter = crate::convert::create_converter(frame_format).unwrap();
        let color_type = converter.convert_inplace(&mut frame_mmap);

        Ok(ImageInfo {
            data: frame_mmap.deref().into(),
            width,
            height,
            color_type,
        })
    }

    /// Capture a single output
    pub fn capture_toplevel(
        &mut self,
        option: CaptureOption,
        toplevel: TopLevel,
    ) -> Result<ImageInfo, HaruhiError> {
        let mem_fd = create_shm_fd().unwrap();
        let mem_file = File::from(mem_fd);
        let CaptureTopLevelData {
            width,
            height,
            frame_format,
            ..
        } = self.capture_toplevel_inner(toplevel, option, mem_file.as_fd(), Some(&mem_file))?;

        let mut frame_mmap = unsafe { MmapMut::map_mut(&mem_file).unwrap() };

        let converter = crate::convert::create_converter(frame_format).unwrap();
        let color_type = converter.convert_inplace(&mut frame_mmap);

        Ok(ImageInfo {
            data: frame_mmap.deref().into(),
            width,
            height,
            color_type,
        })
    }

    /// capture with a area region
    pub fn capture_area<F>(
        &mut self,
        option: CaptureOption,
        callback: F,
    ) -> Result<ImageViewInfo, HaruhiError>
    where
        F: AreaSelectCallback,
    {
        let outputs = self.outputs().clone();

        let mut data_list = vec![];
        for data in outputs.into_iter() {
            let mem_fd = create_shm_fd().unwrap();
            let mem_file = File::from(mem_fd);
            let data =
                self.capture_output_inner(data, option, mem_file.as_fd(), Some(&mem_file))?;
            data_list.push(AreaShotInfo { data, mem_file })
        }

        let mut state = LayerShellState::new();
        let mut event_queue: EventQueue<LayerShellState> = self.connection().new_event_queue();
        let globals = self.globals();
        let qh = event_queue.handle();
        let compositor = globals.bind::<WlCompositor, _, _>(&qh, 3..=3, ())?;
        let layer_shell = globals.bind::<ZwlrLayerShellV1, _, _>(&qh, 1..=1, ())?;
        let viewporter = globals.bind::<WpViewporter, _, _>(&qh, 1..=1, ())?;
        let mut layer_shell_surfaces: Vec<(WlSurface, ZwlrLayerSurfaceV1)> =
            Vec::with_capacity(data_list.len());
        for AreaShotInfo { data, .. } in data_list.iter() {
            let CaptureOutputData {
                output,
                buffer,
                real_width,
                real_height,
                transform,
                ..
            } = data;
            let surface = compositor.create_surface(&qh, ());

            let layer_surface = layer_shell.get_layer_surface(
                &surface,
                Some(output),
                Layer::Overlay,
                "wayshot".to_string(),
                &qh,
                output.clone(),
            );

            layer_surface.set_exclusive_zone(-1);
            layer_surface.set_anchor(Anchor::all());
            layer_surface.set_margin(0, 0, 0, 0);

            debug!("Committing surface creation changes.");
            surface.commit();

            debug!("Waiting for layer surface to be configured.");
            while !state.configured_outputs.contains(output) {
                event_queue.blocking_dispatch(&mut state)?;
            }

            surface.set_buffer_transform(*transform);
            // surface.set_buffer_scale(output_info.scale());
            surface.attach(Some(buffer), 0, 0);

            let viewport = viewporter.get_viewport(&surface, &qh, ());
            viewport.set_destination(*real_width as i32, *real_height as i32);

            debug!("Committing surface with attached buffer.");
            surface.commit();
            layer_shell_surfaces.push((surface, layer_surface));
            event_queue.blocking_dispatch(&mut state)?;
        }

        let region_re = callback.slurp(self);

        debug!("Unmapping and destroying layer shell surfaces.");
        for (surface, layer_shell_surface) in layer_shell_surfaces.iter() {
            surface.attach(None, 0, 0);
            surface.commit(); //unmap surface by committing a null buffer
            layer_shell_surface.destroy();
        }
        event_queue.roundtrip(&mut state)?;
        let region = region_re?;

        let shotdata = data_list
            .iter()
            .find(|data| data.in_this_screen(region))
            .ok_or(HaruhiError::CaptureFailed("not in region".to_owned()))?;
        let area = shotdata.clip_area(region).expect("should have");
        let mut frame_mmap = unsafe { MmapMut::map_mut(&shotdata.mem_file).unwrap() };

        let converter = crate::convert::create_converter(shotdata.data.frame_format).unwrap();
        let color_type = converter.convert_inplace(&mut frame_mmap);

        Ok(ImageViewInfo {
            info: ImageInfo {
                data: frame_mmap.deref().into(),
                width: shotdata.data.width,
                height: shotdata.data.height,
                color_type,
            },
            region: area,
        })
    }
}

struct AreaShotInfo {
    data: CaptureOutputData,
    mem_file: File,
}

impl AreaShotInfo {
    fn in_this_screen(
        &self,
        Region {
            position: point, ..
        }: Region,
    ) -> bool {
        let CaptureOutputData {
            real_width,
            real_height,
            screen_position: Position { x, y },
            ..
        } = self.data;
        if point.y < y
            || point.x < x
            || point.x > x + real_width as i32
            || point.y > y + real_height as i32
        {
            return false;
        }
        true
    }
    fn clip_area(&self, region: Region) -> Option<Region> {
        if !self.in_this_screen(region) {
            return None;
        }
        let CaptureOutputData {
            real_width,
            real_height,
            width,
            height,
            screen_position,
            ..
        } = self.data;
        let Region {
            position: point,
            size,
        } = region;
        let relative_point = point - screen_position;
        let position = Position {
            x: (relative_point.x as f64 * width as f64 / real_width as f64) as i32,
            y: (relative_point.y as f64 * height as f64 / real_height as f64) as i32,
        };

        Some(Region {
            position,
            size: Size {
                width: (size.width as f64 * width as f64 / real_width as f64) as i32,
                height: (size.height as f64 * height as f64 / real_height as f64) as i32,
            },
        })
    }
}
