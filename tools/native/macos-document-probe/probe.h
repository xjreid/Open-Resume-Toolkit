// Synthetic development probe only. Never link this code into the desktop or parser.
#ifndef ORT_DOCUMENT_PROBE_H
#define ORT_DOCUMENT_PROBE_H

#include <CoreFoundation/CoreFoundation.h>
#include <Security/SecTask.h>
#include <arpa/inet.h>
#include <errno.h>
#include <fcntl.h>
#include <spawn.h>
#include <stdbool.h>
#include <stdint.h>
#include <string.h>
#include <sys/resource.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/wait.h>
#include <unistd.h>
#include <xpc/xpc.h>

#define PROBE_SERVICE "com.openresumetoolkit.document-sandbox-probe.worker"
#define PROBE_MARKER "ORT_SYNTHETIC_DESCRIPTOR_V1\n"
#define PROBE_FD_LIMIT 64

// Filesystem/network denials require permission errors. Child probes additionally
// accept EAGAIN only under a verified zero NPROC limit (see child_outcome).
// Missing files, dead listeners and arbitrary errors never produce evidence.
enum probe_outcome { PROBE_ALLOWED = 0, PROBE_DENIED = 1, PROBE_ERROR = 2 };

static int access_outcome(int error) {
    return (error == EPERM || error == EACCES) ? PROBE_DENIED : PROBE_ERROR;
}

static int probe_path(const char *path, int flags) {
    int fd = open(path, flags | O_CLOEXEC);
    if (fd < 0) return access_outcome(errno);
    // Opening for write is sufficient to demonstrate write authority. Do not
    // truncate/modify even these synthetic targets.
    close(fd);
    return PROBE_ALLOWED;
}

static int probe_loopback(uint16_t port) {
    int fd = socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0) return access_outcome(errno);
    struct timeval timeout = {.tv_sec = 1, .tv_usec = 0};
    if (setsockopt(fd, SOL_SOCKET, SO_SNDTIMEO, &timeout, sizeof(timeout)) != 0) {
        close(fd);
        return PROBE_ERROR;
    }
    struct sockaddr_in address = {0};
    address.sin_len = sizeof(address);
    address.sin_family = AF_INET;
    address.sin_port = htons(port);
    address.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
    int rc = connect(fd, (struct sockaddr *)&address, sizeof(address));
    int error = errno;
    close(fd);
    return rc == 0 ? PROBE_ALLOWED : access_outcome(error);
}

static bool exact_limit(int resource, rlim_t value) {
    struct rlimit limit;
    return getrlimit(resource, &limit) == 0
        && limit.rlim_cur == value && limit.rlim_max == value;
}

static int child_outcome(int error, bool limited) {
    // EAGAIN alone is ambiguous. Accept it only for a non-root process whose
    // zero soft AND hard NPROC limits have just been verified. The same helper
    // must also pass an unrestricted positive control before applying limits.
    if (error == EAGAIN && limited && getuid() != 0 && geteuid() != 0
        && exact_limit(RLIMIT_NPROC, 0)) return PROBE_DENIED;
    return access_outcome(error);
}

static int probe_child(bool limited) {
    // Fixed, harmless command; no shell, user arguments or inherited secrets.
    char *const args[] = {"/usr/bin/true", NULL};
    char *const environment[] = {"PATH=/usr/bin:/bin", NULL};
    pid_t child;
    int rc = posix_spawn(&child, args[0], NULL, NULL, args, environment);
    if (rc != 0) return child_outcome(rc, limited);
    int status = 0;
    pid_t waited;
    do { waited = waitpid(child, &status, 0); } while (waited < 0 && errno == EINTR);
    // Creation itself violates the planned no-child boundary, even if the
    // child subsequently crashes or is denied a resource. Always reap it.
    return waited == child ? PROBE_ALLOWED : PROBE_ERROR;
}

static int probe_fork(bool limited) {
    pid_t child = fork();
    // XPC is multithreaded: the child only uses async-signal-safe _exit. Never
    // touch Foundation, malloc, XPC or logging in a post-fork child.
    if (child == 0) _exit(0);
    if (child < 0) return child_outcome(errno, limited);
    int status = 0;
    pid_t waited;
    do { waited = waitpid(child, &status, 0); } while (waited < 0 && errno == EINTR);
    return waited == child ? PROBE_ALLOWED : PROBE_ERROR;
}

static bool has_sandbox_entitlement(void) {
    SecTaskRef task = SecTaskCreateFromSelf(kCFAllocatorDefault);
    if (!task) return false;
    CFTypeRef value = SecTaskCopyValueForEntitlement(task, CFSTR("com.apple.security.app-sandbox"), NULL);
    bool enabled = value && CFGetTypeID(value) == CFBooleanGetTypeID()
        && CFBooleanGetValue((CFBooleanRef)value);
    if (value) CFRelease(value);
    CFRelease(task);
    return enabled;
}

static xpc_object_t run_probes(int input, const char *sibling, const char *alias, uint16_t port, bool limited) {
    xpc_object_t result = xpc_dictionary_create(NULL, NULL, 0);
    char buffer[sizeof(PROBE_MARKER)] = {0};
    struct stat metadata;
    bool readable = fstat(input, &metadata) == 0 && S_ISREG(metadata.st_mode)
        && metadata.st_size == sizeof(PROBE_MARKER) - 1
        && pread(input, buffer, sizeof(buffer), 0) == sizeof(PROBE_MARKER) - 1
        && memcmp(buffer, PROBE_MARKER, sizeof(PROBE_MARKER) - 1) == 0;
    // pwrite on the read-only transferred descriptor must return EBADF.
    int write_rc = (int)pwrite(input, "X", 1, 0);
    bool read_only = write_rc == -1 && errno == EBADF;
    xpc_dictionary_set_bool(result, "descriptorRead", readable);
    xpc_dictionary_set_bool(result, "descriptorReadOnly", read_only);
    xpc_dictionary_set_bool(result, "sandboxEntitlement", has_sandbox_entitlement());
    xpc_dictionary_set_int64(result, "siblingRead", probe_path(sibling, O_RDONLY));
    xpc_dictionary_set_int64(result, "siblingWrite", probe_path(sibling, O_WRONLY));
    xpc_dictionary_set_int64(result, "symlinkRead", probe_path(alias, O_RDONLY));
    xpc_dictionary_set_int64(result, "loopbackConnect", probe_loopback(port));
    xpc_dictionary_set_int64(result, "childCreation", probe_child(limited));
    xpc_dictionary_set_int64(result, "childFork", probe_fork(limited));
    return result;
}

#endif
