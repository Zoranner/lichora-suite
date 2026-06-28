use std::path::Path;

use anyhow::Result;
use log::warn;

const OUTPUT_LATEST_CAPACITY_BYTES: u32 = 16 * 1024;
const OUTPUT_QUEUE_ITEM_CAPACITY: u32 = 256;
const OUTPUT_QUEUE_MAX_PAYLOAD_LEN: u32 = 16 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BrowserOutputIpcSpec {
    pub latest: ipc::ChannelSpec,
    pub queue: ipc::MappedQueueSpec,
}

pub struct BrowserOutputIpcChannels {
    latest: ipc::ChannelMappedFile,
    queue: ipc::MappedSpscQueue,
    next_sequence: u64,
}

impl BrowserOutputIpcChannels {
    pub(crate) fn open_or_create(session_id: &str, browser_id: &str) -> Result<Self> {
        Self::open_or_create_in_dir(ipc_directory(), session_id, browser_id)
    }

    fn open_or_create_in_dir(
        directory: impl AsRef<Path>,
        session_id: &str,
        browser_id: &str,
    ) -> Result<Self> {
        let spec = browser_output_ipc_spec(session_id, browser_id);
        let directory = directory.as_ref();
        let latest = ipc::ChannelMappedFile::open_in_dir(
            directory,
            &spec.latest,
            ipc::ChannelOpenMode::OpenExisting,
        )
        .or_else(|_| {
            ipc::ChannelMappedFile::open_in_dir(
                directory,
                &spec.latest,
                ipc::ChannelOpenMode::Create,
            )
        })?;
        let queue = ipc::MappedSpscQueue::open_in_dir(
            directory,
            &spec.queue,
            ipc::ChannelOpenMode::OpenExisting,
        )
        .or_else(|_| {
            ipc::MappedSpscQueue::open_in_dir(directory, &spec.queue, ipc::ChannelOpenMode::Create)
        })?;

        Ok(Self {
            latest,
            queue,
            next_sequence: 0,
        })
    }

    pub(crate) fn publish(&mut self, payload: ipc::OutputPayload) -> Result<()> {
        self.next_sequence = self.next_sequence.saturating_add(1);
        let sequence = self.next_sequence;
        let kind = payload.kind() as u32;
        let bytes = payload.encode();

        self.latest.publish_latest(sequence, 0, &bytes)?;
        if let Err(error) = self
            .queue
            .try_push(ipc::MappedQueueItem::new(kind, sequence, &bytes))
        {
            warn!(
                "Failed to publish IPC v2 output queue item kind={} sequence={}: {}",
                kind, sequence, error
            );
        }
        Ok(())
    }
}

pub(crate) fn browser_output_ipc_spec(session_id: &str, browser_id: &str) -> BrowserOutputIpcSpec {
    let latest_name =
        ipc::build_browser_channel_name(session_id, browser_id, ipc::ChannelKind::Output);
    BrowserOutputIpcSpec {
        latest: ipc::ChannelSpec::new(
            latest_name.clone(),
            ipc::ChannelKind::Output,
            OUTPUT_LATEST_CAPACITY_BYTES,
        ),
        queue: ipc::MappedQueueSpec::new(
            format!("{latest_name}_queue"),
            ipc::ChannelKind::Output,
            OUTPUT_QUEUE_ITEM_CAPACITY,
            OUTPUT_QUEUE_MAX_PAYLOAD_LEN,
        ),
    }
}

fn ipc_directory() -> std::path::PathBuf {
    std::env::var_os("EBI_IPC_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("EmbeddedBrowserIpc"))
}

#[cfg(test)]
mod tests {
    use super::{
        browser_output_ipc_spec, BrowserOutputIpcChannels, OUTPUT_LATEST_CAPACITY_BYTES,
        OUTPUT_QUEUE_ITEM_CAPACITY, OUTPUT_QUEUE_MAX_PAYLOAD_LEN,
    };

    #[test]
    fn browser_output_ipc_spec_matches_ipc_native_names_and_layout() {
        let spec = browser_output_ipc_spec("session-42", "browser-A");

        assert_eq!(
            "EmbeddedBrowser_session-42_browser-A_output",
            spec.latest.name
        );
        assert_eq!(ipc::ChannelKind::Output, spec.latest.channel_kind);
        assert_eq!(OUTPUT_LATEST_CAPACITY_BYTES, spec.latest.capacity_bytes);
        assert_eq!(
            "EmbeddedBrowser_session-42_browser-A_output_queue",
            spec.queue.name
        );
        assert_eq!(ipc::ChannelKind::Output, spec.queue.channel_kind);
        assert_eq!(OUTPUT_QUEUE_ITEM_CAPACITY, spec.queue.item_capacity);
        assert_eq!(OUTPUT_QUEUE_MAX_PAYLOAD_LEN, spec.queue.max_payload_len);
    }

    #[test]
    fn publishes_output_payload_to_latest_and_queue() {
        let temp = tempfile::tempdir().unwrap();
        let session_id = format!("session-{}", uuid::Uuid::new_v4());
        let browser_id = format!("browser-{}", uuid::Uuid::new_v4());
        let mut channels =
            BrowserOutputIpcChannels::open_or_create_in_dir(temp.path(), &session_id, &browser_id)
                .expect("output channels");
        let payload = ipc::OutputPayload::Caret(ipc::CaretOutput {
            x: 10,
            y: 20,
            width: 0,
            height: 30,
            visible: true,
        });

        channels.publish(payload.clone()).unwrap();
        drop(channels);

        let spec = browser_output_ipc_spec(&session_id, &browser_id);
        let mut latest = ipc::ChannelMappedFile::open_in_dir(
            temp.path(),
            &spec.latest,
            ipc::ChannelOpenMode::OpenExisting,
        )
        .unwrap();
        let mut queue = ipc::MappedSpscQueue::open_in_dir(
            temp.path(),
            &spec.queue,
            ipc::ChannelOpenMode::OpenExisting,
        )
        .unwrap();
        let latest_payload =
            ipc::OutputPayload::decode(&latest.try_read_latest().unwrap().unwrap().payload)
                .unwrap();
        let queued = queue.try_pop().unwrap().unwrap();

        assert_eq!(payload, latest_payload);
        assert_eq!(ipc::OutputPayloadKind::Caret as u32, queued.kind);
        assert_eq!(
            payload,
            ipc::OutputPayload::decode(&queued.payload).unwrap()
        );
    }
}
