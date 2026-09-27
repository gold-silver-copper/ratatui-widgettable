# Records the demo video that `assets/record.sh` converts into `assets/demo.webp` for the README.
# Run it using that script rather than directly, as the script tells the tape where the project is.
#
# See https://www.joshka.net/betamax/ for more information about tape files.

Output target/demo.mp4

Require cargo

Set Shell "bash"
Set FontFamily "Menlo"
Set FontSize 16
# the demo redraws every 100ms, so there is nothing to gain from a higher frame rate
Set Framerate 10
Set Width 1400
Set Height 640
Set Padding 16
Set WindowBar Colorful
Set KeyboardOverlay Keys
Set TypingSpeed 100ms
Set WaitTimeout 120s

# Build and start the demo without recording
Hide
Type 'cd "${DEMO_DIR:?}" && cargo build --quiet --example demo && clear && ./target/debug/examples/demo'
Enter
Wait+Screen /widgettable/
Show

# Watch the running tasks make progress
Sleep 2.5s

# Select the next task, start it, and expand its row
Down
Sleep 1s
Space
Sleep 1.5s
Enter
Sleep 2s

# Move to the Progress column and bump the progress
Right 2
Sleep 1s
Type "+++"
Sleep 1.5s

# Sort by progress, then reverse the order
Type "s"
Sleep 1.5s
Type "s"
Sleep 2s

# Collapse the row and start another task
Enter
Sleep 1s
Down 2
Sleep 800ms
Space
Sleep 1.5s

# Add a task and delete it again
Type "a"
Sleep 1.5s
Type "d"
Sleep 1.5s

# Jump to the last and first tasks
Type "G"
Sleep 1.5s
Type "g"
Sleep 3s

Type "q"
