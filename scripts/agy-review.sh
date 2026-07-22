#!/usr/bin/env bash
#
# agy-review.sh -- headless GitHub PR reviewer driven by Antigravity CLI (`agy`).
#
# Runs on a SELF-HOSTED GitHub Actions runner that lives on a machine where `agy`
# is already logged in via Google OAuth. Because it uses the CLI's cached OAuth
# session (not an API key), every review is billed against your Google AI Ultra
# rate limits -- i.e. free under the subscription, no metered API spend.
#
# Flow: resolve PR -> `gh pr diff` -> build an adversarial-reviewer prompt
#       (+ repo style guide) -> `agy --print` under a PTY -> post via `gh pr comment`.
#
# See ../README.md for setup, the issue #76 PTY workaround, and the ToS caveat.
set -euo pipefail

# --- configuration (all env-overridable from the workflow) ---------------------
AGY_BIN="${AGY_BIN:-agy}"
command -v "$AGY_BIN" >/dev/null 2>&1 || AGY_BIN="$HOME/.local/bin/agy"
AGY_MODEL="${AGY_MODEL:-}"                 # empty = agy's configured default (Gemini 3.x Pro)
AGY_EFFORT="${AGY_EFFORT:-high}"           # low|medium|high
AGY_PRINT_TIMEOUT="${AGY_PRINT_TIMEOUT:-5m}"
MAX_DIFF_BYTES="${MAX_DIFF_BYTES:-115000}" # truncate very large diffs (keep prompt under the arg limit)
MAX_PROMPT_BYTES="${MAX_PROMPT_BYTES:-125000}" # hard ceiling on the whole prompt: agy takes it as a
                                           # --print VALUE (not stdin), and a single execve argument
                                           # cannot exceed MAX_ARG_STRLEN (PAGE_SIZE*32 = 128 KiB on
                                           # Linux). Over it, execve fails with E2BIG before agy even
                                           # starts -- a deterministic failure the retry loop cannot
                                           # clear. This backstops MAX_DIFF_BYTES + boilerplate + style.
ARG_SIZE_CEILING=128000                     # hard cap: a configured MAX_PROMPT_BYTES above the
                                           # MAX_ARG_STRLEN-derived safe bound would defeat the guard
                                           # and re-expose E2BIG, so clamp any override down to it.
if ! [ "$MAX_PROMPT_BYTES" -le "$ARG_SIZE_CEILING" ] 2>/dev/null; then
  log "MAX_PROMPT_BYTES='${MAX_PROMPT_BYTES}' invalid or above the ${ARG_SIZE_CEILING}-byte ceiling; clamping"
  MAX_PROMPT_BYTES="$ARG_SIZE_CEILING"
fi
STYLE_GUIDE="${STYLE_GUIDE:-.github/agy-review.md}"  # repo-relative; loaded if present
                                           # (dedicated name -- avoids colliding with GEMINI.md/AGENTS.md)
CONV_DIR="${CONV_DIR:-$HOME/.gemini/antigravity-cli/conversations}"
LOG="${RUNNER_TEMP:-/tmp}/agy-review.log"
AGY_LOCK="${AGY_LOCK:-$HOME/.gemini/antigravity-cli/.agy-review.lock}"
AGY_LOCK_WAIT="${AGY_LOCK_WAIT:-600}"      # seconds to wait for the agy lock before proceeding
AGY_RETRIES="${AGY_RETRIES:-3}"            # attempts to get a usable agy response
AGY_RETRY_DELAY="${AGY_RETRY_DELAY:-15}"   # base backoff seconds between retries (grows per attempt)
MARKER="<!-- antigravity-pr-review -->"

log() { printf '[agy-review] %s\n' "$*" >&2; }
have_text() { [ -s "$1" ] && grep -q '[^[:space:]]' "$1"; }

REPO="${GITHUB_REPOSITORY:?GITHUB_REPOSITORY not set}"

