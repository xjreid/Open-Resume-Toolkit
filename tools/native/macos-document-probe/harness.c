// Standalone signed test bundle: no desktop/profile/vault integration.
#include "probe.h"
#include <dispatch/dispatch.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>

static xpc_object_t control = NULL;
static xpc_object_t measured = NULL;
static xpc_object_t hardened = NULL;
static xpc_object_t hard_limits = NULL;
static const char *boolean_keys[] = {"descriptorRead", "descriptorReadOnly", "sandboxEntitlement"};
static const char *outcome_keys[] = {"siblingRead", "siblingWrite", "symlinkRead", "loopbackConnect", "childCreation", "childFork"};
static const char *limit_keys[] = {"nprocSoft", "nprocHard", "nofileSoft", "nofileHard", "coreSoft", "coreHard"};
static const char *limit_boolean_keys[] = {"raiseDenied", "descriptorCeilingDenied", "descriptorRecovery"};
static const int parent_resources[] = {RLIMIT_NPROC, RLIMIT_NOFILE, RLIMIT_CORE};
static struct rlimit parent_limits[3];

static void fail(void) {
    fputs("Synthetic sandbox probe failed; no containment evidence was accepted.\n", stderr);
    exit(1);
}

static void fixture_path(char *output, size_t capacity, const char *root, const char *name) {
    int written = snprintf(output, capacity, "%s/%s", root, name);
    if (written < 0 || (size_t)written >= capacity) fail();
}

static bool valid_result(xpc_object_t result) {
    if (!result || xpc_get_type(result) != XPC_TYPE_DICTIONARY || xpc_dictionary_get_count(result) != 9) return false;
    for (size_t i = 0; i < 3; i++) {
        xpc_object_t value = xpc_dictionary_get_value(result, boolean_keys[i]);
        if (!value || xpc_get_type(value) != XPC_TYPE_BOOL) return false;
    }
    for (size_t i = 0; i < 6; i++) {
        xpc_object_t value = xpc_dictionary_get_value(result, outcome_keys[i]);
        if (!value || xpc_get_type(value) != XPC_TYPE_INT64) return false;
        int64_t number = xpc_int64_get_value(value);
        if (number < PROBE_ALLOWED || number > PROBE_ERROR) return false;
    }
    return true;
}

static bool valid_limits(xpc_object_t result) {
    if (!result || xpc_get_type(result) != XPC_TYPE_DICTIONARY || xpc_dictionary_get_count(result) != 9) return false;
    for (size_t i = 0; i < 6; i++) {
        xpc_object_t value = xpc_dictionary_get_value(result, limit_keys[i]);
        if (!value || xpc_get_type(value) != XPC_TYPE_UINT64) return false;
        if (xpc_uint64_get_value(value) > PROBE_FD_LIMIT) return false;
    }
    for (size_t i = 0; i < 3; i++) {
        xpc_object_t value = xpc_dictionary_get_value(result, limit_boolean_keys[i]);
        if (!value || xpc_get_type(value) != XPC_TYPE_BOOL) return false;
    }
    return true;
}

static void print_limits(xpc_object_t result) {
    putchar('{');
    for (size_t i = 0; i < 6; i++) {
        printf("%s\"%s\":%llu", i ? "," : "", limit_keys[i],
            (unsigned long long)xpc_dictionary_get_uint64(result, limit_keys[i]));
    }
    for (size_t i = 0; i < 3; i++) {
        printf(",\"%s\":%s", limit_boolean_keys[i],
            xpc_dictionary_get_bool(result, limit_boolean_keys[i]) ? "true" : "false");
    }
    putchar('}');
}

static void print_result(xpc_object_t result) {
    putchar('{');
    for (size_t i = 0; i < 3; i++) {
        printf("%s\"%s\":%s", i ? "," : "", boolean_keys[i],
            xpc_dictionary_get_bool(result, boolean_keys[i]) ? "true" : "false");
    }
    for (size_t i = 0; i < 6; i++) {
        printf(",\"%s\":%lld", outcome_keys[i], (long long)xpc_dictionary_get_int64(result, outcome_keys[i]));
    }
    putchar('}');
}

