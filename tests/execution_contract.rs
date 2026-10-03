//! Black-box checks for the process boundary. A private `git` executable in
//! each test's PATH makes every byte and process lifetime observable without
//! depending on the installed Git or its configuration.

#![cfg(unix)]

use std::ffi::OsString;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use git_spawn::{
    CancellationToken, CommandExecutor, Error, ExecutionFailureKind, GitCommand, OutputLimits,
    OutputStream, ProcessStatus, VersionCommand,
};
use tokio::io::AsyncWriteExt;

const FIXTURE: &str = r#"#!/bin/sh
case "$1" in
  --version)
    printf 'version|%s' "${GIT_SPAWN_TEST_OVERRIDE-unset}"
    ;;
  bytes)
    printf '\377\000abc'
    printf '\376\000err' >&2
    exit "$2"
    ;;
  signal)
    kill -TERM "$$"
    ;;
  input)
    /bin/cat
    ;;
  close-input)
    exit 0
    ;;
  environment)
    printf '%s|%s' "${GIT_SPAWN_TEST_OVERRIDE-unset}" "${GIT_SPAWN_TEST_INHERITED-unset}"
    ;;
  secret)
    printf '%s' "$GIT_SPAWN_TEST_SECRET"
    printf '%s' "$GIT_SPAWN_TEST_SECRET" >&2
    exit 7
    ;;
  side-effect)
    printf ran > "$GIT_SPAWN_TEST_EFFECT"
    ;;
  finite-output)
    n=0
    while [ "$n" -lt 8192 ]; do
      printf '0123456789abcdef'
      printf 'fedcba9876543210' >&2
      n=$((n + 1))
    done
    ;;
  flood)
    printf '%s' "$$" > "$GIT_SPAWN_TEST_PARENT_PID"
    n=0
    while [ "$n" -lt 100000 ]; do
      printf '0123456789abcdef'
      printf 'fedcba9876543210' >&2
      n=$((n + 1))
    done
    ;;
  block)
    printf '%s' "$$" > "$GIT_SPAWN_TEST_PARENT_PID"
    printf 'out\000' 
    printf 'err\377' >&2
    printf ready > "$GIT_SPAWN_TEST_READY"
    while :; do /bin/sleep 1; done
    ;;
  no-read)
    printf '%s' "$$" > "$GIT_SPAWN_TEST_PARENT_PID"
    printf ready > "$GIT_SPAWN_TEST_READY"
    while :; do /bin/sleep 1; done
    ;;
  descendant)
    (
      while [ ! -e "$GIT_SPAWN_TEST_RELEASE" ]; do /bin/sleep 0.02; done
      printf escaped > "$GIT_SPAWN_TEST_EFFECT"
    ) &
    printf '%s' "$!" > "$GIT_SPAWN_TEST_DESCENDANT_PID"
    printf '%s' "$$" > "$GIT_SPAWN_TEST_PARENT_PID"
    printf ready > "$GIT_SPAWN_TEST_READY"
    wait
    ;;
  orphaned-pipe)
    (
      while [ ! -e "$GIT_SPAWN_TEST_RELEASE" ]; do /bin/sleep 0.02; done
      printf escaped > "$GIT_SPAWN_TEST_EFFECT"
    ) &
    printf '%s' "$!" > "$GIT_SPAWN_TEST_DESCENDANT_PID"
    printf '%s' "$$" > "$GIT_SPAWN_TEST_PARENT_PID"
    printf ready > "$GIT_SPAWN_TEST_READY"
    exit 0
    ;;
  escaped-pipe)
    printf '%s' "$$" > "$GIT_SPAWN_TEST_PARENT_PID"
    "$GIT_SPAWN_HARNESS_EXE" --exact escaped_pipe_child_harness --ignored --nocapture &
    wait
    ;;
  *)
    printf 'unknown fixture mode: %s' "$1" >&2
    exit 99
    ;;
esac
"#;

