use image::ColorType;
use wayland_client::protocol::wl_buffer::WlBuffer;
use wayland_client::protocol::wl_output::{self, WlOutput};
use wayland_client::protocol::wl_shm::{self, Format};
use wayland_client::protocol::wl_shm_pool::WlShmPool;
use wayland_client::QueueHandle;
use wayland_client::{Connection, Dispatch, WEnum};
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_frame_v1::{
    self, ZwlrScreencopyFrameV1,
};

use std::os::fd::{AsFd, AsRawFd, OwnedFd};
use std::{
    ffi::CStr,
    fs::File,
    time::{SystemTime, UNIX_EPOCH},
};

use nix::{
    fcntl,
    sys::{memfd, mman, stat},
    unistd,
};

use crate::convert;
use memmap2::MmapMut;

use crate::haruhierror::HaruhiError;
use crate::wlrshotbasestate::HaruhiShotState;

/// mark the area selected
/// from x, y, width, height
/// mark for [Option<(i32,i32,i32,i32)>]
pub enum SlurpArea {
    None,
    Area {
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    },
}

impl From<Option<(i32, i32, i32, i32)>> for SlurpArea {
    fn from(value: Option<(i32, i32, i32, i32)>) -> Self {
        match value {
            None => Self::None,
            Some((x, y, width, height)) => Self::Area {
                x,
                y,
                width,
                height,
            },
        }
    }
}

impl From<SlurpArea> for Option<(i32, i32, i32, i32)> {
    fn from(value: SlurpArea) -> Self {
        match value {
            SlurpArea::None => None,
            SlurpArea::Area {
                x,
                y,
                width,
                height,
            } => Some((x, y, width, height)),
        }
    }
}

#[derive(Debug)]
enum ScreenCopyState {
    Staging,
    Pedding,
    Finished,
    Failed,
}

/// About the information of the frame
pub struct FrameInfo {
    /// frameformat: [FrameFormat]
    pub frameformat: FrameFormat,
    /// frame_mmap: contain the information of an image
    pub frame_mmap: MmapMut,