# --- resolve the PR number from the triggering event --------------------------
case "${GITHUB_EVENT_NAME:-}" in
  pull_request|pull_request_target)
    PR="$(jq -r '.pull_request.number' "$GITHUB_EVENT_PATH")"
    ;;
  issue_comment)
    is_pr="$(jq -r '.issue.pull_request // empty' "$GITHUB_EVENT_PATH")"
    body="$(jq -r '.comment.body // ""' "$GITHUB_EVENT_PATH")"
    [ -n "$is_pr" ] || { log "comment is not on a PR; skipping"; exit 0; }
    case "$body" in
      /agy-review*) : ;;
      *) log "comment is not an /agy-review command; skipping"; exit 0 ;;
    esac
    PR="$(jq -r '.issue.number' "$GITHUB_EVENT_PATH")"
    ;;
  *)
    PR="${1:-}"
    [ -n "$PR" ] || { log "unknown event; pass a PR number as \$1"; exit 1; }
    ;;
esac
log "reviewing ${REPO}#${PR}"

# Remove every temp file on exit. Pre-declared so the trap is safe under `set -u` even if the
# script exits before a given file is created.
diff_file= meta_file= prompt_file= out_file= raw= body_file=
trap 'rm -f "$diff_file" "$meta_file" "$prompt_file" "$out_file" "$raw" "$body_file"' EXIT

# --- fetch the diff + metadata -------------------------------------------------
diff_file="$(mktemp)"; meta_file="$(mktemp)"
gh pr diff "$PR" --repo "$REPO" > "$diff_file" || { log "gh pr diff failed"; exit 1; }
gh pr view "$PR" --repo "$REPO" --json title > "$meta_file" 2>/dev/null || echo '{}' > "$meta_file"

if ! have_text "$diff_file"; then log "empty diff; nothing to review"; exit 0; fi

truncated=""
if [ "$(wc -c < "$diff_file")" -gt "$MAX_DIFF_BYTES" ]; then
  head -c "$MAX_DIFF_BYTES" "$diff_file" > "$diff_file.cut" && mv "$diff_file.cut" "$diff_file"
  truncated=$'\n\n> Note: the diff was truncated to '"${MAX_DIFF_BYTES}"$' bytes for this review.'
  log "diff truncated to ${MAX_DIFF_BYTES} bytes"
fi

# --- build the prompt ----------------------------------------------------------
title="$(jq -r '.title // ""' "$meta_file")"
style=""; [ -f "$STYLE_GUIDE" ] && style="$(cat "$STYLE_GUIDE")"

prompt_file="$(mktemp)"
{
  cat <<EOF
You are an adversarial code reviewer doing a first-pass review of a GitHub pull request.
Act as a skeptical senior engineer, not the author. Be concise, specific, and honest.

Output (GitHub-flavored Markdown, no preamble):
1. A one-sentence summary of what the PR does.
2. "### Blocking issues" -- correctness, security, data-loss, or breaking-change
   problems only. Write "None found." if there are none.
3. "### Suggestions" -- non-blocking improvements; cite file and line where you can.
4. "### Nitpicks" -- optional, keep terse.
Do not praise. Focus on what could be wrong. If the change is trivial, say so briefly.

PR title: ${title}
EOF
  if [ -n "$style" ]; then
    printf '\n--- PROJECT STYLE GUIDE (enforce these) ---\n%s\n' "$style"
  fi
  printf '\n--- UNIFIED DIFF ---\n'
  cat "$diff_file"
} > "$prompt_file"

# --- guard the argv size (E2BIG) -----------------------------------------------
# agy takes the prompt as a --print VALUE, so the whole prompt is one execve argument
# and must stay under MAX_ARG_STRLEN (128 KiB). MAX_DIFF_BYTES bounds the diff, but the
# boilerplate + style guide ride on top, so cap the assembled prompt as a hard backstop.
if [ "$(wc -c < "$prompt_file")" -gt "$MAX_PROMPT_BYTES" ]; then
  head -c "$MAX_PROMPT_BYTES" "$prompt_file" > "$prompt_file.cut" && mv "$prompt_file.cut" "$prompt_file"
  truncated+=$'\n\n> Note: the review prompt was capped to '"${MAX_PROMPT_BYTES}"$' bytes (execve arg-size limit).'
  log "prompt capped to ${MAX_PROMPT_BYTES} bytes (execve arg-size ceiling)"
