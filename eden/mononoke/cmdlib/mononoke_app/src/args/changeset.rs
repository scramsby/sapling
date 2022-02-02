/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

use anyhow::{Context, Result};
use bonsai_hg_mapping::BonsaiHgMappingRef;
use bookmarks::{BookmarkName, BookmarksRef};
use clap::{ArgGroup, Args};
use context::CoreContext;
use mercurial_types::HgChangesetId;
use mononoke_types::ChangesetId;
use repo_identity::RepoIdentityRef;

/// Command line arguments for specifying a changeset.
#[derive(Args, Debug)]
#[clap(group(
    ArgGroup::new("changeset")
        .required(true)
        .args(&["changeset-id", "hg-id", "bookmark"]),
))]
pub struct ChangesetArgs {
    /// Bonsai changeset id
    #[clap(long, short = 'i')]
    changeset_id: Option<ChangesetId>,

    /// Hg changeset id
    #[clap(long)]
    hg_id: Option<HgChangesetId>,

    /// Bookmark name
    #[clap(long, short = 'B')]
    bookmark: Option<BookmarkName>,
}

impl ChangesetArgs {
    pub async fn resolve_changeset(
        &self,
        ctx: &CoreContext,
        repo: &(impl BookmarksRef + BonsaiHgMappingRef + RepoIdentityRef),
    ) -> Result<Option<ChangesetId>> {
        if let Some(bookmark) = &self.bookmark {
            repo.bookmarks()
                .get(ctx.clone(), bookmark)
                .await
                .with_context(|| format!("Failed to resolve bookmark '{}'", bookmark))
        } else if let Some(hg_id) = self.hg_id {
            repo.bonsai_hg_mapping()
                .get_bonsai_from_hg(ctx, repo.repo_identity().id(), hg_id)
                .await
                .with_context(|| format!("Failed to resolve hg changeset id {}", hg_id))
        } else {
            Ok(self.changeset_id)
        }
    }
}
