//! Private ticket screenshots. T33 and T12 are not present in the base yet;
//! these ports isolate their future adapters without introducing a second upload pipeline.
//! No HTTP route is mounted until authenticated sessions, CSRF and throttling are available.

use uuid::Uuid;

/// Deliberately disabled unless deployment configuration explicitly opts in.
#[derive(Debug, Default, Clone, Copy)]
pub struct AttachmentPolicy {
    pub enabled: bool,
    pub max_bytes: usize,
    pub max_pixels: u64,
    pub max_per_message: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentError {
    Disabled,
    InvalidInput,
    NotFound,
    Conflict,
    Unavailable,
}

/// Values come from authenticated server context, never request ownership fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttachmentTarget {
    pub actor: Uuid,
    pub ticket: Uuid,
    pub message: Uuid,
}

impl AttachmentTarget {
    fn valid(self) -> bool {
        [self.actor, self.ticket, self.message]
            .iter()
            .all(|id| id.get_version_num() == 7 && id.get_variant() == uuid::Variant::RFC4122)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenshotFormat {
    Png,
    Jpeg,
    Webp,
}

/// Only normalized, decoded and re-encoded pixels may cross the upload port.
/// Original filenames, paths, claimed MIME types and original bytes are not persisted.
#[derive(Clone)]
pub struct ProcessedScreenshot {
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub format: ScreenshotFormat,
}

impl std::fmt::Debug for ProcessedScreenshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProcessedScreenshot")
            .field("byte_size", &self.bytes.len())
            .field("width", &self.width)
            .field("height", &self.height)
            .field("format", &self.format)
            .finish()
    }
}

/// T12 adapter contract: enforce byte/pixel bounds before allocation, detect type by
/// content, fully decode, reject malformed/trailing payloads, strip metadata and
/// re-encode PNG/JPEG/WebP. Never return an original or publicly stored object.
#[allow(async_fn_in_trait)]
pub trait ScreenshotPipeline {
    async fn process(
        &self,
        source: &[u8],
        max_bytes: usize,
        max_pixels: u64,
    ) -> Result<ProcessedScreenshot, AttachmentError>;
}

/// T33 persistence adapter. Unknown ownership/status/visibility must deny.
#[allow(async_fn_in_trait)]
pub trait TicketAttachmentStore {
    /// Require an active account, owned ticket accepting replies, and a public
    /// message belonging to that ticket and authored by the actor. Hide other
    /// owners and internal messages with NotFound. Apply the per-message limit.
    async fn authorize_append(
        &self,
        target: AttachmentTarget,
        limit: usize,
    ) -> Result<(), AttachmentError>;

    /// Recheck every authorization condition and the count under a ticket/message
    /// lock, then persist private normalized bytes and their attachment relation
    /// atomically. Rollback must leave no reachable blob. Allocate a UUIDv7 ID;
    /// do not accept caller storage paths or public media references.
    async fn append_private(
        &self,
        target: AttachmentTarget,
        image: ProcessedScreenshot,
        limit: usize,
    ) -> Result<Uuid, AttachmentError>;

    /// Reauthorize the current active owner and public message on every read,
    /// including thumbnails; verify attachment -> message -> ticket membership.
    /// Return NotFound for inaccessible, deleted or unknown records. Never return
    /// a public URL. HTTP adapters must send private, no-store and nosniff headers.
    async fn read_private(
        &self,
        target: AttachmentTarget,
        attachment: Uuid,
    ) -> Result<ProcessedScreenshot, AttachmentError>;
}

#[derive(Debug)]
pub struct Attachments<P, S> {
    policy: AttachmentPolicy,
    pipeline: P,
    store: S,
}

impl<P: ScreenshotPipeline, S: TicketAttachmentStore> Attachments<P, S> {
    pub fn new(policy: AttachmentPolicy, pipeline: P, store: S) -> Self {
        Self {
            policy,
            pipeline,
            store,
        }
    }

    fn check(&self, target: AttachmentTarget) -> Result<(), AttachmentError> {
        if !self.policy.enabled {
            return Err(AttachmentError::Disabled);
        }
        if self.policy.max_bytes == 0
            || self.policy.max_pixels == 0
            || self.policy.max_per_message == 0
        {
            return Err(AttachmentError::Unavailable);
        }
        if !target.valid() {
            return Err(AttachmentError::InvalidInput);
        }
        Ok(())
    }

    fn check_image(&self, image: &ProcessedScreenshot) -> Result<(), AttachmentError> {
        if image.bytes.is_empty()
            || image.bytes.len() > self.policy.max_bytes
            || image.width == 0
            || image.height == 0
            || u64::from(image.width) * u64::from(image.height) > self.policy.max_pixels
        {
            return Err(AttachmentError::InvalidInput);
        }
        Ok(())
    }

    pub async fn append(
        &self,
        target: AttachmentTarget,
        source: &[u8],
    ) -> Result<Uuid, AttachmentError> {
        self.check(target)?;
        self.store
            .authorize_append(target, self.policy.max_per_message)
            .await?;
        if source.is_empty() || source.len() > self.policy.max_bytes {
            return Err(AttachmentError::InvalidInput);
        }
        let image = self
            .pipeline
            .process(source, self.policy.max_bytes, self.policy.max_pixels)
            .await?;
        self.check_image(&image)?;
        self.store
            .append_private(target, image, self.policy.max_per_message)
            .await
    }

    pub async fn read(
        &self,
        target: AttachmentTarget,
        attachment: Uuid,
    ) -> Result<ProcessedScreenshot, AttachmentError> {
        self.check(target)?;
        if attachment.get_version_num() != 7 || attachment.get_variant() != uuid::Variant::RFC4122 {
            return Err(AttachmentError::InvalidInput);
        }
        let image = self.store.read_private(target, attachment).await?;
        self.check_image(&image)?;
        Ok(image)
    }
}

#[cfg(test)]
mod tests;
