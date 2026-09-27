#!/bin/sh
# A `git` first on a test child's PATH (30c): the kill seam for the
# set-aside put-back. No fail-point lives in production code; this stands in
# for the real git on one argv and passes every other call through.
#
#   GRAFT_SHIM_REAL_GIT  the real git, exec'd for every other call
#   GRAFT_SHIM_ON        plain-apply: `stash apply` without `--index`
#                        drop:        `stash drop`
#   GRAFT_SHIM_DO        kill: SIGKILL the parent (the process running
#                              `apply`), then exit without running git
#                        fail: exit 1 with a line on stderr, git not run
#
# Shared by graft-ops (`tests/it/set_aside_kill.rs`) and graft
# (`tests/it/m6_stash_put_back/set_aside.rs`), both through `include_str!`.

args=" $* "
hit=0
case "$GRAFT_SHIM_ON" in
plain-apply)
    case "$args" in
    *" stash apply --index "*) ;;
    *" stash apply "*) hit=1 ;;
    esac
    ;;
drop)
    case "$args" in
    *" stash drop "*) hit=1 ;;
    esac
    ;;
esac

if [ "$hit" = 1 ]; then
    case "$GRAFT_SHIM_DO" in
    kill)
        kill -9 "$PPID"
        exit 1
        ;;
    fail)
        echo "error: the test shim refuses this $GRAFT_SHIM_ON" >&2
        exit 1
        ;;
    esac
fi
exec "$GRAFT_SHIM_REAL_GIT" "$@"
