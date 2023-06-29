use wayland_client::protocol::wl_buffer::WlBuffer;
use wayland_client::protocol::wl_output::{self, WlOutput};
use wayland_client::protocol::wl_shm::{self, Format};
use wayland_client::protocol::wl_shm_pool::WlShmPool;
use wayland_client::QueueHandle;
use wayland_client::{Connection, Dispatch, EventQueue, WEnum};
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_frame_v1;
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1;

use std::error::Error;
use std::os::fd::FromRawFd;
use std::{
    ffi::CStr,
    fs::File,
    os::unix::prelude::RawFd,
    time::{SystemTime, UNIX_EPOCH},
};

use nix::{
    fcntl,
    sys::{memfd, mman, stat},
    unistd,
};

use memmap2::MmapMut;

use crate::wlrcaptruestate::AppData;

#[derive(Debug)]
pub enum ScreenCopyState {
    Staging,
    Pedding,
    Finished,
    Failed,
}

pub struct FrameInfo {
    pub frameformat: FrameFormat,
    pub frame_mmap: MmapMut,
    pub transform: wl_output::Transform,
    pub realwidth: u32,
    pub realheight: u32,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct FrameFormat {
    pub format: Format,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
}
/// capture_output_frame.
fn create_shm_fd() -> std::io::Result<RawFd> {
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
                    fd,
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
                Err(errno) => match unistd::close(fd) {
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

impl Dispatch<WlBuffer, ()> for AppData {
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

impl Dispatch<WlShmPool, ()> for AppData {
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

impl Dispatch<ZwlrScreencopyFrameV1, ()> for AppData {
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
                state.state = ScreenCopyState::Finished;
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
                        state.state = ScreenCopyState::Failed;
                        return;
                    }
                };
                tracing::info!("Format is {:?}", format);
                state.formats.push(FrameFormat {
                    format,
                    width,
                    height,
                    stride,
                });
                state.state = ScreenCopyState::Pedding;
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
                state.state = ScreenCopyState::Failed;
            }
            _ => unreachable!()
        }
    }
}

impl AppData {
    #[inline]
    fn finished(&self) -> bool {
        matches!(
            self.state,
            ScreenCopyState::Failed | ScreenCopyState::Finished
        )
    }
    #[inline]
    fn ispedding(&self) -> bool {
        matches!(self.state, ScreenCopyState::Pedding)
    }

    pub fn capture_output_frame(
        &mut self,
        output: &WlOutput,
        event_queue: &mut EventQueue<Self>,
        (realwidth, realheight): (i32, i32),
        transform: wl_output::Transform,
        slurpoption: Option<(i32, i32, i32, i32)>,
    ) -> Option<FrameInfo> {
        let manager = self.wlr_screencopy.as_ref().unwrap();

        tracing::info!("windowinfo ==> width :{realwidth}, height: {realheight}");
        let qh = event_queue.handle();
        let frame = match slurpoption {
            None => manager.capture_output(0, output, &qh, ()),
            Some((x, y, width, height)) => {
                manager.capture_output_region(0, output, x, y, width, height, &qh, ())
            }
        };
        let mut frameformat = None;
        let mut frame_mmap = None;
        loop {
            event_queue.blocking_dispatch(self).unwrap();
            if self.finished() {
                break;
            }
            if self.ispedding() {
                frameformat = self
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
                let frame_format = frameformat.as_ref().unwrap();
                let frame_bytes = frame_format.stride * frame_format.height;
                let mut state_result = || {
                    let mem_fd = create_shm_fd()?;
                    let mem_file = unsafe { File::from_raw_fd(mem_fd) };
                    mem_file.set_len(frame_bytes as u64)?;

                    let shm_pool =
                        self.shm
                            .as_ref()
                            .unwrap()
                            .create_pool(mem_fd, frame_bytes as i32, &qh, ());

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
                    Ok::<(), Box<dyn Error>>(())
                };
                if let Err(e) = state_result() {
                    tracing::error!("Something error: {e}");
                    std::process::exit(1);
                }
            }
        }
        match self.state {
            ScreenCopyState::Finished => {
                self.formats.clear();
                self.state = ScreenCopyState::Staging;
                let output = FrameInfo {
                    frameformat: frameformat.unwrap(),
                    frame_mmap: frame_mmap.unwrap(),
                    transform,
                    realwidth: realwidth as u32,
                    realheight: realheight as u32,
                };
                Some(output)
            }
            ScreenCopyState::Failed => {
                tracing::error!("Cannot take screen copy");
                None
            }
            _ => unreachable!(),
        }
    }
}
