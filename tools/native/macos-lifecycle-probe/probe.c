// Synthetic supervision experiment only. Never linked into ORT or a parser.
// Compile separately as PROBE_HOST, PROBE_SUPERVISOR and PROBE_CHILD.
#include <CoreFoundation/CoreFoundation.h>
#include <Security/SecTask.h>
#include <dispatch/dispatch.h>
#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <mach-o/dyld.h>
#include <poll.h>
#include <signal.h>
#include <spawn.h>
#include <stdatomic.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/resource.h>
#include <sys/stat.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>
#include <xpc/xpc.h>

#define SERVICE "com.openresumetoolkit.lifecycle-probe.supervisor"
#define INPUT "ORT_SYNTHETIC_DESCRIPTOR_V1\n"
#define READY "ORT_READY_V1\n"
#define RESULT "ORT_RESULT_V1\n"
#define PIPE_LIMIT 4096
enum { NORMAL, CANCEL, TIMEOUT, FLOOD_OUT, FLOOD_ERR, BAD_EXIT, BAD_OUTPUT, RESULT_STALL, EOF_STALL, CASE_COUNT };
enum { OK, CANCELLED, TIMED_OUT, OUTPUT_LIMIT, WORKER_FAILED, PROTOCOL, IO_FAILED };

#if defined(PROBE_CHILD) || defined(PROBE_SUPERVISOR)
static bool sandboxed(void) {
    SecTaskRef task = SecTaskCreateFromSelf(kCFAllocatorDefault);
    if (!task) return false;
    CFTypeRef value = SecTaskCopyValueForEntitlement(task, CFSTR("com.apple.security.app-sandbox"), NULL);
    bool valid = value && CFGetTypeID(value) == CFBooleanGetTypeID()
        && CFBooleanGetValue((CFBooleanRef)value);
    if (value) CFRelease(value);
    CFRelease(task);
    return valid;
}
#endif

#if defined(PROBE_CHILD)
static bool limit(int resource, rlim_t value) {
    struct rlimit before, after, requested = {.rlim_cur = value, .rlim_max = value};
    return getrlimit(resource, &before) == 0 && before.rlim_cur >= value
        && before.rlim_max >= value && setrlimit(resource, &requested) == 0
        && getrlimit(resource, &after) == 0
        && after.rlim_cur == value && after.rlim_max == value;
}

static void emit(int fd, const char *bytes, size_t length) {
    while (length) {
        ssize_t written = write(fd, bytes, length);
        if (written < 0 && errno == EINTR) continue;
        if (written <= 0) _exit(66);
        bytes += written;
        length -= (size_t)written;
    }
}

int main(int argc, char **argv) {
    if (argc != 2 || strlen(argv[1]) != 1 || argv[1][0] < '0'
        || argv[1][0] >= '0' + CASE_COUNT || getuid() == 0 || geteuid() == 0) return 65;
    int mode = argv[1][0] - '0';
    // Inheritance is from the minimal sandboxed supervisor, not the desktop.
    // Apply limits before even reading the fixed synthetic input.
    if (!sandboxed() || !limit(RLIMIT_CORE, 0) || !limit(RLIMIT_NPROC, 0)
        || !limit(RLIMIT_NOFILE, 64)) return 65;
    struct sigaction ignored = {0};
    ignored.sa_handler = SIG_IGN;
    sigemptyset(&ignored.sa_mask);
    if (sigaction(SIGTERM, &ignored, NULL) != 0) return 65;
    // Test-only fallback. A SIGALRM exit never counts as supervisor success.
    alarm(12);
    char input[sizeof(INPUT)] = {0};
    struct stat metadata;
    if (fstat(STDIN_FILENO, &metadata) != 0 || !S_ISREG(metadata.st_mode)
        || metadata.st_size != sizeof(INPUT) - 1
        || pread(STDIN_FILENO, input, sizeof(input), 0) != sizeof(INPUT) - 1
        || memcmp(input, INPUT, sizeof(INPUT) - 1) != 0
        || pwrite(STDIN_FILENO, "X", 1, 0) != -1 || errno != EBADF) return 65;
    emit(STDOUT_FILENO, READY, sizeof(READY) - 1);
    if (mode == CANCEL || mode == TIMEOUT) for (;;) pause();
    if (mode == FLOOD_OUT || mode == FLOOD_ERR) {
        char chunk[1024];
        memset(chunk, 'x', sizeof(chunk));
        for (;;) emit(mode == FLOOD_OUT ? STDOUT_FILENO : STDERR_FILENO, chunk, sizeof(chunk));
    }
    if (mode == BAD_OUTPUT) emit(STDOUT_FILENO, "INVALID\n", 8);
    else emit(STDOUT_FILENO, RESULT, sizeof(RESULT) - 1);
    if (mode == EOF_STALL) { close(STDOUT_FILENO); close(STDERR_FILENO); }
    if (mode == RESULT_STALL || mode == EOF_STALL) for (;;) pause();
    return mode == BAD_EXIT ? 65 : 0;
}
#endif

