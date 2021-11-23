#!/bin/bash

set -e

main() {
  if [[ -z $LOCAL_TERRA_REPO_PATH ]]; then
    echo "LOCAL_TERRA_REPO_PATH must be set"
    return 1
  fi

  tests=$(ls tests/*.ts | \
    # test_helpers is not a test file
    grep -v test_helpers | \
    # oracle tests must be run with slower block times (see below)
    grep -v oracle | \
    # MWEs for broken tests
    grep -v red_bank_deposit_bug_mwe | \
    # broken tests due to the `addr_validate errored: Input is empty` bug
    grep -v repay_ust | grep -v liquidations
  )
  echo $tests

  # ensure LocalTerra is stopped
  docker compose -f $LOCAL_TERRA_REPO_PATH/docker-compose.yml down

  # start LocalTerra
  sed -E -i .bak '/timeout_(propose|prevote|precommit|commit)/s/[0-9]+m?s/200ms/' $LOCAL_TERRA_REPO_PATH/config/config.toml
  docker compose -f $LOCAL_TERRA_REPO_PATH/docker-compose.yml up -d

  # run tests
  for test in $tests; do
    echo Running $test
    node --loader ts-node/esm $test
  done

  docker compose -f $LOCAL_TERRA_REPO_PATH/docker-compose.yml down

  # oracle tests
  sed -E -i .bak '/timeout_(propose|prevote|precommit|commit)/s/[0-9]+m?s/1500ms/' $LOCAL_TERRA_REPO_PATH/config/config.toml
  docker compose -f $LOCAL_TERRA_REPO_PATH/docker-compose.yml up -d

  echo Running tests/oracle.ts
  node --loader ts-node/esm tests/oracle.ts

  docker compose -f $LOCAL_TERRA_REPO_PATH/docker-compose.yml down
  sed -E -i .bak '/timeout_(propose|prevote|precommit|commit)/s/[0-9]+m?s/200ms/' $LOCAL_TERRA_REPO_PATH/config/config.toml
}

main
