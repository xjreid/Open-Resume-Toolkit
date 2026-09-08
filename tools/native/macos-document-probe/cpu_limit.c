// Isolated synthetic CPU enforcement probe. No parser or sandbox claim.
#include <errno.h>
#include <signal.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <sys/resource.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

static double now(void) {
    struct timespec time;
    if (clock_gettime(CLOCK_MONOTONIC, &time) != 0) return -1;
    return (double)time.tv_sec + (double)time.tv_nsec / 1e9;
}

static bool wall_kill = false;
static int observed_signal = 0;
static double ignored_signal_cpu_seconds = 0;
static bool run(unsigned mode) {
    int handshake[2];
    if (pipe(handshake) != 0) return false;
    pid_t child = fork();
    if (child < 0) { close(handshake[0]); close(handshake[1]); return false; }
    if (child == 0) {
        close(handshake[0]);
        struct rlimit limit = {1, 1}, after, raised = {2, 2};
        struct rlimit cores = {0, 0};
        // A hostile parser can ignore the soft-limit signal. The hard CPU
        // limit must still stop it, without the supervisor's wall-time kill.
        if (signal(SIGXCPU, mode == 2 ? SIG_IGN : SIG_DFL) == SIG_ERR || setrlimit(RLIMIT_CORE, &cores) != 0
            || setrlimit(RLIMIT_CPU, &limit) != 0 || getrlimit(RLIMIT_CPU, &after) != 0
            || after.rlim_cur != 1 || after.rlim_max != 1
            || setrlimit(RLIMIT_CPU, &raised) != -1 || errno != EPERM) _exit(65);
        if (write(handshake[1], "R", 1) != 1) _exit(65);
        close(handshake[1]);
        if (!mode) _exit(0);
        volatile uint64_t counter = 0;
        for (;;) counter++;
    }
    close(handshake[1]);
    double start = now();
    bool timed_out = start < 0;
    bool child_owned = true;
    int status = 0;
    struct rusage usage = {0};
    pid_t result = 0;
    while (!timed_out) {
        result = wait4(child, &status, WNOHANG, &usage);
        if (result == child) break;
        if (result < 0 && errno != EINTR) { child_owned = false; break; }
        double current = now();
        if (current < 0 || current - start > 8) { timed_out = true; break; }
        struct timespec delay = {.tv_nsec = 10000000};
        (void)nanosleep(&delay, NULL);
    }
    if (result != child) {
        // Only our unreaped direct child is eligible for this cleanup signal.
        // ECHILD means no owned child remains; never signal a potentially reused PID.
        if (child_owned) {
            if (mode == 2) wall_kill = true;
            (void)kill(child, SIGKILL);
            do { result = wait4(child, &status, 0, &usage); } while (result < 0 && errno == EINTR);
            if (mode == 2 && result == child) ignored_signal_cpu_seconds =
                (double)usage.ru_utime.tv_sec + (double)usage.ru_utime.tv_usec / 1e6
                + (double)usage.ru_stime.tv_sec + (double)usage.ru_stime.tv_usec / 1e6;
        }
        close(handshake[0]);
        return false;
    }
    char ready[2] = {0};
    ssize_t count = read(handshake[0], ready, sizeof(ready));
    close(handshake[0]);
    if (timed_out || count != 1 || ready[0] != 'R') return false;
    if (mode == 2 && WIFSIGNALED(status)) observed_signal = WTERMSIG(status);
    if (mode == 2) ignored_signal_cpu_seconds =
        (double)usage.ru_utime.tv_sec + (double)usage.ru_utime.tv_usec / 1e6
        + (double)usage.ru_stime.tv_sec + (double)usage.ru_stime.tv_usec / 1e6;
    return mode ? WIFSIGNALED(status) && WTERMSIG(status) == (mode == 1 ? SIGXCPU : SIGKILL)
        : WIFEXITED(status) && WEXITSTATUS(status) == 0;
}

int main(void) {
    if (getuid() == 0 || geteuid() == 0) return 65;
    bool control = run(0), signal_observed = run(1), enforced = run(2);
    printf("{\"controlExited\":%s,\"cpuSignalObserved\":%s,\"hardCpuKillObserved\":%s,\"supervisorWallKill\":%s,\"observedSignal\":%d,\"ignoredSignalCpuSeconds\":%.6f,\"cpuSeconds\":1,\"fullContainmentProven\":false,\"importEnabled\":false}\n",
        control ? "true" : "false", signal_observed ? "true" : "false", enforced ? "true" : "false", wall_kill ? "true" : "false", observed_signal, ignored_signal_cpu_seconds);
    return control && signal_observed ? 0 : 65;
}
