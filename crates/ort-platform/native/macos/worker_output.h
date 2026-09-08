#ifndef ORT_WORKER_OUTPUT_H
#define ORT_WORKER_OUTPUT_H
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

// Native adapter component only. This does not launch or establish containment.
#define ORT_OUTPUT_CHUNK_CAPACITY 8192
#define ORT_OUTPUT_POLL_CEILING_MS 25

enum ort_output_kind { ORT_OUTPUT_PENDING, ORT_OUTPUT_STDOUT, ORT_OUTPUT_STDERR,
    ORT_OUTPUT_STDOUT_EOF, ORT_OUTPUT_STDERR_EOF, ORT_OUTPUT_FAILED };
struct ort_output_event {
    enum ort_output_kind kind;
    size_t length;
    unsigned char bytes[ORT_OUTPUT_CHUNK_CAPACITY];
};
struct ort_output_reader {
    int descriptors[2];
    size_t totals[2];
    size_t limits[2];
    bool eof[2];
    bool failed;
    unsigned next_stream;
};

// Takes ownership of both distinct descriptors, including on initialization
// failure. The trusted caller supplies the central supervisor byte limits.
// Input descriptors must be the private read ends of two ordinary pipes.
bool ort_output_init(struct ort_output_reader *reader, int stdout_fd,
    int stderr_fd, size_t stdout_limit, size_t stderr_limit);
// At most one bounded read per call. No retry loop under EINTR or readiness
// races. maximum_wait_ms is clamped to the supervisor's 25 ms poll ceiling.
// A returned event never contains stderr content. Its length counts discarded
// bytes so the Rust policy can independently enforce its own stderr ceiling.
struct ort_output_event ort_output_receive(struct ort_output_reader *reader,
    unsigned maximum_wait_ms);
// Idempotent; closes only currently owned descriptors. No PID or signal API.
void ort_output_close(struct ort_output_reader *reader);
#endif
