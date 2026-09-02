// This is a cooperative, synthetic probe, NOT the hostile-document worker.
#include "probe.h"
#include "limits.h"
#include <dispatch/dispatch.h>
#include <stdlib.h>
#include <stdatomic.h>

static atomic_flag used = ATOMIC_FLAG_INIT;

static void accept_peer(xpc_connection_t peer) {
    xpc_connection_set_event_handler(peer, ^(xpc_object_t request) {
        if (xpc_get_type(request) != XPC_TYPE_DICTIONARY || atomic_flag_test_and_set(&used)) return;
        const char *sibling = xpc_dictionary_get_string(request, "sibling");
        const char *alias = xpc_dictionary_get_string(request, "alias");
        uint64_t port = xpc_dictionary_get_uint64(request, "port");
        int input = xpc_dictionary_dup_fd(request, "input");
        if (!sibling || !alias || input < 0 || port == 0 || port > UINT16_MAX
            || strlen(sibling) > 1024 || strlen(alias) > 1024) _exit(65);
        // First retain the plain App Sandbox baseline. No parser or untrusted
        // code is present in either phase of this synthetic experiment.
        xpc_object_t result = run_probes(input, sibling, alias, (uint16_t)port, false);
        if (!install_limits()) _exit(65);
        xpc_object_t hardened = run_probes(input, sibling, alias, (uint16_t)port, true);
        xpc_object_t limits = probe_limits(input);
        close(input);
        xpc_object_t reply = xpc_dictionary_create_reply(request);
        if (!reply) _exit(65);
        xpc_dictionary_set_value(reply, "result", result);
        xpc_dictionary_set_value(reply, "hardened", hardened);
        xpc_dictionary_set_value(reply, "hardLimits", limits);
        xpc_connection_send_message(peer, reply);
        xpc_release(result);
        xpc_release(hardened);
        xpc_release(limits);
        xpc_release(reply);
        // Let the reply drain, then terminate this specific probe process.
        // Parent observes connection interruption; it never kills an XPC PID.
        dispatch_after(dispatch_time(DISPATCH_TIME_NOW, NSEC_PER_SEC),
            dispatch_get_global_queue(QOS_CLASS_DEFAULT, 0), ^{ _exit(0); });
    });
    xpc_connection_resume(peer);
}

int main(void) {
    if (getuid() == 0 || geteuid() == 0) _exit(65);
    // Cooperative failsafe only, not a production CPU/wall-time boundary.
    alarm(10);
    xpc_main(accept_peer);
}
