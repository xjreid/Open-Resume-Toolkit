// Synthetic helper-only experiment. Never call from the desktop/runner process.
#ifndef ORT_DOCUMENT_PROBE_LIMITS_H
#define ORT_DOCUMENT_PROBE_LIMITS_H

#include "probe.h"

static bool lower_limit(int resource, rlim_t value) {
    struct rlimit before;
    if (getrlimit(resource, &before) != 0
        || before.rlim_cur < value || before.rlim_max < value) return false;
    struct rlimit limit = {.rlim_cur = value, .rlim_max = value};
    return setrlimit(resource, &limit) == 0 && exact_limit(resource, value);
}

static bool raise_denied(int resource, rlim_t value) {
    struct rlimit raised = {.rlim_cur = value + 1, .rlim_max = value + 1};
    int rc = setrlimit(resource, &raised);
    int error = errno;
    return rc == -1 && error == EPERM && exact_limit(resource, value);
}

static bool install_limits(void) {
    // NPROC enforcement exempts root. Refuse root and never change uid, user-
    // wide settings, launchd settings, or the parent's resource limits.
    return getuid() != 0 && geteuid() != 0
        && lower_limit(RLIMIT_CORE, 0)
        && lower_limit(RLIMIT_NPROC, 0)
        && lower_limit(RLIMIT_NOFILE, PROBE_FD_LIMIT);
}

static xpc_object_t probe_limits(int input) {
    xpc_object_t result = xpc_dictionary_create(NULL, NULL, 0);
    const int resources[] = {RLIMIT_NPROC, RLIMIT_NOFILE, RLIMIT_CORE};
    const char *soft_keys[] = {"nprocSoft", "nofileSoft", "coreSoft"};
    const char *hard_keys[] = {"nprocHard", "nofileHard", "coreHard"};
    for (size_t i = 0; i < 3; i++) {
        struct rlimit limit;
        if (getrlimit(resources[i], &limit) != 0) _exit(65);
        xpc_dictionary_set_uint64(result, soft_keys[i], limit.rlim_cur);
        xpc_dictionary_set_uint64(result, hard_keys[i], limit.rlim_max);
    }
    bool immutable = raise_denied(RLIMIT_NPROC, 0)
        && raise_denied(RLIMIT_NOFILE, PROBE_FD_LIMIT)
        && raise_denied(RLIMIT_CORE, 0);
    xpc_dictionary_set_bool(result, "raiseDenied", immutable);

    // Exhaust only this helper's tiny descriptor budget. Duplicate a known-good
    // descriptor, not new file paths. Always close exactly the fds we obtained.
    // No unbounded allocation, threads, fork storm, or machine-wide exhaustion.
    int duplicates[PROBE_FD_LIMIT + 1];
    size_t count = 0;
    int failure = 0;
    while (count < PROBE_FD_LIMIT + 1) {
        int fd = fcntl(input, F_DUPFD_CLOEXEC, 0);
        if (fd < 0) { failure = errno; break; }
        duplicates[count++] = fd;
    }
    for (size_t i = 0; i < count; i++) close(duplicates[i]);
    bool denied = count > 0 && failure == EMFILE
        && exact_limit(RLIMIT_NOFILE, PROBE_FD_LIMIT);
    int recovered = fcntl(input, F_DUPFD_CLOEXEC, 0);
    bool recovery = recovered >= 0;
    if (recovered >= 0) close(recovered);
    xpc_dictionary_set_bool(result, "descriptorCeilingDenied", denied);
    xpc_dictionary_set_bool(result, "descriptorRecovery", recovery);
    return result;
}

#endif
