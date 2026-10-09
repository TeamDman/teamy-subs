# Command completion budgets

Every leaf explicitly returns an advisory `elapsed_duration_warn_threshold`. The entrypoint measures invocation and rendering, warns only when elapsed is strictly greater than the budget, and preserves errors/cancellation. Budget metadata is separate from JSON/CSV/text reports. This is a completion warning, not a watchdog. Request counts, byte sizes and work counts below express expected work, never observed elapsed.

- Home/cache show and internal SubDL cleanup/status: 1 second for local path/state inspection.
- Home/cache open: 3 seconds for detached folder opening.
- Cache clean: 1 second plus 250 ms per removed top-level cache directory (an advisory deletion expectation, not a filesystem bound).
- Convert: 1 second plus 1 second per whole MiB of input for parse/transform/write.
- Extract: 5 seconds plus 1 second per subtitle stream times (one plus whole 10 MiB chunks in the source container); each ffmpeg extraction scans that input. Existing kill-on-drop/cancellation remains intact.
- SubDL logout: 1 second plus 1 second per removed lease/task.
- SubDL account/title/subtitle search: 5 seconds setup/decoding plus 30 seconds for one bounded HTTP request. Subtitle download: the same setup plus two requests, 65 seconds total; transport/archive size limits remain intact.
- SubDL commands using an explicit op reference return `None`: 1Password approval is user interactive and has no deadline. The client records this execution mode so leaf receipts retain it; cached/environment-key commands retain finite request budgets. Login always uses that approval path and returns `None`.
- yt-dlp download returns `None`: an arbitrary URL can expand to an unbounded playlist and the default language selector includes all languages. No playlist/media count bound is provided by this command.

Tests cover below/equal/above warnings and explicit exemptions, saturating work scaling, report JSON/CSV separation, authentication mode/request budgets, and a synthetic local conversion. No service requests are needed for these new tests.