int main(int argc, char **argv) {
    // The runner creates exactly this private synthetic fixture layout. No
    // arbitrary document/profile target or externally supplied network address.
    if (argc != 2 || getuid() == 0 || geteuid() == 0) fail();
    for (size_t i = 0; i < 3; i++) {
        if (getrlimit(parent_resources[i], &parent_limits[i]) != 0) fail();
    }
    char root[PATH_MAX];
    struct stat metadata;
    const char prefix[] = "/private/tmp/ort-document-sandbox-";
    if (!realpath(argv[1], root) || strcmp(root, argv[1]) != 0
        || strncmp(root, prefix, sizeof(prefix) - 1) != 0
        || strchr(root + sizeof(prefix) - 1, '/') != NULL
        || lstat(root, &metadata) != 0 || !S_ISDIR(metadata.st_mode)
        || metadata.st_uid != getuid() || (metadata.st_mode & 0777) != 0700) fail();
    char input_path[PATH_MAX], sibling[PATH_MAX], alias[PATH_MAX];
    fixture_path(input_path, sizeof(input_path), root, "input.txt");
    fixture_path(sibling, sizeof(sibling), root, "sibling.txt");
    fixture_path(alias, sizeof(alias), root, "sibling-link");
    int input = open(input_path, O_RDONLY | O_CLOEXEC | O_NOFOLLOW);
    if (input < 0) fail();
    int listener = socket(AF_INET, SOCK_STREAM, 0);
    if (listener < 0) fail();
    struct sockaddr_in address = {0};
    address.sin_len = sizeof(address);
    address.sin_family = AF_INET;
    address.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
    socklen_t length = sizeof(address);
    if (bind(listener, (struct sockaddr *)&address, sizeof(address)) != 0
        || listen(listener, 8) != 0 || getsockname(listener, (struct sockaddr *)&address, &length) != 0) fail();
    uint16_t port = ntohs(address.sin_port);
    control = run_probes(input, sibling, alias, port, false);
    if (!valid_result(control)) fail();
    fputs("sandbox-probe: positive-control measurements collected\n", stderr);

    // Main serial queue orders reply, interruption and timeout processing.
    xpc_connection_t connection = xpc_connection_create(PROBE_SERVICE, dispatch_get_main_queue());
    if (!connection) fail();
    xpc_connection_set_event_handler(connection, ^(xpc_object_t event) {
        if (event == XPC_ERROR_CONNECTION_INTERRUPTED) {
            if (!measured || !hardened || !hard_limits) fail();
            bool parent_unchanged = true;
            for (size_t i = 0; i < 3; i++) {
                struct rlimit current;
                if (getrlimit(parent_resources[i], &current) != 0
                    || current.rlim_cur != parent_limits[i].rlim_cur
                    || current.rlim_max != parent_limits[i].rlim_max) parent_unchanged = false;
            }
            parent_unchanged = parent_unchanged && probe_child(false) == PROBE_ALLOWED;
            fputs("sandbox-probe: cooperative helper disconnect observed\n", stderr);
            printf("{\"schemaVersion\":2,\"control\":");
            print_result(control);
            printf(",\"sandboxed\":");
            print_result(measured);
            printf(",\"hardened\":");
            print_result(hardened);
            printf(",\"hardLimits\":");
            print_limits(hard_limits);
            printf(",\"parentUnaffected\":%s,\"cooperativeDisconnectObserved\":true}\n",
                parent_unchanged ? "true" : "false");
            close(input);
            close(listener);
            exit(0);
        }
        if (xpc_get_type(event) == XPC_TYPE_ERROR) fail();
    });
    xpc_connection_resume(connection);
    xpc_object_t request = xpc_dictionary_create(NULL, NULL, 0);
    xpc_dictionary_set_fd(request, "input", input);
    xpc_dictionary_set_string(request, "sibling", sibling);
    xpc_dictionary_set_string(request, "alias", alias);
    xpc_dictionary_set_uint64(request, "port", port);
    xpc_connection_send_message_with_reply(connection, request, dispatch_get_main_queue(), ^(xpc_object_t reply) {
        if (xpc_get_type(reply) != XPC_TYPE_DICTIONARY || measured) fail();
        xpc_object_t result = xpc_dictionary_get_value(reply, "result");
        xpc_object_t limited = xpc_dictionary_get_value(reply, "hardened");
        xpc_object_t limits = xpc_dictionary_get_value(reply, "hardLimits");
        if (!valid_result(result) || !valid_result(limited) || !valid_limits(limits)) fail();
        measured = xpc_retain(result);
        hardened = xpc_retain(limited);
        hard_limits = xpc_retain(limits);
        fputs("sandbox-probe: helper measurements collected\n", stderr);
    });
    xpc_release(request);
    dispatch_after(dispatch_time(DISPATCH_TIME_NOW, 15 * NSEC_PER_SEC), dispatch_get_main_queue(), ^{
        xpc_connection_cancel(connection);
        fail();
    });
    dispatch_main();
}