#if defined(PROBE_SUPERVISOR)
static atomic_flag used = ATOMIC_FLAG_INIT;
static atomic_bool cancelled = false;

static uint64_t milliseconds(void) {
    struct timespec time;
    if (clock_gettime(CLOCK_MONOTONIC, &time) != 0) _exit(70);
    return (uint64_t)time.tv_sec * 1000 + (uint64_t)time.tv_nsec / 1000000;
}

static bool reader_pipe(int descriptors[2]) {
    if (pipe(descriptors) != 0) return false;
    for (size_t i = 0; i < 2; i++) {
        if (descriptors[i] < 3 || fcntl(descriptors[i], F_SETFD, FD_CLOEXEC) != 0) return false;
    }
    return fcntl(descriptors[0], F_SETFL, O_NONBLOCK) == 0;
}

static void supervise(xpc_connection_t peer, xpc_object_t reply, int input, int mode) {
    // Keep all action sources above stdio so dup2 ordering cannot alias another
    // source if a service runtime has closed one of its standard descriptors.
    if (input < 3) {
        int relocated = fcntl(input, F_DUPFD_CLOEXEC, 3);
        close(input);
        input = relocated;
    }
    struct stat metadata;
    int flags = fcntl(input, F_GETFL);
    if (input < 0 || flags < 0 || (flags & O_ACCMODE) != O_RDONLY
        || fstat(input, &metadata) != 0 || !S_ISREG(metadata.st_mode)
        || metadata.st_size != sizeof(INPUT) - 1) _exit(70);
    int output[2] = {-1, -1}, errors[2] = {-1, -1};
    char executable[PATH_MAX], canonical[PATH_MAX], child_path[PATH_MAX];
    uint32_t capacity = sizeof(executable);
    // Resolve only this signed test service's fixed embedded child. No supplied
    // executable, shell, PATH search, general file operation or process ID.
    if (_NSGetExecutablePath(executable, &capacity) != 0 || !realpath(executable, canonical)) _exit(70);
    char *separator = strrchr(canonical, '/');
    if (!separator) _exit(70);
    *separator = '\0';
    int length = snprintf(child_path, sizeof(child_path), "%s/ort-lifecycle-child", canonical);
    if (length < 0 || (size_t)length >= sizeof(child_path)
        || !reader_pipe(output) || !reader_pipe(errors)) _exit(70);
    posix_spawn_file_actions_t actions;
    posix_spawnattr_t attributes;
    if (posix_spawn_file_actions_init(&actions) != 0 || posix_spawnattr_init(&attributes) != 0) _exit(70);
    sigset_t empty_mask, default_signals;
    sigemptyset(&empty_mask);
    sigemptyset(&default_signals);
    sigaddset(&default_signals, SIGALRM);
    sigaddset(&default_signals, SIGTERM);
    sigaddset(&default_signals, SIGPIPE);
    if (posix_spawn_file_actions_adddup2(&actions, input, STDIN_FILENO) != 0
        || posix_spawn_file_actions_adddup2(&actions, output[1], STDOUT_FILENO) != 0
        || posix_spawn_file_actions_adddup2(&actions, errors[1], STDERR_FILENO) != 0
        || posix_spawnattr_setsigmask(&attributes, &empty_mask) != 0
        || posix_spawnattr_setsigdefault(&attributes, &default_signals) != 0
        || posix_spawnattr_setflags(&attributes, POSIX_SPAWN_CLOEXEC_DEFAULT
            | POSIX_SPAWN_SETSIGMASK | POSIX_SPAWN_SETSIGDEF) != 0) _exit(70);
    char mode_text[] = {(char)('0' + mode), '\0'};
    char *const args[] = {child_path, mode_text, NULL};
    char *const environment[] = {"PATH=/usr/bin:/bin", NULL};
    pid_t child = -1;
    uint64_t started = milliseconds();
    int launch = posix_spawn(&child, child_path, &actions, &attributes, args, environment);
    posix_spawn_file_actions_destroy(&actions);
    posix_spawnattr_destroy(&attributes);
    close(input);
    close(output[1]);
    close(errors[1]);
    if (launch != 0 || child <= 0) _exit(70);

    // This single thread is the sole waitpid/signal owner. SIGCHLD stays default;
    // the direct child's PID cannot be recycled until WE reap it. Never signal
    // an XPC-provided PID, a process group, or a PID after waitpid has reaped it.
    int status = 0, reason = OK;
    bool reaped = false, killed = false, ready = false;
    bool eof[2] = {false, false};
    size_t totals[2] = {0, 0};
    char prefix[64] = {0};
    size_t retained = 0;
    int readers[] = {output[0], errors[0]};
    while (milliseconds() - started < 4000) {
        if (reason == OK && atomic_load(&cancelled)) reason = CANCELLED;
        if (reason == OK && milliseconds() - started >= 1000) reason = TIMED_OUT;
        if (reason != OK && !reaped && !killed) {
            if (kill(child, SIGKILL) == 0) killed = true;
            else if (errno != ESRCH) { reason = IO_FAILED; break; }
        }
        if (!reaped) {
            pid_t waited = waitpid(child, &status, WNOHANG);
            if (waited == child) reaped = true;
            // Unexpected wait failure loses ownership evidence. Leave the loop
            // immediately; never send another signal using this PID.
            else if (waited < 0 && errno != EINTR) { reason = IO_FAILED; break; }
        }
        // Drain both streams in bounded/fair chunks. No read_to_end, output-sized
        // allocation or decoded/logged stderr. Poll deadlines even when silent.
        for (size_t stream = 0; stream < 2; stream++) {
            if (eof[stream]) continue;
            char chunk[1024];
            ssize_t count = read(readers[stream], chunk, sizeof(chunk));
            if (count == 0) { eof[stream] = true; continue; }
            if (count < 0) {
                if (errno != EAGAIN && errno != EINTR && reason == OK) reason = IO_FAILED;
                continue;
            }
            size_t size = (size_t)count;
            if (totals[stream] > PIPE_LIMIT || size > PIPE_LIMIT - totals[stream]) {
                totals[stream] = PIPE_LIMIT + 1;
                if (reason == OK) reason = OUTPUT_LIMIT;
            } else totals[stream] += size;
            if (stream == 0 && retained < sizeof(prefix)) {
                size_t copy = size < sizeof(prefix) - retained ? size : sizeof(prefix) - retained;
                memcpy(prefix + retained, chunk, copy);
                retained += copy;
            }
        }
        if (!ready && retained >= sizeof(READY) - 1 && memcmp(prefix, READY, sizeof(READY) - 1) == 0) {
            ready = true;
            xpc_object_t notice = xpc_dictionary_create(NULL, NULL, 0);
            xpc_dictionary_set_bool(notice, "ready", true);
            xpc_connection_send_message(peer, notice);
            xpc_release(notice);
        }
        if (reaped && eof[0] && eof[1]) break;
        struct pollfd pollers[2] = {{.fd = eof[0] ? -1 : readers[0], .events = POLLIN},
                                    {.fd = eof[1] ? -1 : readers[1], .events = POLLIN}};
        if (poll(pollers, 2, 10) < 0 && errno != EINTR && reason == OK) reason = IO_FAILED;
    }
    close(readers[0]);
    close(readers[1]);
    // Only OS exit status plus both EOFs can accept the fixed result. A ready
    // message or valid bytes before nonzero/signal exit are never success.
    if (!reaped || !eof[0] || !eof[1]) reason = IO_FAILED;
    if (reason == OK && (!WIFEXITED(status) || WEXITSTATUS(status) != 0)) reason = WORKER_FAILED;
    const char expected[] = READY RESULT;
    if (reason == OK && (!ready || totals[0] != sizeof(expected) - 1 || totals[1] != 0
        || retained != sizeof(expected) - 1 || memcmp(prefix, expected, sizeof(expected) - 1) != 0)) reason = PROTOCOL;
    // Check cancellation once more after draining to cover late output/exit.
    if (reason == OK && atomic_load(&cancelled)) reason = CANCELLED;
    xpc_dictionary_set_uint64(reply, "case", mode);
    xpc_dictionary_set_uint64(reply, "reason", reason);
    xpc_dictionary_set_bool(reply, "readyObserved", ready);
    xpc_dictionary_set_bool(reply, "childReaped", reaped);
    xpc_dictionary_set_bool(reply, "killSent", killed);
    xpc_dictionary_set_bool(reply, "stdoutEof", eof[0]);
    xpc_dictionary_set_bool(reply, "stderrEof", eof[1]);
    xpc_dictionary_set_bool(reply, "accepted", reason == OK);
    xpc_dictionary_set_int64(reply, "exitCode", reaped && WIFEXITED(status) ? WEXITSTATUS(status) : -1);
    xpc_dictionary_set_int64(reply, "signal", reaped && WIFSIGNALED(status) ? WTERMSIG(status) : 0);
    xpc_dictionary_set_uint64(reply, "stdoutBytes", totals[0]);
    xpc_dictionary_set_uint64(reply, "stderrBytes", totals[1]);
    xpc_dictionary_set_uint64(reply, "elapsedMs", milliseconds() - started);
    xpc_connection_send_message(peer, reply);
    xpc_release(reply);
    xpc_release(peer);
    dispatch_after(dispatch_time(DISPATCH_TIME_NOW, NSEC_PER_SEC), dispatch_get_main_queue(), ^{ _exit(0); });
}

