#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum FramePixelFormat {
    Bgra32 = 1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum FrameType {
    Full = 1,
    Resize = 2,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrameSlot {
    pub sequence: u64,
    pub width: u32,
    pub height: u32,
    pub pixel_format: FramePixelFormat,
    pub frame_type: FrameType,
    pub pixels: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FramePublishResult {
    Published { slot_index: usize },
    Dropped,
}

impl FramePublishResult {
    pub fn published(self) -> bool {
        matches!(self, Self::Published { .. })
    }
}

pub struct FrameRingState {
    slots: Vec<Option<FrameSlot>>,
    slot_size: usize,
    next_slot: usize,
    latest_slot: Option<usize>,
    acknowledged_sequence: u64,
    submitted_count: u64,
    published_count: u64,
    dropped_count: u64,
}

impl FrameRingState {
    pub fn new(slot_count: usize, slot_size: usize) -> Self {
        let slot_count = slot_count.max(1);

        Self {
            slots: vec![None; slot_count],
            slot_size,
            next_slot: 0,
            latest_slot: None,
            acknowledged_sequence: 0,
            submitted_count: 0,
            published_count: 0,
            dropped_count: 0,
        }
    }

    pub fn publish_full(
        &mut self,
        sequence: u64,
        width: u32,
        height: u32,
        pixels: &[u8],
    ) -> FramePublishResult {
        self.submitted_count = self.submitted_count.saturating_add(1);

        if self.should_drop_same_size_waiting_for_ack(width, height) {
            self.dropped_count = self.dropped_count.saturating_add(1);
            return FramePublishResult::Dropped;
        }

        let frame_type = match self.latest_dimensions() {
            Some((latest_width, latest_height))
                if latest_width != width || latest_height != height =>
            {
                FrameType::Resize
            }
            _ => FrameType::Full,
        };
        let slot_index = self.next_slot;
        self.slots[slot_index] = Some(FrameSlot {
            sequence,
            width,
            height,
            pixel_format: FramePixelFormat::Bgra32,
            frame_type,
            pixels: pixels[..pixels.len().min(self.slot_size)].to_vec(),
        });
        self.latest_slot = Some(slot_index);
        self.next_slot = (self.next_slot + 1) % self.slots.len();
        self.published_count = self.published_count.saturating_add(1);

        FramePublishResult::Published { slot_index }
    }

    pub fn ack(&mut self, sequence: u64) {
        self.acknowledged_sequence = self.acknowledged_sequence.max(sequence);
    }

    pub fn latest_frame(&self) -> Option<&FrameSlot> {
        self.latest_slot
            .and_then(|slot_index| self.slots[slot_index].as_ref())
    }

    pub fn acknowledged_sequence(&self) -> u64 {
        self.acknowledged_sequence
    }

    pub fn submitted_count(&self) -> u64 {
        self.submitted_count
    }

    pub fn published_count(&self) -> u64 {
        self.published_count
    }

    pub fn dropped_count(&self) -> u64 {
        self.dropped_count
    }

    fn should_drop_same_size_waiting_for_ack(&self, width: u32, height: u32) -> bool {
        let Some(latest) = self.latest_frame() else {
            return false;
        };

        latest.width == width
            && latest.height == height
            && latest.sequence > self.acknowledged_sequence
    }

    fn latest_dimensions(&self) -> Option<(u32, u32)> {
        self.latest_frame().map(|frame| (frame.width, frame.height))
    }
}