fi
# Byte truncation (here or in the MAX_DIFF_BYTES cap above) can slice a multi-byte UTF-8 sequence.
# agy is a Rust binary and std::env::args() PANICS on a non-UTF-8 argument, which would reintroduce
# an instant startup failure -- exactly the class of bug this guard exists to prevent. Drop any
# invalid/partial sequences when iconv is available (glibc + macOS ship it); a no-op when clean.
if command -v iconv >/dev/null 2>&1; then
  # Explicit branches, not `&& mv || rm`: that idiom masks an mv failure and would then feed agy the
  # original (possibly split) bytes. A successful iconv must replace the file; a failed mv is fatal
  # (set -e), a failed iconv leaves the original and we proceed (it may already be clean, and any
  # residual invalid byte now surfaces via the captured stderr rather than a silent instant crash).
  if iconv -c -f UTF-8 -t UTF-8 "$prompt_file" > "$prompt_file.utf8" 2>/dev/null; then
    mv "$prompt_file.utf8" "$prompt_file"
  else
    rm -f "$prompt_file.utf8"
  fi
fi

# --- run agy headless, under a PTY (works around agy issue #76: -p drops --------
#     stdout when stdout is not a TTY, e.g. piped/redirected/subprocess) ---------
flags=( --print-timeout "$AGY_PRINT_TIMEOUT" --sandbox --dangerously-skip-permissions )
[ -n "$AGY_MODEL" ]  && flags+=( --model "$AGY_MODEL" )
[ -n "$AGY_EFFORT" ] && flags+=( --effort "$AGY_EFFORT" )

out_file="$(mktemp)"
here="$(cd "$(dirname "$0")" && pwd)"
: > "$LOG"

# Serialize agy across concurrent review jobs on this host. agy runs a SINGLETON
# local language-server + conversation store per user, so two `--print` calls at
# once collide (one reports the backend "unavailable"). flock makes jobs queue
# instead of failing. Best-effort: if the lock can't be taken, proceed anyway.
if command -v flock >/dev/null 2>&1; then
  # Create the lock dir first: a failed `exec 9>` redirection is a FATAL shell error (it aborts
  # before the `|| log` fallback can run), so ensure the parent exists on a fresh runner. `>>` opens
  # for append rather than truncating the lockfile — flock uses the fd, not the contents.
  mkdir -p "$(dirname "$AGY_LOCK")" 2>/dev/null || true
  exec 9>>"$AGY_LOCK" 2>/dev/null \
    && flock -w "$AGY_LOCK_WAIT" 9 \
    || log "agy lock unavailable or timed out (${AGY_LOCK_WAIT}s); proceeding unserialized"
fi