static void accept_peer(xpc_connection_t peer) {
    __block bool owner = false;
    xpc_connection_set_target_queue(peer, dispatch_get_main_queue());
    xpc_connection_set_event_handler(peer, ^(xpc_object_t request) {
        if (xpc_get_type(request) == XPC_TYPE_ERROR) {
            if (owner) atomic_store(&cancelled, true);
            return;
        }
        if (xpc_get_type(request) != XPC_TYPE_DICTIONARY) return;
        if (owner && xpc_dictionary_get_count(request) == 1
            && xpc_dictionary_get_bool(request, "cancel")) { atomic_store(&cancelled, true); return; }
        xpc_object_t mode_value = xpc_dictionary_get_value(request, "start");
        if (xpc_dictionary_get_count(request) != 2 || !mode_value
            || xpc_get_type(mode_value) != XPC_TYPE_UINT64
            || xpc_uint64_get_value(mode_value) >= CASE_COUNT || atomic_flag_test_and_set(&used)) return;
        int mode = (int)xpc_uint64_get_value(mode_value);
        int input = xpc_dictionary_dup_fd(request, "input");
        xpc_object_t reply = xpc_dictionary_create_reply(request);
        if (input < 0 || !reply) _exit(65);
        owner = true;
        xpc_retain(peer);
        dispatch_async(dispatch_get_global_queue(QOS_CLASS_DEFAULT, 0), ^{ supervise(peer, reply, input, mode); });
    });
    xpc_connection_resume(peer);
}