struct Fixture {
    dir: tempfile::TempDir,
    bin: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("bin");
        std::fs::create_dir(&bin).unwrap();
        let git = bin.join("git");
        std::fs::write(&git, FIXTURE).unwrap();
        std::fs::set_permissions(&git, std::fs::Permissions::from_mode(0o755)).unwrap();
        Self { dir, bin }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    fn executor(&self) -> CommandExecutor {
        CommandExecutor::new()
            .cwd(self.dir.path())
            .with_env("PATH", self.bin.as_os_str())
            .with_env("GIT_SPAWN_TEST_READY", self.path("ready"))
            .with_env("GIT_SPAWN_TEST_RELEASE", self.path("release"))
            .with_env("GIT_SPAWN_TEST_EFFECT", self.path("effect"))
            .with_env("GIT_SPAWN_TEST_PARENT_PID", self.path("parent.pid"))
            .with_env("GIT_SPAWN_TEST_DESCENDANT_PID", self.path("descendant.pid"))
    }

    async fn ready(&self) {
        wait_for_file(&self.path("ready")).await;
    }
}

async fn wait_for_file(path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !path.exists() {
        assert!(
            Instant::now() < deadline,
            "fixture never wrote {}",
            path.display()
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

fn failure(error: Error) -> Box<git_spawn::ExecutionFailure> {
    match error {
        Error::Execution { failure } => failure,
        other => panic!("expected structured execution failure, got {other:?}"),
    }
}

#[tokio::test]
async fn raw_bytes_and_nonzero_exit_are_preserved_in_unchecked_and_checked_paths() {
    let fixture = Fixture::new();
    let args = vec![OsString::from("bytes"), OsString::from("7")];
    let output = fixture
        .executor()
        .execute_command_os_unchecked(args.clone())
        .await
        .unwrap();
    assert_eq!(output.stdout, b"\xff\0abc");
    assert_eq!(output.stderr, b"\xfe\0err");
    assert_eq!(output.status, ProcessStatus::Exited { code: 7 });
    assert_eq!(output.exit_code, 7);
    assert!(!output.success);

    let error = fixture
        .executor()
        .execute_command_os(args)
        .await
        .unwrap_err();
    let Error::CommandFailed {
        exit_code,
        status,
        stdout,
        stderr,
        ..
    } = error
    else {
        panic!("expected checked status failure");
    };
    assert_eq!(exit_code, 7);
    assert_eq!(status, ProcessStatus::Exited { code: 7 });
    assert_eq!(stdout, b"\xff\0abc");
    assert_eq!(stderr, b"\xfe\0err");
}

#[tokio::test]
async fn spawn_failure_is_distinct_from_child_execution_failure() {
    let fixture = Fixture::new();
    let empty_bin = fixture.path("empty-bin");
    std::fs::create_dir(&empty_bin).unwrap();
    let err = CommandExecutor::new()
        .with_env("PATH", empty_bin)
        .execute_command_os_unchecked(vec!["side-effect".into()])
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Io { .. }));
    assert!(!fixture.path("effect").exists());
}

#[tokio::test]
async fn nonexistent_cwd_is_reported_as_spawn_io_failure() {
    let fixture = Fixture::new();
    let missing_cwd = fixture.path("no-such-directory");
    let err = fixture
        .executor()
        .cwd(&missing_cwd)
        .execute_command_os_unchecked(vec!["side-effect".into()])
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Io { .. }));
    assert!(!missing_cwd.exists());
    assert!(!fixture.path("effect").exists());
}

#[tokio::test]
async fn cancellation_before_spawn_has_no_child_side_effect() {
    let fixture = Fixture::new();
    let token = CancellationToken::new();
    token.cancel();
    let err = fixture
        .executor()
        .cancellation_token(token)
        .execute_command_os_unchecked(vec!["side-effect".into()])
        .await
        .unwrap_err();
    let failure = failure(err);
    assert!(matches!(failure.kind, ExecutionFailureKind::Cancelled));
    assert!(failure.status.is_none());
    assert!(!failure.cleanup.direct_child_reaped);
    assert!(!fixture.path("effect").exists());
}

