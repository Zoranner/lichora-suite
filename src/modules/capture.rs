//! Frame publication through IPC v2 frame channels.

use anyhow::{ensure, Result};

const FRAME_SLOT_COUNT: u32 = 2;
const FRAME_SLOT_SIZE: u32 = 64 * 1024 * 1024;
const MAX_WIDTH: usize = 2560;
const MAX_HEIGHT: usize = 1440;
const BYTES_PER_PIXEL: usize = 4;
const MAX_PIXEL_BUFFER_SIZE: usize = MAX_WIDTH * MAX_HEIGHT * BYTES_PER_PIXEL;

pub struct CaptureModule {
    frame_channel: ipc::FrameChannel,
    frame_sequence: u64,
    width: i32,
    height: i32,
}

impl CaptureModule {
    pub fn new_frame_channel_only(
        session_id: &str,
        browser_id: &str,
        width: i32,
        height: i32,
    ) -> Result<Self> {
        let frame_channel = open_or_create_frame_channel(session_id, browser_id)?;
        Ok(Self {
            frame_channel,
            frame_sequence: 0,
            width,
            height,
        })
    }

    pub fn write_paint_frame(
        &mut self,
        width: i32,
        height: i32,
        pixels: &[u8],
        _dirty_rects: &[(i32, i32, i32, i32)],
    ) -> Result<bool> {
        ensure!(width > 0 && height > 0, "invalid capture size");
        let pixel_data_size = width as usize * height as usize * BYTES_PER_PIXEL;
        ensure!(
            pixel_data_size == pixels.len(),
            "pixel buffer length does not match capture size"
        );
        ensure!(
            pixel_data_size <= MAX_PIXEL_BUFFER_SIZE,
            "pixel buffer exceeds capture slot size"
        );

        self.frame_sequence = self.frame_sequence.saturating_add(1);
        let published = self
            .frame_channel
            .publish_full(self.frame_sequence, width as u32, height as u32, pixels)?
            .published();
        if published {
            self.width = width;
            self.height = height;
        }
        Ok(published)
    }

    #[allow(dead_code)]
    pub fn size(&self) -> (i32, i32) {
        (self.width, self.height)
    }
}

fn open_or_create_frame_channel(session_id: &str, browser_id: &str) -> Result<ipc::FrameChannel> {
    let directory = ipc_directory();
    let name = ipc::build_browser_channel_name(session_id, browser_id, ipc::ChannelKind::Frame);
    let spec = ipc::FrameChannelSpec::new(name, FRAME_SLOT_COUNT, FRAME_SLOT_SIZE);
    ipc::FrameChannel::open_in_dir(&directory, &spec, ipc::ChannelOpenMode::OpenExisting)
        .or_else(|_| {
            ipc::FrameChannel::open_in_dir(&directory, &spec, ipc::ChannelOpenMode::Create)
        })
        .map_err(Into::into)
}

fn ipc_directory() -> std::path::PathBuf {
    std::env::var_os("EBI_IPC_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("EmbeddedBrowserIpc"))
}

#[cfg(test)]
mod tests {
    use super::CaptureModule;

    #[test]
    fn publishes_frame_channel_without_legacy_capture_file() {
        let temp = tempfile::tempdir().unwrap();
        std::env::set_var("EBI_IPC_DIR", temp.path());
        let session_id = uuid::Uuid::new_v4().to_string();
        let browser_id = uuid::Uuid::new_v4().to_string();
        let mut module =
            CaptureModule::new_frame_channel_only(&session_id, &browser_id, 2, 2).unwrap();
        let pixels = vec![9u8; 2 * 2 * 4];

        assert!(module.write_paint_frame(2, 2, &pixels, &[]).unwrap());

        let name =
            ipc::build_browser_channel_name(&session_id, &browser_id, ipc::ChannelKind::Frame);
        let spec = ipc::FrameChannelSpec::new(name, 2, 64 * 1024 * 1024);
        let mut reader =
            ipc::FrameChannel::open_in_dir(temp.path(), &spec, ipc::ChannelOpenMode::OpenExisting)
                .unwrap();
        let mut buffer = vec![0u8; pixels.len()];
        let copied = reader.try_copy_latest(&mut buffer).unwrap();

        assert_eq!(1, copied.sequence);
        assert_eq!(2, copied.width);
        assert_eq!(2, copied.height);
        assert_eq!(pixels, buffer);
        std::env::remove_var("EBI_IPC_DIR");
    }
}
