//! Native-owned, single-slot review lifecycle. Not an IPC or parser entry point.
//! Callers must derive the owner from native window state, never renderer input,
//! and serialize access on a blocking worker. No review data is persisted here.

use std::time::{Duration, Instant};

use ort_documents::import::ImportProposal;
use ort_domain::{SaveResumePayload, VersionedResumeResponse};
use uuid::Uuid;

use crate::import_review::{ImportReview, ReviewDecision, ReviewError};

pub const REVIEW_LIFETIME: Duration = Duration::from_mins(30);

/// Created by native window/session setup; not deserializable from IPC.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ReviewOwner(Uuid);
impl Default for ReviewOwner {
    fn default() -> Self {
        Self(Uuid::now_v7())
    }
}

/// Opaque identity, separate from the authoritative native owner.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ReviewToken(Uuid);
impl ReviewToken {
    /// Parses a renderer-visible identity only, never native owner authority.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        if value.len() != 36 {
            return None;
        }
        ort_domain::EntityId::parse(value)
            .ok()
            .map(|id| Self(id.as_uuid()))
    }

    #[must_use]
    pub fn identifier(self) -> String {
        self.0.to_string()
    }
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum SessionError {
    #[error("a review is already active")]
    Busy,
    #[error("the review is unavailable or has expired")]
    Unavailable,
    #[error(transparent)]
    Review(#[from] ReviewError),
}

#[derive(Debug, PartialEq, Eq)]
pub enum CommitError<E> {
    Session(SessionError),
    Storage(E),
    /// Storage reported success with an unexpected result. Retire the review:
    /// the operation may already have committed and must not be replayed.
    UnexpectedReceipt,
}

struct ActiveReview {
    owner: ReviewOwner,
    token: ReviewToken,
    deadline: Instant,
    review: ImportReview,
}

#[derive(Default)]
pub struct ReviewSessions {
    active: Option<ActiveReview>,
}

impl ReviewSessions {
    /// Checks the single review slot after applying its fixed expiry.
    pub fn is_active(&mut self, now: Instant) -> bool {
        self.expire(now);
        self.active.is_some()
    }

    /// Starts from a parent-validated proposal only after native containment and
    /// cleanup gates pass. This method itself does not prove those gates.
    ///
    /// # Errors
    /// Refuses replacement of an active review, invalid drafts or clock overflow.
    pub fn begin(
        &mut self,
        owner: ReviewOwner,
        base: VersionedResumeResponse,
        proposal: ImportProposal,
        now: Instant,
    ) -> Result<ReviewToken, SessionError> {
        self.expire(now);
        if self.active.is_some() {
            return Err(SessionError::Busy);
        }
        let deadline = now
            .checked_add(REVIEW_LIFETIME)
            .ok_or(SessionError::Unavailable)?;
        let review = ImportReview::new(base, proposal)?;
        let token = ReviewToken(Uuid::now_v7());
        self.active = Some(ActiveReview {
            owner,
            token,
            deadline,
            review,
        });
        Ok(token)
    }

    fn authorized(
        &mut self,
        owner: ReviewOwner,
        token: ReviewToken,
        now: Instant,
    ) -> Result<&mut ActiveReview, SessionError> {
        self.expire(now);
        self.active
            .as_mut()
            .filter(|active| active.owner == owner && active.token == token)
            .ok_or(SessionError::Unavailable)
    }

    /// Returns retained source and decisions without extending the deadline.
    ///
    /// # Errors
    /// Refuses expired, foreign or retired identities.
    pub fn read(
        &mut self,
        owner: ReviewOwner,
        token: ReviewToken,
        now: Instant,
    ) -> Result<&ImportReview, SessionError> {
        Ok(&self.authorized(owner, token, now)?.review)
    }

    /// Changes one explicit decision; None returns it to pending.
    ///
    /// # Errors
    /// Refuses foreign/expired identities and invalid item indices or limits.
    pub fn decide(
        &mut self,
        owner: ReviewOwner,
        token: ReviewToken,
        now: Instant,
        index: usize,
        decision: Option<ReviewDecision>,
    ) -> Result<(), SessionError> {
        let review = &mut self.authorized(owner, token, now)?.review;
        match decision {
            Some(decision) => review.decide(index, decision)?,
            None => review.reset_decision(index)?,
        }
        Ok(())
    }

    /// Replaces all decisions only for the native owner's active review.
    ///
    /// # Errors
    /// Rejects foreign/expired sessions, block-count mismatch and content limits
    /// without changing any existing decision.
    pub fn replace_choices(
        &mut self,
        owner: ReviewOwner,
        token: ReviewToken,
        now: Instant,
        choices: ort_domain::ImportChoices,
    ) -> Result<(), SessionError> {
        self.authorized(owner, token, now)?
            .review
            .replace_choices(choices)?;
        Ok(())
    }

    /// Validates against the freshly loaded saved draft, then invokes exactly
    /// one trusted storage CAS. The callback must use `payload.expected_revision`
    /// and return the actual saved revision/document, never a renderer receipt.
    /// Mutable ownership excludes edits/cancel/another commit during the call.
    ///
    /// # Errors
    /// Failed validation never calls storage. Storage errors retain the review
    /// for inspection; a later retry still requires fresh draft validation/CAS.
    pub fn commit<E>(
        &mut self,
        owner: ReviewOwner,
        token: ReviewToken,
        now: Instant,
        current: &VersionedResumeResponse,
        save: impl FnOnce(&SaveResumePayload) -> Result<VersionedResumeResponse, E>,
    ) -> Result<VersionedResumeResponse, CommitError<E>> {
        let active = self
            .authorized(owner, token, now)
            .map_err(CommitError::Session)?;
        let candidate = active
            .review
            .prepare(current)
            .map_err(|error| CommitError::Session(SessionError::Review(error)))?;
        let saved = save(&candidate).map_err(CommitError::Storage)?;
        self.active = None;
        if current.revision.checked_add(1) != Some(saved.revision)
            || saved.document != candidate.document
        {
            return Err(CommitError::UnexpectedReceipt);
        }
        Ok(saved)
    }

    /// Cancels the exact active review, or confirms the slot is already empty.
    ///
    /// # Errors
    /// Foreign/stale tokens cannot cancel the current review.
    pub fn cancel(
        &mut self,
        owner: ReviewOwner,
        token: ReviewToken,
        now: Instant,
    ) -> Result<(), SessionError> {
        self.expire(now);
        if self.active.is_some() {
            self.authorized(owner, token, now)?;
            self.active = None;
        }
        Ok(())
    }

    /// Native window teardown drops its source and decisions. No secure-erasure
    /// claim is made for allocator copies of document text.
    pub fn close_owner(&mut self, owner: ReviewOwner) {
        if self
            .active
            .as_ref()
            .is_some_and(|active| active.owner == owner)
        {
            self.active = None;
        }
    }

    /// Invoke on the native cleanup timer as well as command access. Idle expiry
    /// is not automatic: the eventual adapter must schedule this method.
    pub fn expire(&mut self, now: Instant) {
        if self
            .active
            .as_ref()
            .is_some_and(|active| now >= active.deadline)
        {
            self.active = None;
        }
    }
}
