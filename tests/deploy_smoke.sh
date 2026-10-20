#!/usr/bin/env bash
# Exercise deployment sequencing and rollback with a simulated Docker daemon.
set -euo pipefail
project="$(cd "$(dirname "$0")/.." && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/bin" "$work/app/deploy"
cp "$project/deploy/remote.sh" "$work/app/deploy/remote.sh"
printf '{}\n' > "$work/app/config.json"
cat > "$work/bin/docker" <<'DOCKER'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >> "$SIM_STATE/commands"
case "$1" in
  inspect)
    if [ "${2:-}" = -f ]; then
      case "$3" in
        *State.Running*) if [ "$SIM_FAIL" = yes ]; then echo false; else echo true; fi ;;
        *RestartCount*) echo 0 ;;
        *compose.project*) : ;;
        *) echo test-image ;;
      esac
    else
      test -f "$SIM_STATE/active"
    fi
    ;;
  stop) printf 'stopped\n' > "$SIM_STATE/active" ;;
  rename)
    if [ "$2" = sili-bot ]; then
      mv "$SIM_STATE/active" "$SIM_STATE/rollback"
    else
      mv "$SIM_STATE/rollback" "$SIM_STATE/active"
    fi
    ;;
  compose)
    case "$2" in
      --env-file)
        if [ "$4" = up ]; then printf 'rust\n' > "$SIM_STATE/active"; fi
        if [ "$4" = run ]; then
          case "$*" in *"--entrypoint tar"*) tar -czf - --files-from=/dev/null ;; esac
          cat >/dev/null
        fi
        ;;
    esac
    ;;
  rm) rm -f "$SIM_STATE/active" ;;
  start) printf 'legacy-running\n' > "$SIM_STATE/active" ;;
  exec) echo 'Configuration is valid' ;;
  logs) echo 'Sili bot started' ;;
  *) exit 1 ;;
esac
DOCKER
chmod +x "$work/bin/docker"
printf '#!/usr/bin/env bash\nexit 0\n' > "$work/bin/sleep"
chmod +x "$work/bin/sleep"
export PATH="$work/bin:$PATH"
for mode in no yes; do
  export SIM_STATE="$work/state-$mode" SIM_FAIL="$mode"
  mkdir "$SIM_STATE"
  printf 'legacy-running\n' > "$SIM_STATE/active"
  : > "$work/app/.deploy.env"
  if bash "$work/app/deploy/remote.sh" </dev/null > "$work/result-$mode" 2>&1; then
    test "$mode" = no
    grep -q '^Rust bot running; backup snapshot:' "$work/result-$mode"
    grep -q '^rust$' "$SIM_STATE/active"
    test -f "$SIM_STATE/rollback"
    gzip -t "$work/app"/backups/dynamic-*.tar.gz
    grep -q -- '--cap-add CHOWN --cap-add DAC_OVERRIDE --entrypoint chown' "$SIM_STATE/commands"
    grep -q 'exec sili-bot sili-telegram-bot --check-config' "$SIM_STATE/commands"
  else
    test "$mode" = yes
    grep -q '^legacy-running$' "$SIM_STATE/active"
    grep -q 'restoring the previous container' "$work/result-$mode"
  fi
done
printf 'Deployment smoke tests passed: full cutover and failed-start rollback\n'