#[tokio::test]
async fn git_command_trait_forwards_execution_configuration() {
    let fixture = Fixture::new();
    let mut command = VersionCommand::new();
    command
        .current_dir(fixture.dir.path())
        .env("PATH", fixture.bin.as_os_str())
        .env("GIT_SPAWN_TEST_OVERRIDE", "temporary")
        .env_remove("GIT_SPAWN_TEST_OVERRIDE")
        .stdin_null()
        .cancellation_token(CancellationToken::new())
        .with_timeout(Duration::from_secs(5))
        .cleanup_timeout(Duration::from_secs(1))
        .output_limits(OutputLimits {
            stdout: Some(64),
            stderr: Some(64),
        });
    let output = command.execute_raw_unchecked().await.unwrap();
    assert_eq!(output.stdout, b"version|unset");
}

#[tokio::test]
async fn signal_termination_is_distinct_from_an_exit_code() {
    let fixture = Fixture::new();
    let output = fixture
        .executor()
        .execute_command_os_unchecked(vec!["signal".into()])
        .await
        .unwrap();
    assert_eq!(
        output.status,
        ProcessStatus::Signaled {
            signal: libc::SIGTERM,
            core_dumped: false,
        }
    );
    assert!(!output.success);
}

#[tokio::test]
async fn explicit_stdin_modes_and_early_reader_close_are_classified() {
    let fixture = Fixture::new();
    let input = b"a\0\xff\n".to_vec();
    let echoed = fixture
        .executor()
        .stdin_bytes(input.clone())
        .execute_command_os(vec!["input".into()])
        .await
        .unwrap();
    assert_eq!(echoed.stdout, input);

    let empty = fixture
        .executor()
        .stdin_null()
        .execute_command_os(vec!["input".into()])
        .await
        .unwrap();
    assert!(empty.stdout.is_empty());

    let default_eof = fixture
        .executor()
        .execute_command_os(vec!["input".into()])
        .await
        .unwrap();
    assert!(default_eof.stdout.is_empty());

    // More than a pipe's capacity: the child exits without reading it. The
    // caller can tell its promised input was not fully delivered.
    let err = tokio::time::timeout(
        Duration::from_secs(5),
        fixture
            .executor()
            .stdin_bytes(vec![42; 2 * 1024 * 1024])
            .execute_command_os(vec!["close-input".into()]),
    )
    .await
    .expect("stdin writer stalled after the reader exited")
    .unwrap_err();
    let failure = failure(err);
    assert!(matches!(failure.kind, ExecutionFailureKind::Input { .. }));
    assert!(failure.cleanup.direct_child_reaped);
}

struct EventWriter(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for EventWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[tokio::test(flavor = "current_thread")]
async fn trace_and_error_formatting_hide_argv_environment_and_output() {
    let fixture = Fixture::new();
    let sentinel = "secret-token-7b14e8f3";
    let events = Arc::new(Mutex::new(Vec::new()));
    let writer_events = Arc::clone(&events);
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_ansi(false)
        .with_writer(move || EventWriter(Arc::clone(&writer_events)))
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);
    let executor = fixture
        .executor()
        .with_env("GIT_SPAWN_TEST_SECRET", sentinel);
    let err = executor
        .execute_command_os(vec!["secret".into(), sentinel.into()])
        .await
        .unwrap_err();
    let Error::CommandFailed {
        command,
        stdout,
        stderr,
        ..
    } = &err
    else {
        panic!("expected checked exit failure: {err:?}");
    };
    assert!(command.contains(sentinel));
    assert_eq!(stdout, sentinel.as_bytes());
    assert_eq!(stderr, sentinel.as_bytes());
    for formatted in [
        format!("{executor:?}"),
        format!("{err:?}"),
        format!("{err}"),
        String::from_utf8(events.lock().unwrap().clone()).unwrap(),
    ] {
        assert!(
            !formatted.contains(sentinel),
            "sensitive data in {formatted:?}"
        );
    }
}

#[tokio::test]
async fn inherited_stdin_is_available_only_when_explicitly_selected() {
    let fixture = Fixture::new();
    run_stdin_harness(&fixture, "inherit").await;
}

