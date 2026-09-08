#include "worker_output.h"
#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <poll.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

void ort_output_close(struct ort_output_reader *reader) {
    for (unsigned i = 0; i < 2; i++) {
        int fd = reader->descriptors[i];
        reader->descriptors[i] = -1;
        // Do not retry close after EINTR: a reused fd must never be closed.
        if (fd >= 0) (void)close(fd);
    }
}

static bool prepare_pipe(int fd) {
    struct stat info;
    int flags = fcntl(fd, F_GETFL);
    int descriptor_flags = fcntl(fd, F_GETFD);
    return fd >= 0 && fstat(fd, &info) == 0 && S_ISFIFO(info.st_mode)
        && flags >= 0 && (flags & O_ACCMODE) == O_RDONLY
        && descriptor_flags >= 0
        && fcntl(fd, F_SETFL, flags | O_NONBLOCK) == 0
        && fcntl(fd, F_SETFD, descriptor_flags | FD_CLOEXEC) == 0;
}

bool ort_output_init(struct ort_output_reader *reader, int out, int err,
    size_t out_limit, size_t err_limit) {
    memset(reader, 0, sizeof(*reader));
    reader->descriptors[0] = out;
    reader->descriptors[1] = err == out ? -1 : err;
    reader->limits[0] = out_limit;
    reader->limits[1] = err_limit;
    if (out == err || !out_limit || !err_limit || out_limit == SIZE_MAX
        || err_limit == SIZE_MAX || !prepare_pipe(out) || !prepare_pipe(err)) {
        reader->failed = true;
        ort_output_close(reader);
        return false;
    }
    return true;
}

struct ort_output_event ort_output_receive(struct ort_output_reader *reader,
    unsigned maximum_wait_ms) {
    struct ort_output_event event = { .kind = ORT_OUTPUT_PENDING };
    if (reader->failed) { event.kind = ORT_OUTPUT_FAILED; return event; }
    if (reader->descriptors[0] < 0 && reader->descriptors[1] < 0) return event;
    struct pollfd descriptors[2] = {
        { .fd = reader->descriptors[0], .events = POLLIN },
        { .fd = reader->descriptors[1], .events = POLLIN }
    };
    unsigned wait = maximum_wait_ms < ORT_OUTPUT_POLL_CEILING_MS
        ? maximum_wait_ms : ORT_OUTPUT_POLL_CEILING_MS;
    int ready = poll(descriptors, 2, (int)wait);
    if (ready < 0 && errno == EINTR) return event;
    if (ready < 0) goto failed;
    if (!ready) return event;
    for (unsigned turn = 0; turn < 2; turn++) {
        unsigned stream = (reader->next_stream + turn) % 2;
        short flags = descriptors[stream].revents;
        if (!flags) continue;
        if (flags & POLLNVAL) goto failed;
        if (!(flags & (POLLIN | POLLHUP | POLLERR))) goto failed;
        reader->next_stream = (stream + 1) % 2;
        size_t remaining = reader->limits[stream] - reader->totals[stream];
        size_t capacity = remaining < ORT_OUTPUT_CHUNK_CAPACITY
            ? remaining + 1 : ORT_OUTPUT_CHUNK_CAPACITY;
        ssize_t count = read(reader->descriptors[stream], event.bytes, capacity);
        if (count < 0 && (errno == EAGAIN || errno == EWOULDBLOCK || errno == EINTR)) return event;
        if (count < 0 || (size_t)count > remaining) goto failed;
        if (!count) {
            int fd = reader->descriptors[stream];
            reader->descriptors[stream] = -1;
            (void)close(fd);
            reader->eof[stream] = true;
            event.kind = stream ? ORT_OUTPUT_STDERR_EOF : ORT_OUTPUT_STDOUT_EOF;
            return event;
        }
        event.length = (size_t)count;
        reader->totals[stream] += event.length;
        event.kind = stream ? ORT_OUTPUT_STDERR : ORT_OUTPUT_STDOUT;
        if (stream) memset(event.bytes, 0, sizeof(event.bytes));
        return event;
    }
    return event;
failed:
    reader->failed = true;
    ort_output_close(reader);
    memset(&event, 0, sizeof(event));
    event.kind = ORT_OUTPUT_FAILED;
    return event;
}
