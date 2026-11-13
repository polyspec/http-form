#!/bin/sh
# Runs a command with HTTP_FORM_OTHER_FS, a directory on another file system than the temporary directory, which the
# test of a move to another file system needs (H-9).
#
#   scripts/other-fs.sh <command> [<argument> ...]
#
# On Linux the directory is /dev/shm, a tmpfs. On macOS the script makes a disk image of APFS in a directory of the
# run, attaches it there and detaches and removes it when the command ends or fails, and when the run receives SIGINT
# or SIGTERM, which it passes on to the process group of the command. hdiutil returns when the image is attached or
# detached, so the run waits for no time.
set -u
if [ "$#" -lt 1 ]; then
	echo "usage: scripts/other-fs.sh <command> [<argument> ...]" >&2
	exit 2
fi
case $(uname -s) in
Linux)
	HTTP_FORM_OTHER_FS=/dev/shm exec "$@"
	;;
Darwin) ;;
*)
	echo "scripts/other-fs.sh: $(uname -s) is no platform of the library" >&2
	exit 2
	;;
esac
directory=$(mktemp -d) || exit 1
mount=$directory/mount
attached=
remove() {
	if [ -n "$attached" ]; then
		hdiutil detach -quiet "$mount" || echo "the disk image at $mount is not detached" >&2
	fi
	rm -rf "$directory"
}
mkdir "$mount" || { remove; exit 1; }
hdiutil create -quiet -size 16m -fs APFS -volname http-form-other -layout NONE "$directory/other.dmg" || { remove; exit 1; }
hdiutil attach -quiet -nobrowse -noverify -mountpoint "$mount" "$directory/other.dmg" || { remove; exit 1; }
attached=yes
# Job control gives the command a process group of its own, so that a signal reaches every process of the command.
set -m
HTTP_FORM_OTHER_FS=$mount "$@" &
command=$!
stop() {
	kill "-$1" "-$command" 2>/dev/null
	wait "$command"
	remove
	exit "$2"
}
trap 'stop INT 130' INT
trap 'stop TERM 143' TERM
wait "$command"
status=$?
remove
exit "$status"