#[tokio::test]
async fn default_and_null_stdin_ignore_actual_parent_input_when_timed_or_untimed() {
    let fixture = Fixture::new();
    for mode in ["default", "default-timed", "null", "null-timed"] {
        run_stdin_harness(&fixture, mode).await;
    }
}

async fn run_stdin_harness(fixture: &Fixture, mode: &str) {
    let mut child = tokio::process::Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("stdin_child_harness")
        .arg("--ignored")
        .env("GIT_SPAWN_HARNESS_BIN", &fixture.bin)
        .env("GIT_SPAWN_HARNESS_MODE", mode)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(b"inherited\0\xff").await.unwrap();
    drop(stdin);
    let output = tokio::time::timeout(Duration::from_secs(5), child.wait_with_output())
        .await
        .expect("stdin harness did not finish")
        .unwrap();
    assert!(
        output.status.success(),
        "stdin harness mode {mode} failed: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
#[ignore = "run as a child with piped stdin"]
fn stdin_child_harness() {
    let bin = std::env::var_os("GIT_SPAWN_HARNESS_BIN").unwrap();
    let mode = std::env::var("GIT_SPAWN_HARNESS_MODE").unwrap();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let mut executor = CommandExecutor::new().with_env("PATH", bin);
            if mode.contains("timed") {
                executor = executor.timeout(Duration::from_secs(2));
            }
            if mode == "null" || mode == "null-timed" {
                executor = executor.stdin_null();
            } else if mode == "inherit" {
                executor = executor.stdin_inherit();
            }
            let output = executor
                .execute_command_os(vec!["input".into()])
                .await
                .unwrap();
            if mode == "inherit" {
                assert_eq!(output.stdout, b"inherited\0\xff");
            } else {
                assert!(output.stdout.is_empty(), "{mode} inherited parent data");
            }
        });
}

#[tokio::test]
async fn output_limits_bound_both_streams_and_reap_the_child() {
    let fixture = Fixture::new();
    #[cfg(target_os = "linux")]
    let _guard = ProcessGuard(fixture.path("parent.pid"));
    let err = tokio::time::timeout(
        Duration::from_secs(5),
        fixture
            .executor()
            .output_limits(OutputLimits {
                stdout: Some(127),
                stderr: Some(131),
            })
            .execute_command_os_unchecked(vec!["flood".into()]),
    )
    .await
    .expect("full stdout/stderr pipes deadlocked")
    .unwrap_err();
    let failure = failure(err);
    assert!(matches!(
        failure.kind,
        ExecutionFailureKind::CaptureLimit {
            stream: OutputStream::Stdout | OutputStream::Stderr,
            ..
        }
    ));
    assert!(failure.stdout.len() <= 127);
    assert!(failure.stderr.len() <= 131);
    match failure.kind {
        ExecutionFailureKind::CaptureLimit {
            stream: OutputStream::Stdout,
            ..
        } => assert!(failure.capture.stdout_truncated),
        ExecutionFailureKind::CaptureLimit {
            stream: OutputStream::Stderr,
            ..
        } => assert!(failure.capture.stderr_truncated),
        _ => unreachable!(),
    }
    assert!(failure.cleanup.direct_child_reaped);
}

#[tokio::test]
async fn both_pipes_are_drained_to_eof_without_capture_limits() {
    let fixture = Fixture::new();
    let output = tokio::time::timeout(
        Duration::from_secs(5),
        fixture
            .executor()
            .execute_command_os(vec!["finite-output".into()]),
    )
    .await
    .expect("simultaneous stdout and stderr writes deadlocked")
    .unwrap();
    assert_eq!(output.stdout.len(), 16 * 8192);
    assert_eq!(output.stderr.len(), 16 * 8192);
    assert_eq!(&output.stdout[..16], b"0123456789abcdef");
    assert_eq!(&output.stderr[..16], b"fedcba9876543210");
}

