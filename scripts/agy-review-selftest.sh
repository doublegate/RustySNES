#!/usr/bin/env bash
#
# agy-review-selftest.sh -- guards the comment-selection logic in `agy-review.sh`.
#
# Why this exists: that filter decides which PR comments the bot DELETES, and it has been wrong
# twice, both times invisibly.
#
#   1. The just-posted comment was not reliably excluded. `new_comment_id` came from re-querying
#      the comment list, which races GitHub's read replication; on a miss the exclusion became
#      `select(.id != null)`, true for every id, and the run deleted the review it had just
#      published.
#   2. jq's `--arg`/`--argjson` were handed to `gh api`, which has no such flags. It exited
#      non-zero, `2>/dev/null` hid the message, and `set -o pipefail` + `set -e` killed the script
#      AFTER posting — so stale comments silently accumulated and the job went red with nothing in
#      the log explaining why.
#
# Neither was catchable by looking at the review the bot posted: both times it posted fine. So the
# filter is tested here directly, offline, against fixtures — no network, no `gh`, no runner.
#
# Run: bash scripts/agy-review-selftest.sh

set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

# Source the constants out of the reviewer without running it. `agy-review.sh` does its work at
# top level, so it cannot simply be sourced; the two values under test are lifted by pattern
# instead. That coupling is deliberate: if either declaration is renamed or reshaped, this test
# fails loudly rather than silently checking a stale copy of the filter.
extract_marker() {
  sed -n 's/^MARKER="\(.*\)"$/\1/p' "$SCRIPT_DIR/agy-review.sh" | head -n 1
}
extract_filter() {
  sed -n "/^SELECT_STALE_JQ='/,/'\$/p" "$SCRIPT_DIR/agy-review.sh" \
    | sed "1s/^SELECT_STALE_JQ='//; \$s/'\$//"
}

MARKER="$(extract_marker)"
FILTER="$(extract_filter)"

[ -n "$MARKER" ] || { echo "FAIL: could not extract MARKER from agy-review.sh" >&2; exit 1; }
[ -n "$FILTER" ] || { echo "FAIL: could not extract SELECT_STALE_JQ from agy-review.sh" >&2; exit 1; }

fixture() {
  cat <<JSON
[
  {"id": 111, "user": {"type": "Bot",  "login": "github-actions[bot]"}, "body": "$MARKER\nold review"},
  {"id": 222, "user": {"type": "Bot",  "login": "github-actions[bot]"}, "body": "$MARKER\nolder still"},
  {"id": 333, "user": {"type": "User", "login": "someone"},             "body": "$MARKER\nnot ours"},
  {"id": 444, "user": {"type": "Bot",  "login": "other-bot"},           "body": "$MARKER\nwrong bot"},
  {"id": 555, "user": {"type": "Bot",  "login": "github-actions[bot]"}, "body": "an ordinary bot comment"},
  {"id": 999, "user": {"type": "Bot",  "login": "github-actions[bot]"}, "body": "$MARKER\nJUST POSTED"}
]
JSON
}

select_ids() {
  fixture | jq -r --arg marker "$MARKER" --argjson new_id "$1" "$FILTER" | sort -n | tr '\n' ' '
}

fails=0
check() {
  local name="$1" want="$2" got="$3"
  if [ "$got" = "$want" ]; then
    echo "  ok    $name"
  else
    echo "  FAIL  $name"
    echo "          want: [$want]"
    echo "          got:  [$got]"
    fails=$((fails + 1))
  fi
}

echo "agy-review comment-selection self-test"

# The whole point: the comment just published is never selected for deletion.
check "excludes the just-posted comment" "111 222 " "$(select_ids 999)"

# The author filter is a security control, not tidiness: without it any user could paste the
# marker into a comment and have the bot delete comments on the next run.
check "ignores other users and other bots" "111 222 " "$(select_ids 999)"

# A bot comment without the marker is somebody else's feature (a CI summary, a deploy note).
check "ignores bot comments without the marker" "111 222 " "$(select_ids 999)"

# Regression #1, pinned: an unknown id must not select everything. The caller now refuses to run
# the delete at all in this case, but the filter itself is checked so the two guards are
# independent rather than one relying on the other.
check "an id of 0 still excludes nothing real" "111 222 999 " "$(select_ids 0)"

# A different id in the set behaves the same way, so the exclusion is genuinely by value.
check "excludes whichever id it is given" "222 999 " "$(select_ids 111)"

# Regression #2, pinned: `--arg`/`--argjson` belong to jq. If they are ever moved onto `gh api`
# again, that command exits non-zero — assert the flags are not passed to `gh api` in the script.
if grep -nE 'gh api[^|]*--(arg|argjson)' "$SCRIPT_DIR/agy-review.sh" >/dev/null 2>&1; then
  echo "  FAIL  --arg/--argjson passed to \`gh api\` (jq flags; gh api rejects them)"
  fails=$((fails + 1))
else
  echo "  ok    --arg/--argjson are not passed to \`gh api\`"
fi

if [ "$fails" -ne 0 ]; then
  echo "$fails check(s) failed" >&2
  exit 1
fi
echo "all checks passed"
