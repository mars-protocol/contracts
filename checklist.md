# Update Contracts - Cherry-pick from core-contracts

## Overview
Pulling latest changes from core-contracts by cherry-picking specific commits.

## Commits to Cherry-pick (in reverse chronological order)
- [x] `27e8a393232b3f4af2cb7fd89f32fb81dc025881` (oldest - start here) ✅ Conflicts resolved
- [x] `db65c00a347d13d81af1cf4b19776ab4b72eeb07` ✅ Conflicts resolved
- [x] `cb3ef67df47a7ef75d0c9fa1d0577bfc54c59e5d` ✅ Cherry-picked cleanly (2025-11-03)
- [ ] `b508ec06e5b90ed5200fecf4be06a95574c0f1c4` (newest - finish here)
  - ⏭️ Cherry-pick skipped (files removed locally); needs manual decision

## Progress Checklist
- [x] Check current git status and branch
  - Current branch: `latest-core-contracts`
  - Uncommitted changes in: Makefile.toml
- [x] Add core-contracts as remote (if not already present)
  - ✅ Already configured: `git@github.com:mars-protocol/core-contracts.git`
- [x] Fetch latest changes from core-contracts
  - ✅ Fetched successfully, found all target commits
- [ ] Cherry-pick commits in order
  - ✅ First commit (27e8a393...) conflicts resolved
  - ✅ Second commit (db65c00a...) conflicts resolved
  - ✅ Third commit (cb3ef67d...) conflicts resolved
  - ⚠️ Final commit (b508ec06...) skipped: neutron migration files absent in workspace
- [ ] Test and verify changes
- [ ] Document any conflicts or issues

## Conflicts to Resolve

**✅ Commit 27e8a393... (Add Spot trading fees) - RESOLVED**

**✅ Commit db65c00a... (Add whitelist for rewards distributors) - RESOLVED**

**✅ Commit cb3ef67d... (Add spot swap fee query) - RESOLVED**

**Previous conflicts (resolved):**
- `contracts/account-nft/tests/tests/helpers/mock_env_builder.rs`
- `contracts/credit-manager/src/instantiate.rs`
- `contracts/credit-manager/src/query.rs`
- `contracts/credit-manager/src/state.rs`
- `contracts/credit-manager/src/swap.rs`
- `contracts/credit-manager/src/update_config.rs`
- `contracts/credit-manager/src/utils.rs`
- `contracts/credit-manager/tests/tests/test_update_config.rs`
- `contracts/health/tests/tests/helpers/mock_env_builder.rs`
- `packages/testing/src/multitest/helpers/mock_env.rs`
- `packages/types/src/credit_manager/instantiate.rs`
- `packages/types/src/credit_manager/query.rs`
- `scripts/deploy/base/deployer.ts`
- `scripts/deploy/neutron/devnet-config.ts`
- `scripts/deploy/neutron/mainnet-config.ts`
- `scripts/deploy/neutron/testnet-config.ts`
- `scripts/deploy/osmosis/mainnet-config.ts`
- `scripts/deploy/osmosis/testnet-config.ts`
- `scripts/types/config.ts`

## Notes
- Started: 2025-10-01T14:59:29+08:00
- ✅ First commit (27e8a393...) completed with conflicts resolved
- ✅ Second commit (db65c00a...) completed with conflicts resolved
- ✅ Third commit (cb3ef67d...) completed with conflicts resolved (2025-11-03)
- ⚠️ Cherry-pick b508ec06... skipped (neutron migrations deleted upstream here)