#[tokio::test]
async fn capture_limit_is_inclusive_and_zero_means_no_bytes() {
    let fixture = Fixture::new();
    let full = fixture
        .executor()
        .output_limits(OutputLimits {
            stdout: Some(5),
            stderr: Some(5),
        })
        .execute_command_os_unchecked(vec!["bytes".into(), "0".into()])
        .await
        .unwrap();
    assert_eq!(full.stdout.len(), 5);
    assert_eq!(full.stderr.len(), 5);

    for stream in [OutputStream::Stdout, OutputStream::Stderr] {
        let limits = match stream {
            OutputStream::Stdout => OutputLimits {
                stdout: Some(0),
                stderr: None,
            },
            OutputStream::Stderr => OutputLimits {
                stdout: None,
                stderr: Some(0),
            },
        };
        let err = fixture
            .executor()
            .output_limits(limits)
            .execute_command_os_unchecked(vec!["bytes".into(), "0".into()])
            .await
            .unwrap_err();
        let failure = failure(err);
        assert!(matches!(
            failure.kind,
            ExecutionFailureKind::CaptureLimit { stream: found, limit: 0 } if found == stream
        ));
        match stream {
            OutputStream::Stdout => assert!(failure.capture.stdout_truncated),
            OutputStream::Stderr => assert!(failure.capture.stderr_truncated),
        }
    }
}

#[tokio::test]
async fn timeout_retains_partial_binary_output_and_reports_cleanup() {
    let fixture = Fixture::new();
    #[cfg(target_os = "linux")]
    let _guard = ProcessGuard(fixture.path("parent.pid"));
    let err = fixture
        .executor()
        .timeout(Duration::from_millis(250))
        .execute_command_os_unchecked(vec!["block".into()])
        .await
        .unwrap_err();
    let failure = failure(err);
    assert!(matches!(
        failure.kind,
        ExecutionFailureKind::TimedOut { .. }
    ));
    assert_eq!(failure.stdout, b"out\0");
    assert_eq!(failure.stderr, b"err\xff");
    assert!(failure.cleanup.direct_child_reaped);
}

async fn cancelled_command(has_timeout: bool) {
    let fixture = Fixture::new();
    #[cfg(target_os = "linux")]
    let _guard = ProcessGuard(fixture.path("parent.pid"));
    let token = CancellationToken::new();
    let mut executor = fixture.executor().cancellation_token(token.clone());
    if has_timeout {
        executor = executor.timeout(Duration::from_secs(30));
    }
    let command = tokio::spawn(async move {
        executor
            .execute_command_os_unchecked(vec!["block".into()])
            .await
    });
    fixture.ready().await;
    token.cancel();
    let err = tokio::time::timeout(Duration::from_secs(5), command)
        .await
        .expect("cancellation did not settle")
        .unwrap()
        .unwrap_err();
    let failure = failure(err);
    assert!(matches!(failure.kind, ExecutionFailureKind::Cancelled));
    assert_eq!(failure.stdout, b"out\0");
    assert_eq!(failure.stderr, b"err\xff");
    assert!(failure.cleanup.direct_child_reaped);
}

#[tokio::test]
async fn cancellation_without_a_timeout_reaps_and_preserves_bytes() {
    cancelled_command(false).await;
}

#[tokio::test]
async fn cancellation_with_a_timeout_reaps_and_preserves_bytes() {
    cancelled_command(true).await;
}

#[tokio::test]
async fn cancellation_interrupts_a_blocked_stdin_writer() {
    let fixture = Fixture::new();
    #[cfg(target_os = "linux")]
    let _guard = ProcessGuard(fixture.path("parent.pid"));
    let token = CancellationToken::new();
    let executor = fixture
        .executor()
        .stdin_bytes(vec![b'x'; 4 * 1024 * 1024])
        .cancellation_token(token.clone());
    let task = tokio::spawn(async move {
        executor
            .execute_command_os_unchecked(vec!["no-read".into()])
            .await
    });
    fixture.ready().await;
    token.cancel();
    let err = tokio::time::timeout(Duration::from_secs(5), task)
        .await
        .expect("blocked writer did not respond to cancellation")
        .unwrap()
        .unwrap_err();
    let failure = failure(err);
    assert!(matches!(failure.kind, ExecutionFailureKind::Cancelled));
    assert!(failure.cleanup.direct_child_reaped);
}

