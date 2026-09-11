# `Ctrl+Z` is undo, not suspend

In a terminal `Ctrl+Z` is `SIGTSTP` and suspends the process, so a terminal app
that binds it to undo is doing something unusual. We bind it to undo anyway,
because the undo journal is Graft's headline safety feature and the muscle
memory for undo is universal, while suspending a Git client is rare and still
available through `Ctrl+\` and the job-control keys the shell already provides.
Setting `keys.suspend_on_ctrl_z = true` restores the default behaviour.

## Consequences

Graft must handle suspend correctly for the escape-hatch path *and* for
`SIGTSTP` arriving from elsewhere: restore the terminal on suspend and
re-initialise on resume, or the eighth non-negotiable principle ("leave the
terminal clean") is broken by a keystroke.
