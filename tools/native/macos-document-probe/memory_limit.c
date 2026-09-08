// Isolated resource-limit measurement only. No parser, file input or sandbox claim.
#include <errno.h>
#include <inttypes.h>
#include <mach/mach.h>
#include <stdbool.h>
#include <stdio.h>
#include <sys/mman.h>
#include <sys/resource.h>
#include <unistd.h>

#define MEMORY_LIMIT (512ULL * 1024ULL * 1024ULL)
int main(void) {
    if (getuid() == 0 || geteuid() == 0) return 65;
    alarm(5);
    const size_t excessive = (size_t)MEMORY_LIMIT + 16384;
    void *control = mmap(NULL, excessive, PROT_NONE, MAP_PRIVATE | MAP_ANON, -1, 0);
    if (control == MAP_FAILED || munmap(control, excessive) != 0) return 65;
    struct rlimit before, after, limit = { MEMORY_LIMIT, MEMORY_LIMIT };
    if (getrlimit(RLIMIT_AS, &before) != 0 || before.rlim_cur < MEMORY_LIMIT || before.rlim_max < MEMORY_LIMIT) return 65;
    mach_task_basic_info_data_t info = {0};
    mach_msg_type_number_t count = MACH_TASK_BASIC_INFO_COUNT;
    if (task_info(mach_task_self(), MACH_TASK_BASIC_INFO, (task_info_t)&info, &count) != KERN_SUCCESS) return 65;
    bool established = setrlimit(RLIMIT_AS, &limit) == 0;
    int establishment_errno = established ? 0 : errno;
    bool exact = established && getrlimit(RLIMIT_AS, &after) == 0
        && after.rlim_cur == MEMORY_LIMIT && after.rlim_max == MEMORY_LIMIT;
    bool denied = false, raise_denied = false, small_mapping = false;
    if (exact) {
        void *attempt = mmap(NULL, excessive, PROT_NONE, MAP_PRIVATE | MAP_ANON, -1, 0);
        denied = attempt == MAP_FAILED && errno == ENOMEM;
        if (attempt != MAP_FAILED) (void)munmap(attempt, excessive);
        struct rlimit raised = { MEMORY_LIMIT + 16384, MEMORY_LIMIT + 16384 };
        raise_denied = setrlimit(RLIMIT_AS, &raised) == -1 && errno == EPERM;
        const size_t small = 1024 * 1024;
        volatile unsigned char *memory = mmap(NULL, small, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANON, -1, 0);
        if (memory != MAP_FAILED) {
            memory[0] = 1; memory[small - 1] = 2;
            small_mapping = memory[0] == 1 && memory[small - 1] == 2;
            (void)munmap((void *)memory, small);
        }
    }
    // SDK-declared alternative, applied only to this isolated helper's task.
    // Success alone would not establish fatal enforcement or raise denial.
    int old_footprint = 0;
    kern_return_t footprint_result = task_set_phys_footprint_limit(mach_task_self(), 512, &old_footprint);
    printf("{\"controlMapped\":true,\"virtualBytesBefore\":%" PRIu64 ",\"residentBytesBefore\":%" PRIu64 ",\"establishmentErrno\":%d,\"limitEstablished\":%s,\"exactLimit\":%s,\"oversizedMappingDenied\":%s,\"raiseDenied\":%s,\"smallMappingWorks\":%s,\"footprintSetterResult\":%d,\"fullContainmentProven\":false,\"importEnabled\":false}\n",
        (uint64_t)info.virtual_size, (uint64_t)info.resident_size, establishment_errno,
        established ? "true" : "false", exact ? "true" : "false", denied ? "true" : "false", raise_denied ? "true" : "false", small_mapping ? "true" : "false", footprint_result);
    return 0;
}