#[tokio::test]
async fn child_environment_can_override_and_remove_inherited_values() {
    let fixture = Fixture::new();
    let original = std::env::var_os("GIT_SPAWN_TEST_INHERITED");
    let current_exe = std::env::current_exe().unwrap();
    let output = tokio::process::Command::new(current_exe)
        .arg("--exact")
        .arg("environment_child_harness")
        .arg("--ignored")
        .env("GIT_SPAWN_TEST_INHERITED", "inherited-sentinel")
        .env("GIT_SPAWN_HARNESS_BIN", &fixture.bin)
        .output()
        .await
        .unwrap();
    assert!(
        output.status.success(),
        "child harness failed: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert_eq!(std::env::var_os("GIT_SPAWN_TEST_INHERITED"), original);
}

#[test]
#[ignore = "run as a child with an isolated inherited environment"]
fn environment_child_harness() {
    let bin = std::env::var_os("GIT_SPAWN_HARNESS_BIN").unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let executor = CommandExecutor::new()
            .with_env("PATH", bin)
            .with_env("GIT_SPAWN_TEST_OVERRIDE", "override-sentinel")
            .env_remove("GIT_SPAWN_TEST_INHERITED");
        let output = executor
            .execute_command_os_unchecked(vec!["environment".into()])
            .await
            .unwrap();
        assert_eq!(output.stdout, b"override-sentinel|unset");
        assert_eq!(
            std::env::var("GIT_SPAWN_TEST_INHERITED").unwrap(),
            "inherited-sentinel"
        );
        assert!(!format!("{executor:?}").contains("override-sentinel"));
    });
}

#[cfg(target_os = "linux")]
fn active_process(pid: i32) -> bool {
    let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) else {
        return false;
    };
    // The comm field may contain spaces and parentheses; state follows its
    // final closing parenthesis. A zombie is already incapable of side effects.
    let state = stat.rsplit_once(") ").unwrap().1.as_bytes()[0];
    state != b'Z' && state != b'X'
}

#[cfg(target_os = "linux")]
fn process_start(pid: i32) -> Option<String> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    Some(
        stat.rsplit_once(") ")?
            .1
            .split_whitespace()
            .nth(19)?
            .to_owned(),
    )
}

#[cfg(target_os = "linux")]
struct EscapedGuard(PathBuf);

