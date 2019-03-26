/*
 *  Copyright (c) 2019-present, Facebook, Inc.
 *  All rights reserved.
 *
 *  This source code is licensed under the BSD-style license found in the
 *  LICENSE file in the root directory of this source tree. An additional grant
 *  of patent rights can be found in the PATENTS file in the same directory.
 *
 */
#include "eden/fs/store/mononoke/MononokeThriftBackingStore.h"

#include <folly/logging/xlog.h>
#include <scm/mononoke/apiserver/gen-cpp2/MononokeAPIServiceAsyncClient.h>
#include <scm/mononoke/apiserver/gen-cpp2/apiserver_types.h>
#include <servicerouter/client/cpp2/ServiceRouter.h>

#include "eden/fs/model/Blob.h"
#include "eden/fs/model/Hash.h"
#include "eden/fs/model/Tree.h"

using scm::mononoke::apiserver::thrift::MononokeAPIServiceAsyncClient;
using scm::mononoke::apiserver::thrift::MononokeBlob;
using scm::mononoke::apiserver::thrift::MononokeChangeset;
using scm::mononoke::apiserver::thrift::MononokeDirectory;
using scm::mononoke::apiserver::thrift::MononokeFileType;
using scm::mononoke::apiserver::thrift::MononokeGetBlobParams;
using scm::mononoke::apiserver::thrift::MononokeGetChangesetParams;
using scm::mononoke::apiserver::thrift::MononokeGetTreeParams;
using scm::mononoke::apiserver::thrift::MononokeNodeHash;
using scm::mononoke::apiserver::thrift::MononokeRevision;
using scm::mononoke::apiserver::thrift::MononokeTreeHash;

namespace facebook {
namespace eden {
namespace {
TreeEntryType treeEntryTypeFromMononoke(MononokeFileType type) {
  switch (type) {
    case MononokeFileType::FILE:
      return TreeEntryType::REGULAR_FILE;
    case MononokeFileType::TREE:
      return TreeEntryType::TREE;
    case MononokeFileType::EXECUTABLE:
      return TreeEntryType::EXECUTABLE_FILE;
    case MononokeFileType::SYMLINK:
      return TreeEntryType::SYMLINK;
  }

  XLOG(WARNING) << "Unexpected Mononoke file type: " << static_cast<int>(type);
  return TreeEntryType::REGULAR_FILE;
}
} // namespace

MononokeThriftBackingStore::MononokeThriftBackingStore(
    std::string serviceName,
    std::string repo,
    std::shared_ptr<folly::Executor> executor)
    : MononokeThriftBackingStore(
          /*client=*/servicerouter::cpp2::getClientFactory()
              .getClientUnique<MononokeAPIServiceAsyncClient>(serviceName),
          /*repo=*/std::move(repo),
          /*executor=*/std::move(executor)) {}

MononokeThriftBackingStore::MononokeThriftBackingStore(
    std::unique_ptr<MononokeAPIServiceAsyncClient> client,
    std::string repo,
    std::shared_ptr<folly::Executor> executor)
    : repo_(std::move(repo)),
      client_(std::move(client)),
      executor_(std::move(executor)) {}

MononokeThriftBackingStore::~MononokeThriftBackingStore() {}

folly::Future<std::unique_ptr<Tree>> MononokeThriftBackingStore::getTree(
    const Hash& id) {
  const auto& treeHashString = id.toString();

  XLOG(DBG6) << "importing tree '" << treeHashString << "' from mononoke";
  MononokeTreeHash treeHash;
  treeHash.set_hash(treeHashString);

  MononokeGetTreeParams params;
  params.set_repo(repo_);
  params.set_tree_hash(treeHash);

  return client_->semifuture_get_tree(params)
      .via(executor_.get())
      .thenValue([id](const MononokeDirectory&& response) {
        auto& files = response.get_files();

        std::vector<TreeEntry> entries;
        entries.reserve(files.size());

        for (const auto& file : files) {
          if (file.__isset.content_sha1 && file.__isset.size) {
            entries.emplace_back(
                Hash(file.hash.hash),
                file.name,
                treeEntryTypeFromMononoke(file.file_type),
                file.size,
                Hash(file.content_sha1));
          } else {
            entries.emplace_back(
                Hash(file.hash.hash),
                file.name,
                treeEntryTypeFromMononoke(file.file_type));
          }
        }

        return std::make_unique<Tree>(std::move(entries), id);
      });
}
folly::Future<std::unique_ptr<Blob>> MononokeThriftBackingStore::getBlob(
    const Hash& id) {
  const auto& blobHashString = id.toString();

  XLOG(DBG6) << "importing blob '" << blobHashString << "' from mononoke";
  MononokeNodeHash blobHash;
  blobHash.set_hash(blobHashString);

  MononokeGetBlobParams params;
  params.set_repo(repo_);
  params.set_blob_hash(blobHash);

  return client_->semifuture_get_blob(params)
      .via(executor_.get())
      .thenValue([id](const MononokeBlob&& response) {
        return std::make_unique<Blob>(id, std::move(*response.get_content()));
      });
}
folly::Future<std::unique_ptr<Tree>>
MononokeThriftBackingStore::getTreeForCommit(const Hash& commitID) {
  const auto& commitIdString = commitID.toString();

  XLOG(DBG6) << "importing commit '" << commitIdString << "' from mononoke";
  MononokeRevision rev;
  rev.set_commit_hash(commitIdString);

  MononokeGetChangesetParams params;
  params.set_repo(repo_);
  params.set_revision(rev);

  return client_->semifuture_get_changeset(params)
      .via(executor_.get())
      .thenValue([this](const MononokeChangeset&& response) {
        return getTree(Hash(response.get_manifest().get_hash()));
      });
}
} // namespace eden
} // namespace facebook
