#![allow(clippy::unwrap_used, clippy::panic)]

use super::*;
use std::{
    cell::RefCell,
    future::Future,
    task::{Context, Poll, Waker},
};

fn run<T>(future: impl Future<Output = T>) -> T {
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    let mut future = std::pin::pin!(future);
    match future.as_mut().poll(&mut context) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("test adapter unexpectedly suspended"),
    }
}
fn target() -> AttachmentTarget {
    AttachmentTarget {
        actor: Uuid::now_v7(),
        ticket: Uuid::now_v7(),
        message: Uuid::now_v7(),
    }
}
fn policy() -> AttachmentPolicy {
    AttachmentPolicy {
        enabled: true,
        max_bytes: 32,
        max_pixels: 100,
        max_per_message: 1,
    }
}
fn image() -> ProcessedScreenshot {
    ProcessedScreenshot {
        bytes: vec![1, 2],
        width: 2,
        height: 2,
        format: ScreenshotFormat::Png,
    }
}
struct Pipeline {
    calls: RefCell<usize>,
    invalid: bool,
}
impl ScreenshotPipeline for Pipeline {
    async fn process(
        &self,
        source: &[u8],
        _: usize,
        _: u64,
    ) -> Result<ProcessedScreenshot, AttachmentError> {
        *self.calls.borrow_mut() += 1;
        if source != b"valid image" {
            return Err(AttachmentError::InvalidInput);
        }
        let mut image = image();
        if self.invalid {
            image.width = u32::MAX;
        }
        Ok(image)
    }
}
struct Store {
    owner: AttachmentTarget,
    saved: RefCell<Option<(Uuid, ProcessedScreenshot)>>,
    deny_commit: bool,
}
impl TicketAttachmentStore for Store {
    async fn authorize_append(
        &self,
        target: AttachmentTarget,
        _: usize,
    ) -> Result<(), AttachmentError> {
        if target != self.owner {
            return Err(AttachmentError::NotFound);
        }
        if self.saved.borrow().is_some() {
            return Err(AttachmentError::Conflict);
        }
        Ok(())
    }
    async fn append_private(
        &self,
        target: AttachmentTarget,
        image: ProcessedScreenshot,
        limit: usize,
    ) -> Result<Uuid, AttachmentError> {
        self.authorize_append(target, limit).await?;
        if self.deny_commit {
            return Err(AttachmentError::Conflict);
        }
        let id = Uuid::now_v7();
        *self.saved.borrow_mut() = Some((id, image));
        Ok(id)
    }
    async fn read_private(
        &self,
        target: AttachmentTarget,
        id: Uuid,
    ) -> Result<ProcessedScreenshot, AttachmentError> {
        if target != self.owner {
            return Err(AttachmentError::NotFound);
        }
        self.saved
            .borrow()
            .as_ref()
            .filter(|(saved, _)| *saved == id)
            .map(|(_, image)| image.clone())
            .ok_or(AttachmentError::NotFound)
    }
}
fn service(target: AttachmentTarget) -> Attachments<Pipeline, Store> {
    Attachments::new(
        policy(),
        Pipeline {
            calls: RefCell::new(0),
            invalid: false,
        },
        Store {
            owner: target,
            saved: RefCell::new(None),
            deny_commit: false,
        },
    )
}
#[test]
fn upload_then_private_download_and_limit() {
    let target = target();
    let service = service(target);
    let id = run(service.append(target, b"valid image")).unwrap();
    assert_eq!(run(service.read(target, id)).unwrap().bytes, vec![1, 2]);
    assert_eq!(
        run(service.append(target, b"valid image")),
        Err(AttachmentError::Conflict)
    );
    assert_eq!(*service.pipeline.calls.borrow(), 1);
    assert!(matches!(
        run(service.read(target, Uuid::now_v7())),
        Err(AttachmentError::NotFound)
    ));
}
#[test]
fn default_and_unknown_configuration_deny_without_processing() {
    let target = target();
    let mut service = service(target);
    service.policy = AttachmentPolicy::default();
    assert_eq!(
        run(service.append(target, b"valid image")),
        Err(AttachmentError::Disabled)
    );
    service.policy.enabled = true;
    assert_eq!(
        run(service.append(target, b"valid image")),
        Err(AttachmentError::Unavailable)
    );
    assert_eq!(*service.pipeline.calls.borrow(), 0);
}
#[test]
fn other_owner_message_and_ticket_are_hidden() {
    let target = target();
    let service = service(target);
    let id = run(service.append(target, b"valid image")).unwrap();
    for other in [
        AttachmentTarget {
            actor: Uuid::now_v7(),
            ..target
        },
        AttachmentTarget {
            message: Uuid::now_v7(),
            ..target
        },
        AttachmentTarget {
            ticket: Uuid::now_v7(),
            ..target
        },
    ] {
        assert_eq!(
            run(service.append(other, b"valid image")),
            Err(AttachmentError::NotFound)
        );
        assert!(matches!(
            run(service.read(other, id)),
            Err(AttachmentError::NotFound)
        ));
    }
    assert_eq!(*service.pipeline.calls.borrow(), 1);
}
#[test]
fn bad_input_and_invalid_processed_output_never_persist() {
    let target = target();
    let mut service = service(target);
    for bytes in [&b""[..], &b"not an image"[..], &[0; 33][..]] {
        assert_eq!(
            run(service.append(target, bytes)),
            Err(AttachmentError::InvalidInput)
        );
    }
    let invalid = AttachmentTarget {
        message: Uuid::nil(),
        ..target
    };
    assert_eq!(
        run(service.append(invalid, b"valid image")),
        Err(AttachmentError::InvalidInput)
    );
    assert!(matches!(
        run(service.read(target, Uuid::nil())),
        Err(AttachmentError::InvalidInput)
    ));
    service.pipeline.invalid = true;
    assert_eq!(
        run(service.append(target, b"valid image")),
        Err(AttachmentError::InvalidInput)
    );
    assert!(service.store.saved.borrow().is_none());
}
#[test]
fn revoked_authorization_at_commit_does_not_save() {
    let target = target();
    let mut service = service(target);
    service.store.deny_commit = true;
    assert_eq!(
        run(service.append(target, b"valid image")),
        Err(AttachmentError::Conflict)
    );
    assert!(service.store.saved.borrow().is_none());
}