#[cfg(target_os = "linux")]
impl Drop for EscapedGuard {
    fn drop(&mut self) {
        let Ok(identity) = std::fs::read_to_string(&self.0) else {
            return;
        };
        let mut fields = identity.split_whitespace();
        let Some(pid) = fields.next().and_then(|value| value.parse::<i32>().ok()) else {
            return;
        };
        let Some(start) = fields.next() else {
            return;
        };
        if pid > 1 && process_start(pid).as_deref() == Some(start) {
            // SAFETY: PID/start-time identifies this private fixture, which
            // deliberately owns a new session and group. Do not signal reuse.
            unsafe {
                if libc::getpgid(pid) == pid && pid != libc::getpgrp() {
                    libc::kill(-pid, libc::SIGKILL);
                }
            }
        }
    }
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "spawned with inherited pipes to test incomplete cleanup"]
fn escaped_pipe_child_harness() {
    use std::io::Write;
    let ready = PathBuf::from(std::env::var_os("GIT_SPAWN_TEST_READY").unwrap());
    let identity = PathBuf::from(std::env::var_os("GIT_SPAWN_ESCAPED_IDENTITY").unwrap());
    // SAFETY: this isolated helper deliberately leaves the Git-owned group.
    assert_ne!(unsafe { libc::setsid() }, -1);
    let pid = std::process::id() as i32;
    std::fs::write(identity, format!("{pid} {}", process_start(pid).unwrap())).unwrap();
    std::io::stdout().write_all(b"escaped-out\0").unwrap();
    std::io::stdout().flush().unwrap();
    std::io::stderr().write_all(b"escaped-err\xff").unwrap();
    std::io::stderr().flush().unwrap();
    std::fs::write(ready, b"ready").unwrap();
    // A watchdog bounds the helper even if its owning test process dies.
    std::thread::sleep(Duration::from_secs(15));
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn cleanup_deadline_reports_open_pipes_from_an_escaped_descendant() {
    let fixture = Fixture::new();
    let _parent_guard = ProcessGuard(fixture.path("parent.pid"));
    let identity = fixture.path("escaped.identity");
    let escaped_guard = EscapedGuard(identity.clone());
    let token = CancellationToken::new();
    let executor = fixture
        .executor()
        .with_env("GIT_SPAWN_HARNESS_EXE", std::env::current_exe().unwrap())
        .with_env("GIT_SPAWN_ESCAPED_IDENTITY", &identity)
        .cancellation_token(token.clone())
        .cleanup_timeout(Duration::from_millis(150));
    let task = tokio::spawn(async move {
        executor
            .execute_command_os_unchecked(vec!["escaped-pipe".into()])
            .await
    });
    fixture.ready().await;
    let pid = std::fs::read_to_string(&identity)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .parse::<i32>()
        .unwrap();
    token.cancel();
    let failure = failure(
        tokio::time::timeout(Duration::from_secs(5), task)
            .await
            .expect("escaped output pipes prevented bounded cleanup")
            .unwrap()
            .unwrap_err(),
    );
    assert!(matches!(failure.kind, ExecutionFailureKind::Cancelled));
    assert!(failure.cleanup.direct_child_reaped);
    assert!(!failure.capture.stdout_pipe_eof);
    assert!(!failure.capture.stderr_pipe_eof);
    assert!(
        active_process(pid),
        "fixture did not survive outside containment"
    );
    drop(escaped_guard);
    wait_until_stopped(pid).await;
}

#[cfg(target_os = "linux")]
async fn wait_until_stopped(pid: i32) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while active_process(pid) {
        assert!(Instant::now() < deadline, "process {pid} kept executing");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[cfg(target_os = "linux")]
struct ProcessGuard(PathBuf);

#[cfg(target_os = "linux")]
impl Drop for ProcessGuard {
    fn drop(&mut self) {
        if let Ok(contents) = std::fs::read_to_string(&self.0) {
            if let Ok(pid) = contents.parse::<i32>() {
                let descendant = self.0.with_file_name("descendant.pid");
                let descendant = std::fs::read_to_string(descendant)
                    .ok()
                    .and_then(|value| value.parse::<i32>().ok());
                // SAFETY: all queried PIDs came from the private fixture.
                unsafe {
                    let live_member = active_process(pid) || descendant.is_some_and(active_process);
                    let same_group = libc::getpgid(pid) == pid
                        || descendant.is_some_and(|member| libc::getpgid(member) == pid);
                    if pid > 1 && live_member && same_group && pid != libc::getpgrp() {
                        libc::kill(-pid, libc::SIGKILL);
                    } else if pid > 1 && active_process(pid) {
                        libc::kill(pid, libc::SIGKILL);
                    }
                }
            }
        }
    }
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn dropping_an_unbounded_command_future_stops_its_descendant() {
    let fixture = Fixture::new();
    let _guard = ProcessGuard(fixture.path("parent.pid"));
    let executor = fixture.executor();
    let task = tokio::spawn(async move {
        executor
            .execute_command_os_unchecked(vec!["descendant".into()])
            .await
    });
    fixture.ready().await;
    wait_for_file(&fixture.path("parent.pid")).await;
    wait_for_file(&fixture.path("descendant.pid")).await;
    let parent: i32 = std::fs::read_to_string(fixture.path("parent.pid"))
        .unwrap()
        .parse()
        .unwrap();
    let descendant: i32 = std::fs::read_to_string(fixture.path("descendant.pid"))
        .unwrap()
        .parse()
        .unwrap();
    assert!(active_process(parent));
    assert!(active_process(descendant));

    task.abort();
    let _ = task.await;
    wait_until_stopped(parent).await;
    wait_until_stopped(descendant).await;
    std::fs::write(fixture.path("release"), b"go").unwrap();
    assert!(!fixture.path("effect").exists());
}

#[cfg(target_os = "linux")]
async fn interrupted_descendant(cancel: bool) {
    let fixture = Fixture::new();
    let _guard = ProcessGuard(fixture.path("parent.pid"));
    let token = CancellationToken::new();
    let mut executor = fixture.executor();
    if cancel {
        executor = executor.cancellation_token(token.clone());
    } else {
        executor = executor.timeout(Duration::from_millis(500));
    }
    let task = tokio::spawn(async move {
        executor
            .execute_command_os_unchecked(vec!["descendant".into()])
            .await
    });
    fixture.ready().await;
    let parent: i32 = std::fs::read_to_string(fixture.path("parent.pid"))
        .unwrap()
        .parse()
        .unwrap();
    let descendant: i32 = std::fs::read_to_string(fixture.path("descendant.pid"))
        .unwrap()
        .parse()
        .unwrap();
    assert!(active_process(parent));
    assert!(active_process(descendant));
    if cancel {
        token.cancel();
    }
    let err = tokio::time::timeout(Duration::from_secs(5), task)
        .await
        .expect("interruption failed to settle")
        .unwrap()
        .unwrap_err();
    let failure = failure(err);
    assert!(failure.cleanup.direct_child_reaped);
    if cancel {
        assert!(matches!(failure.kind, ExecutionFailureKind::Cancelled));
    } else {
        assert!(matches!(
            failure.kind,
            ExecutionFailureKind::TimedOut { .. }
        ));
    }
    wait_until_stopped(parent).await;
    wait_until_stopped(descendant).await;
    std::fs::write(fixture.path("release"), b"go").unwrap();
    assert!(!fixture.path("effect").exists());
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn cancellation_stops_a_descendant() {
    interrupted_descendant(true).await;
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn timeout_stops_a_descendant() {
    interrupted_descendant(false).await;
}

#[cfg(target_os = "linux")]
async fn interrupted_after_parent_exits(cancel: bool) {
    let fixture = Fixture::new();
    let _guard = ProcessGuard(fixture.path("parent.pid"));
    let token = CancellationToken::new();
    let mut executor = fixture.executor();
    if cancel {
        executor = executor.cancellation_token(token.clone());
    } else {
        executor = executor.timeout(Duration::from_secs(1));
    }
    let task = tokio::spawn(async move {
        executor
            .execute_command_os_unchecked(vec!["orphaned-pipe".into()])
            .await
    });
    fixture.ready().await;
    let parent: i32 = std::fs::read_to_string(fixture.path("parent.pid"))
        .unwrap()
        .parse()
        .unwrap();
    let descendant: i32 = std::fs::read_to_string(fixture.path("descendant.pid"))
        .unwrap()
        .parse()
        .unwrap();
    wait_until_stopped(parent).await;
    assert!(active_process(descendant), "pipe holder exited too soon");
    assert!(!task.is_finished(), "parent exit hid the live pipe holder");
    if cancel {
        token.cancel();
    }
    let err = tokio::time::timeout(Duration::from_secs(5), task)
        .await
        .expect("interruption failed to settle")
        .unwrap()
        .unwrap_err();
    let failure = failure(err);
    assert!(failure.cleanup.direct_child_reaped);
    if cancel {
        assert!(matches!(failure.kind, ExecutionFailureKind::Cancelled));
    } else {
        assert!(matches!(
            failure.kind,
            ExecutionFailureKind::TimedOut { .. }
        ));
    }
    wait_until_stopped(descendant).await;
    std::fs::write(fixture.path("release"), b"go").unwrap();
    assert!(!fixture.path("effect").exists());
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn cancellation_cleans_up_after_direct_child_exits_but_pipe_remains_open() {
    interrupted_after_parent_exits(true).await;
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn timeout_cleans_up_after_direct_child_exits_but_pipe_remains_open() {
    interrupted_after_parent_exits(false).await;
}