int main(void) {
    if (getuid() == 0 || geteuid() == 0 || !sandboxed()) return 65;
    struct sigaction child_status = {0};
    child_status.sa_handler = SIG_DFL;
    sigemptyset(&child_status.sa_mask);
    if (sigaction(SIGCHLD, &child_status, NULL) != 0) return 65;
    xpc_main(accept_peer);
}
#endif

#if defined(PROBE_HOST)
int main(int argc, char **argv) {
    if (argc != 3 || strlen(argv[2]) != 1 || argv[2][0] < '0'
        || argv[2][0] >= '0' + CASE_COUNT || getuid() == 0 || geteuid() == 0) return 65;
    const char prefix[] = "/private/tmp/ort-lifecycle-";
    char path[PATH_MAX];
    struct stat metadata;
    if (!realpath(argv[1], path) || strcmp(path, argv[1]) != 0
        || strncmp(path, prefix, sizeof(prefix) - 1) != 0) return 65;
    char *suffix = strchr(path + sizeof(prefix) - 1, '/');
    if (!suffix || suffix == path + sizeof(prefix) - 1 || strcmp(suffix, "/input.txt") != 0) return 65;
    *suffix = '\0';
    if (lstat(path, &metadata) != 0 || !S_ISDIR(metadata.st_mode)
        || metadata.st_uid != getuid() || (metadata.st_mode & 0777) != 0700) return 65;
    *suffix = '/';
    int input = open(path, O_RDONLY | O_CLOEXEC | O_NOFOLLOW);
    if (input < 0 || fstat(input, &metadata) != 0 || !S_ISREG(metadata.st_mode)
        || metadata.st_uid != getuid() || metadata.st_size != sizeof(INPUT) - 1) return 65;
    int mode = argv[2][0] - '0';
    xpc_connection_t peer = xpc_connection_create(SERVICE, dispatch_get_main_queue());
    if (!peer) return 70;
    __block bool ready = false;
    xpc_connection_set_event_handler(peer, ^(xpc_object_t event) {
        if (xpc_get_type(event) == XPC_TYPE_ERROR) _exit(70);
        if (xpc_get_type(event) != XPC_TYPE_DICTIONARY || ready
            || xpc_dictionary_get_count(event) != 1 || !xpc_dictionary_get_bool(event, "ready")) _exit(70);
        ready = true;
        if (mode == CANCEL) {
            xpc_object_t cancel = xpc_dictionary_create(NULL, NULL, 0);
            xpc_dictionary_set_bool(cancel, "cancel", true);
            xpc_connection_send_message(peer, cancel);
            xpc_release(cancel);
        }
    });
    xpc_connection_resume(peer);
    xpc_object_t request = xpc_dictionary_create(NULL, NULL, 0);
    xpc_dictionary_set_uint64(request, "start", mode);
    xpc_dictionary_set_fd(request, "input", input);
    xpc_connection_send_message_with_reply(peer, request, dispatch_get_main_queue(), ^(xpc_object_t reply) {
        const char *booleans[] = {"readyObserved", "childReaped", "killSent", "stdoutEof", "stderrEof", "accepted"};
        const char *numbers[] = {"case", "reason", "stdoutBytes", "stderrBytes", "elapsedMs"};
        const char *signed_numbers[] = {"exitCode", "signal"};
        if (xpc_get_type(reply) != XPC_TYPE_DICTIONARY || xpc_dictionary_get_count(reply) != 13) _exit(70);
        // Validate types before printing a fixed schema; never interpolate data strings.
        for (size_t i = 0; i < 6; i++) {
            xpc_object_t value = xpc_dictionary_get_value(reply, booleans[i]);
            if (!value || xpc_get_type(value) != XPC_TYPE_BOOL) _exit(70);
        }
        for (size_t i = 0; i < 5; i++) {
            xpc_object_t value = xpc_dictionary_get_value(reply, numbers[i]);
            if (!value || xpc_get_type(value) != XPC_TYPE_UINT64) _exit(70);
        }
        for (size_t i = 0; i < 2; i++) {
            xpc_object_t value = xpc_dictionary_get_value(reply, signed_numbers[i]);
            if (!value || xpc_get_type(value) != XPC_TYPE_INT64) _exit(70);
        }
        printf("{\"schemaVersion\":1");
        for (size_t i = 0; i < 6; i++) printf(",\"%s\":%s", booleans[i], xpc_dictionary_get_bool(reply, booleans[i]) ? "true" : "false");
        for (size_t i = 0; i < 5; i++) printf(",\"%s\":%llu", numbers[i], (unsigned long long)xpc_dictionary_get_uint64(reply, numbers[i]));
        for (size_t i = 0; i < 2; i++) printf(",\"%s\":%lld", signed_numbers[i], (long long)xpc_dictionary_get_int64(reply, signed_numbers[i]));
        puts("}");
        close(input);
        exit(0);
    });
    xpc_release(request);
    dispatch_after(dispatch_time(DISPATCH_TIME_NOW, 8 * NSEC_PER_SEC), dispatch_get_main_queue(), ^{ _exit(70); });
    dispatch_main();
}
#endif