    pub frame_color_type: ColorType,
    /// transform: how the screen is layed
    pub transform: wl_output::Transform,
    /// realwidth: same to above
    pub realwidth: u32,
    /// realheight: it is the height when you selected, that is phycis height
    pub realheight: u32,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct FrameFormat {
    /// contain [Format], which in wl_shm
    pub format: Format,
    /// width: during dispatch , get the width of image
    pub width: u32,
    /// height: during dispatch, get the height of image
    pub height: u32,
    stride: u32,
}

pub(crate) struct WlrCopyStateInfo {
    state: ScreenCopyState,
    formats: Vec<FrameFormat>,
}

impl WlrCopyStateInfo {
    pub(crate) fn init() -> Self {
        Self {
            state: ScreenCopyState::Staging,
            formats: Vec::new(),
        }
    }
}

/// capture_output_frame.
fn create_shm_fd() -> std::io::Result<OwnedFd> {
    // Only try memfd on linux and freebsd.
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    loop {
        // Create a file that closes on succesful execution and seal it's operations.
        match memfd::memfd_create(
            CStr::from_bytes_with_nul(b"wayshot\0").unwrap(),
            memfd::MemFdCreateFlag::MFD_CLOEXEC | memfd::MemFdCreateFlag::MFD_ALLOW_SEALING,
        ) {
            Ok(fd) => {
                // This is only an optimization, so ignore errors.
                // F_SEAL_SRHINK = File cannot be reduced in size.
                // F_SEAL_SEAL = Prevent further calls to fcntl().
                let _ = fcntl::fcntl(
                    fd.as_raw_fd(),
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
            // O_CLOEXEC = Close on succesful execution.
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

impl Dispatch<WlBuffer, ()> for HaruhiShotState {
    fn event(
        _state: &mut Self,
        _proxy: &WlBuffer,
        _event: <WlBuffer as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlShmPool, ()> for HaruhiShotState {
    fn event(
        _state: &mut Self,
        _proxy: &WlShmPool,
        _event: <WlShmPool as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwlrScreencopyFrameV1, ()> for HaruhiShotState {
    fn event(
        state: &mut Self,
        _proxy: &ZwlrScreencopyFrameV1,
        event: <ZwlrScreencopyFrameV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qh: &wayland_client::QueueHandle<Self>,
    ) {
        match event {
            zwlr_screencopy_frame_v1::Event::Ready {
                ..
                //tv_sec_hi,
                //tv_sec_lo,
                //tv_nsec,
            } => {
                state.wlr_copy_state_info.state = ScreenCopyState::Finished;
                tracing::info!("Receive Ready event");
            }
            zwlr_screencopy_frame_v1::Event::Buffer {
                format,
                width,
                height,
                stride,
            } => {
                let format = match format {
                    WEnum::Value(value) => {
                        value
                    },
                    WEnum::Unknown(e) => {
                        tracing::error!("Unknown format :{}",e);
                        state.wlr_copy_state_info.state = ScreenCopyState::Failed;
                        return;
                    }
                };
                tracing::info!("Format is {:?}", format);
                state.wlr_copy_state_info.formats.push(FrameFormat {
                    format,
                    width,
                    height,
                    stride,
                });
                state.wlr_copy_state_info.state = ScreenCopyState::Pedding;
                    // buffer done
            }
            zwlr_screencopy_frame_v1::Event::LinuxDmabuf {
                ..
            } => {
                tracing::info!("Receive LinuxDamBuf event");
            }
            zwlr_screencopy_frame_v1::Event::Damage {
                ..
            } => {
                tracing::info!("Receive Damage event");
            }
            zwlr_screencopy_frame_v1::Event::BufferDone => {
                tracing::info!("Receive BufferDone event");
            }
            zwlr_screencopy_frame_v1::Event::Flags { .. } => {
                tracing::info!("Receive Flags event");
            }
            zwlr_screencopy_frame_v1::Event::Failed => {
                tracing::info!("Receive failed event");
                state.wlr_copy_state_info.state = ScreenCopyState::Failed;
            }
            _ => unreachable!()
        }
    }
}

impl HaruhiShotState {
    #[inline]
    fn finished(&self) -> bool {
        matches!(
            self.wlr_copy_state_info.state,
            ScreenCopyState::Failed | ScreenCopyState::Finished
        )
    }

    #[inline]
    fn is_pedding(&self) -> bool {
        matches!(self.wlr_copy_state_info.state, ScreenCopyState::Pedding)
    }

    /// capture a frame, it will return [FrameInfo], or [HarihiError]
    /// with frameinfo, you can use it to create image
    /// realwidth and realheight  it is the logic width and height you choose
    /// finally the image will resize as the width and height privided here
    /// because the image capture by wm will not be the same size you choose
    /// slurpoption please view [SlurpArea], it accepts a area, when capture region
    pub fn capture_output_frame<T>(
        &mut self,
        output: &WlOutput,
        (realwidth, realheight): (i32, i32),
        transform: wl_output::Transform,
        slurpoption: T,
    ) -> Result<Option<FrameInfo>, HaruhiError>
    where
        T: Into<SlurpArea>,
    {
        let manager = self.wlr_screencopy.as_ref().unwrap();

        tracing::info!("windowinfo ==> width :{realwidth}, height: {realheight}");
        let qh = self.get_event_queue_handle()?;
        let frame = match slurpoption.into() {
            SlurpArea::None => manager.capture_output(0, output, &qh, ()),
            SlurpArea::Area {
                x,
                y,
                width,
                height,
            } => manager.capture_output_region(0, output, x, y, width, height, &qh, ()),
        };
        let mut frameformat: Option<FrameFormat> = None;
        let mut frame_mmap: Option<MmapMut> = None;
        let frame_color_type;
        loop {
            self.block_dispatch()?;
            if self.finished() {
                let frame_mmap = frame_mmap.as_mut().unwrap();
                let frame_format = frameformat.as_ref().unwrap();
                let frame_color_type_converter = convert::create_converter(frame_format.format)
                    .ok_or(HaruhiError::NotSupportFormat)?;
                frame_color_type = frame_color_type_converter.convert_inplace(frame_mmap);
                break;
            }
            if self.is_pedding() {
                frameformat = self
                    .wlr_copy_state_info
                    .formats
                    .iter()
                    .find(|frame| {
                        matches!(
                            frame.format,
                            wl_shm::Format::Xbgr2101010
                                | wl_shm::Format::Abgr2101010
                                | wl_shm::Format::Argb8888
                                | wl_shm::Format::Xrgb8888
                                | wl_shm::Format::Xbgr8888
                        )
                    })
                    .copied();
                let frame_format = frameformat.as_ref().ok_or_else(|| {
                    HaruhiError::QueueError("Canot find a frameformat".to_string())
                })?;
                let frame_bytes = frame_format.stride * frame_format.height;
                let mem_fd = create_shm_fd()?;
                let mem_file = File::from(mem_fd);
                mem_file.set_len(frame_bytes as u64)?;

                let shm_pool = self.shm.as_ref().unwrap().create_pool(
                    mem_file.as_fd(),
                    frame_bytes as i32,
                    &qh,
                    (),
                );

                let buffer = shm_pool.create_buffer(
                    0,
                    frame_format.width as i32,
                    frame_format.height as i32,
                    frame_format.stride as i32,
                    frame_format.format,
                    &qh,
                    (),
                );
                frame.copy(&buffer);

                // TODO:maybe need some adjust
                frame_mmap = Some(unsafe { MmapMut::map_mut(&mem_file)? });
            }
        }
        match self.wlr_copy_state_info.state {
            ScreenCopyState::Finished => {
                self.wlr_copy_state_info.formats.clear();
                self.wlr_copy_state_info.state = ScreenCopyState::Staging;
                let output = FrameInfo {
                    frameformat: frameformat.unwrap(),
                    frame_mmap: frame_mmap.unwrap(),
                    frame_color_type,
                    transform,
                    realwidth: realwidth as u32,
                    realheight: realheight as u32,
                };
                Ok(Some(output))
            }
            ScreenCopyState::Failed => {
                tracing::error!("Cannot take screen copy");
                Ok(None)
            }
            _ => unreachable!(),
        }
    }
}