# Retry the whole agy attempt on empty/failed output: transient "agy is down"
# (backend rate-limit / local-server contention) usually clears within seconds.
# The flock (above) is held across all attempts, released after the loop.
for (( attempt=1; attempt<=AGY_RETRIES; attempt++ )); do
  : > "$out_file"   # clear any partial output from a prior attempt

  if command -v unbuffer >/dev/null 2>&1; then
    log "running agy via unbuffer (allocates a PTY) [attempt ${attempt}/${AGY_RETRIES}]"
    unbuffer "$AGY_BIN" "${flags[@]}" --print "$(cat "$prompt_file")" > "$out_file" 2>>"$LOG" || true
  else
    log "unbuffer not found; falling back to script(1) [attempt ${attempt}/${AGY_RETRIES}]"
    raw="$(mktemp)"
    # `script -c` runs its command through `sh -c`, so every path in the command string is quoted for
    # that inner shell: `'$here'` and `'$prompt_file'` are wrapped in single quotes (the outer double
    # quotes still expand them) so a repo path containing spaces survives the word-split.
    AGY_BIN="$AGY_BIN" script -qfec "'$here'/_agy_print.sh '$prompt_file' ${flags[*]}" "$raw" >/dev/null 2>>"$LOG" || true
    col -b < "$raw" > "$out_file"
    rm -f "$raw"   # each retry makes a fresh $raw; the EXIT trap only holds the last one
  fi

  # normalize CRs without sed -i (avoid in-place edit footguns)
  tr -d '\r' < "$out_file" > "$out_file.clean" && mv "$out_file.clean" "$out_file"

  # --- fallback: recover the answer from agy's conversation SQLite store --------
  #     (belt-and-suspenders for issue #76 on hosts where the PTY trick still
  #     yields nothing). The schema is NOT officially documented and can change
  #     between agy versions -- inspect with `sqlite3 <db> .schema` and adjust.
  if ! have_text "$out_file"; then
    log "print output empty; trying SQLite conversation fallback"
    if command -v sqlite3 >/dev/null 2>&1 && [ -d "$CONV_DIR" ]; then
      db="$(ls -t "$CONV_DIR"/*.db 2>/dev/null | head -1 || true)"
      if [ -n "${db:-}" ]; then
        for q in \
          "SELECT text FROM messages WHERE role='assistant' ORDER BY rowid DESC LIMIT 1;" \
          "SELECT content FROM messages WHERE role='assistant' ORDER BY rowid DESC LIMIT 1;" \
          "SELECT body FROM message WHERE role='assistant' ORDER BY rowid DESC LIMIT 1;"; do
          sqlite3 "$db" "$q" > "$out_file" 2>/dev/null && have_text "$out_file" && break
        done
      fi
    fi
  fi

  have_text "$out_file" && break
  if [ "$attempt" -lt "$AGY_RETRIES" ]; then
    delay=$(( AGY_RETRY_DELAY * attempt ))
    log "no usable output (attempt ${attempt}/${AGY_RETRIES}); retrying in ${delay}s"
    sleep "$delay"
  fi
done
exec 9>&- 2>/dev/null || true    # release the agy lock so the next queued job proceeds

if ! have_text "$out_file"; then
  log "no review output after ${AGY_RETRIES} attempt(s). Check $LOG and confirm 'agy -p \"hi\"' works for this user."
  # Surface agy's stderr into the job log. RUNNER_TEMP is wiped between jobs, so a bare
  # `exit 1` otherwise leaves the real cause invisible in CI (E2BIG, auth, backend, ...).
  if [ -s "$LOG" ]; then
    # Bound the dump to the last lines: GitHub Actions auto-masks registered secrets (incl.
    # GITHUB_TOKEN) in logs, and this is agy's own diagnostic stream, but a bounded tail avoids
    # publishing an unbounded volume of stderr (which could echo prompt/diff content) into CI.
    log "----- captured agy stderr ($LOG), last ${AGY_LOG_TAIL_LINES:-60} lines (secrets auto-masked) -----"
    tail -n "${AGY_LOG_TAIL_LINES:-60}" "$LOG" | sed 's/^/[agy] /' >&2 || true
    log "----- end agy stderr -----"
  else
    log "(agy stderr log is empty -- agy likely failed before writing, e.g. execve E2BIG on an oversized prompt)"
  fi
  exit 1
fi

# --- assemble the comment body -------------------------------------------------
body_file="$(mktemp)"
{
  printf '%s\n' "$MARKER"
  printf '## Antigravity review (Gemini via Ultra)\n\n'
  cat "$out_file"
  printf '%s' "$truncated"
  printf '\n\n<sub>Automated first-pass review by `agy` on a self-hosted runner -- not a human review.</sub>\n'
} > "$body_file"

# --- replace any prior review comment, then post fresh -------------------------
# A failed delete is logged, not swallowed: silently ignoring it would let a transient API/perms
# error leave the old comment in place AND post a new one, so runs accumulate duplicates.
gh api "repos/${REPO}/issues/${PR}/comments" --paginate \
    --jq ".[] | select(.body | contains(\"${MARKER}\")) | .id" 2>/dev/null \
  | while read -r cid; do
      [ -n "$cid" ] || continue
      if ! gh api -X DELETE "repos/${REPO}/issues/comments/${cid}" >/dev/null 2>&1; then
        log "warning: could not delete prior review comment ${cid}; a duplicate may result"
      fi
    done

gh pr comment "$PR" --repo "$REPO" --body-file "$body_file"
log "posted review to ${REPO}#${PR}"
