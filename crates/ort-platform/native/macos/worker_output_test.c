// Synthetic real-pipe checks. No worker, parser, user file, vault or network.
#include "worker_output.h"
#include <assert.h>
#include <fcntl.h>
#include <pthread.h>
#include <signal.h>
#include <stdio.h>
#include <string.h>
#include <time.h>
#include <unistd.h>

static void pipes(int out[2], int err[2]) { assert(pipe(out) == 0); assert(pipe(err) == 0); }
static void write_exact(int fd, const char *text) { size_t count = strlen(text); assert(write(fd, text, count) == (ssize_t)count); }
static double now(void) { struct timespec t; assert(clock_gettime(CLOCK_MONOTONIC, &t) == 0); return (double)t.tv_sec + (double)t.tv_nsec / 1e9; }

static volatile sig_atomic_t interrupted = 0;
static void signal_handler(int signal_number) { (void)signal_number; interrupted = 1; }
static void *interrupt_poll(void *target) {
    struct timespec delay = { .tv_nsec = 5000000 };
    (void)nanosleep(&delay, NULL);
    assert(pthread_kill(*(pthread_t *)target, SIGUSR1) == 0);
    return NULL;
}

int main(void) {
    int out[2], err[2]; struct ort_output_reader reader;
    pipes(out, err); assert(ort_output_init(&reader, out[0], err[0], 32, 16));
    assert(fcntl(out[0], F_GETFL) & O_NONBLOCK);
    assert(fcntl(out[0], F_GETFD) & FD_CLOEXEC);
    double start = now();
    assert(ort_output_receive(&reader, 1000).kind == ORT_OUTPUT_PENDING);
    // Scheduling tolerance is not a real-time OS guarantee. The actual poll
    // timeout is clamped to 25 ms; this catches accidental unbounded waits.
    assert(now() - start < 0.25);
    write_exact(out[1], "partial"); write_exact(err[1], "PRIVATE_STDERR");
    struct ort_output_event event = ort_output_receive(&reader, 0);
    assert(event.kind == ORT_OUTPUT_STDOUT && event.length == 7);
    assert(memcmp(event.bytes, "partial", 7) == 0);
    write_exact(out[1], "more");
    event = ort_output_receive(&reader, 0);
    assert(event.kind == ORT_OUTPUT_STDERR && event.length == 14);
    for (size_t i = 0; i < sizeof(event.bytes); i++) assert(event.bytes[i] == 0);
    assert(ort_output_receive(&reader, 0).kind == ORT_OUTPUT_STDOUT);
    close(out[1]); close(err[1]);
    assert(ort_output_receive(&reader, 0).kind == ORT_OUTPUT_STDERR_EOF);
    assert(ort_output_receive(&reader, 0).kind == ORT_OUTPUT_STDOUT_EOF);
    assert(ort_output_receive(&reader, 0).kind == ORT_OUTPUT_PENDING);
    assert(reader.eof[0] && reader.eof[1]); ort_output_close(&reader);

    // Exact-limit bytes are allowed, one additional byte fails sticky and
    // closes both owned readers without exposing the offending bytes.
    for (unsigned stream = 0; stream < 2; stream++) {
        pipes(out, err); assert(ort_output_init(&reader, out[0], err[0], 4, 4));
        int writer = stream ? err[1] : out[1]; write_exact(writer, "1234");
        event = ort_output_receive(&reader, 0); assert(event.length == 4);
        write_exact(writer, "5"); event = ort_output_receive(&reader, 0);
        assert(event.kind == ORT_OUTPUT_FAILED && event.length == 0);
        for (size_t i = 0; i < sizeof(event.bytes); i++) assert(event.bytes[i] == 0);
        assert(reader.descriptors[0] == -1 && reader.descriptors[1] == -1);
        assert(ort_output_receive(&reader, 0).kind == ORT_OUTPUT_FAILED);
        close(out[1]); close(err[1]); ort_output_close(&reader);
    }
    pipes(out, err);
    assert(!ort_output_init(&reader, out[1], err[0], 4, 4));
    close(out[0]); close(err[1]); // Invalid write end was consumed.
    pipes(out, err);
    assert(!ort_output_init(&reader, out[0], err[0], 0, 4));
    close(out[1]); close(err[1]);
    pipes(out, err);
    assert(!ort_output_init(&reader, out[0], out[0], 4, 4));
    close(out[1]); close(err[0]); close(err[1]);
    // Buffered bytes precede EOF even when both writers already closed.
    pipes(out, err); assert(ort_output_init(&reader, out[0], err[0], 32, 16));
    write_exact(out[1], "buffered"); close(out[1]); close(err[1]);
    event = ort_output_receive(&reader, 0);
    assert(event.kind == ORT_OUTPUT_STDOUT && event.length == 8);
    assert(ort_output_receive(&reader, 0).kind == ORT_OUTPUT_STDERR_EOF);
    assert(ort_output_receive(&reader, 0).kind == ORT_OUTPUT_STDOUT_EOF);
    ort_output_close(&reader);

    // Interrupt a silent poll. The driver returns control instead of restarting
    // a full wait, allowing the owning supervisor to check cancellation/deadline.
    pipes(out, err); assert(ort_output_init(&reader, out[0], err[0], 32, 16));
    struct sigaction action = {0}, previous = {0};
    action.sa_handler = signal_handler; assert(sigemptyset(&action.sa_mask) == 0);
    assert(sigaction(SIGUSR1, &action, &previous) == 0);
    pthread_t self = pthread_self(), sender;
    assert(pthread_create(&sender, NULL, interrupt_poll, &self) == 0);
    event = ort_output_receive(&reader, 25);
    assert(event.kind == ORT_OUTPUT_PENDING);
    assert(pthread_join(sender, NULL) == 0); assert(interrupted);
    assert(!reader.failed);
    assert(sigaction(SIGUSR1, &previous, NULL) == 0);
    close(out[1]); close(err[1]); ort_output_close(&reader);

    // A queued burst cannot force a read/event larger than the fixed chunk.
    pipes(out, err); assert(ort_output_init(&reader, out[0], err[0], 32768, 16));
    assert(fcntl(out[1], F_SETFL, O_NONBLOCK) == 0);
    unsigned char burst[16384]; memset(burst, 'x', sizeof(burst));
    ssize_t queued = write(out[1], burst, sizeof(burst)); assert(queued > 0);
    size_t received = 0;
    while (received < (size_t)queued) {
        event = ort_output_receive(&reader, 0);
        assert(event.kind == ORT_OUTPUT_STDOUT && event.length > 0);
        assert(event.length <= ORT_OUTPUT_CHUNK_CAPACITY);
        received += event.length;
    }
    assert(received == (size_t)queued);
    close(out[1]); close(err[1]); ort_output_close(&reader);

    puts("PASS: silent wait, private pipe flags, partial reads, fair stderr, redaction, EOF, exact limits, sticky failure, buffered EOF, signal interruption and ownership. No containment claim.");
}
